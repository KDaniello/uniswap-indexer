use alloy::{
    primitives::{Address, U256}, 
    providers::{Provider, ProviderBuilder, WsConnect}, 
    rpc::types::Filter, 
    sol, 
    sol_types::SolEvent
};
use bigdecimal::{BigDecimal, ToPrimitive};
use num_traits::{One, Zero};
use eyre::Result;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use std::env;
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, warn, info};
use clickhouse::{Client, Row};
use serde::Serialize;

sol! {
    event Swap(
        address indexed sender,
        address indexed recipient,
        int256 amount0,
        int256 amount1,
        uint160 sqrtPriceX96,
        uint128 liquidity,
        int24 tick,
    );

    // Interface: get token from Pull
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function token0() external view returns (address);
        function token1() external view returns (address);
    }

    // Interface: get decimals
    #[sol(rpc)]
    interface IERC20 {
        function decimals() external view returns (uint8);
    }
}

#[derive(Debug, Serialize, Row)]
struct SwapRecord {
    timestamp: i64,
    tx_hash: String,
    pool_address: String,
    sender: String,
    recipient: String,
    price_usd: f64,
    liquidity: String,
    decimals_shift: i32
}

const Q96_STR: &str = "79228162514264337593543950336";

fn calculate_price(sqrt_price_x96: U256, decimal_diff: i32) -> BigDecimal {
    let price_bd = BigDecimal::from_str(&sqrt_price_x96.to_string()).unwrap_or_default();
    let q96_bd = BigDecimal::from_str(Q96_STR).unwrap();

    let sqrt_price = &price_bd / &q96_bd;
    let price_raw = &sqrt_price * &sqrt_price;

    // Shift correction
    let shift_val = 10u128.pow(decimal_diff.abs() as u32);
    let shift = BigDecimal::from(shift_val);

    let adjusted_price = if decimal_diff > 0 {
        price_raw  * shift
    } else {
        price_raw / shift
    };

    if adjusted_price.is_zero() {
        return BigDecimal::zero();
    }

    let one = BigDecimal::one();
    one / adjusted_price
}

// func: get decimals
async fn fetch_pool_decimals(http_url: &str, pool_addr: Address) -> Result<i32> {
    let provider = ProviderBuilder::new().connect_http(http_url.parse()?);

    let pool_contract = IUniswapV3Pool::new(pool_addr, provider.clone());

    let t0_raw = pool_contract.token0().call().await?.0;
    let t1_raw = pool_contract.token1().call().await?.0;

    let t0_addr = Address::from(t0_raw);
    let t1_addr = Address::from(t1_raw);

    info!("🔍 Token0: {:?}, Token1: {:?}", t0_addr, t1_addr);

    let t0_contract = IERC20::new(t0_addr, provider.clone());
    let t1_contract = IERC20::new(t1_addr, provider.clone());

    let d0 = t0_contract.decimals().call().await?;
    let d1 = t1_contract.decimals().call().await?;

    info!("📊 Decimals: T0={}, T1={}", d0, d1);

    let diff = (d0 as i32) - (d1 as i32);
    Ok(diff)
}

// ClickHouse
fn get_clickhouse_client() -> Client {
    Client::default()
        .with_url("http://localhost:8123")
        .with_user("default")
        .with_password("password123")
        .with_database("crypto_db")
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    dotenv::dotenv().ok();

    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
    let rpc_http_url = env::var("RPC_HTTP_URL").expect("RPC_HTTP_URL (HTTP) must be set");
    let pool_str = env::var("POOL_ADDRESS").unwrap_or_else(|_| "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640".to_string());
    let pool_address = Address::from_str(&pool_str).expect("Invalid pool address");
    
    info!("🦄 Uniswap Indexer v0.2 Started");
    info!("🎯 Pool: {:?}", pool_address);

    info!("⏳ Fetching token decimals...");
    let decimal_diff = fetch_pool_decimals(&rpc_http_url, pool_address).await?;
    info!("✅ Decimal Shift Calculated: {}", decimal_diff);

    let (tx, mut rx) = mpsc::channel::<SwapRecord>(10000);

    tokio::spawn(async move {
        let client = get_clickhouse_client();
        let mut batch = Vec::with_capacity(100); // buffer for batch to send to DB

        while let Some(record) = rx.recv().await {
            batch.push(record);

            if batch.len() >= 10 {

        match client.insert::<SwapRecord>("uniswap_swaps").await {
            Ok(mut insert) => {
                for r in &batch {
                    if let Err(e) = insert.write(r).await {
                        error!("❌ Write error: {:?}", e);
                    }
                }

                match insert.end().await {
                    Ok(_) => info!("💾 Saved {} swaps to ClickHouse", batch.len()),
                    Err(e) => error!("❌ ClickHouse End Error: {:?}", e),
                }
            }
            Err(e) => error!("❌ Failed to create inserter: {:?}", e),
        }
        batch.clear();
    }
        }
    });

    loop {
        info!("Connecting to WebSocket...");
        match run_indexer(&rpc_url, pool_address, decimal_diff, tx.clone()).await {
            Ok(_) => warn!("⚠️ Connection closed. Reconnecting..."),
            Err(e) => error!("❌ WS Error: {:?}. Reconnecting...", e),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run_indexer(rpc_url: &str, target: Address, decimal_diff: i32, tx: mpsc::Sender<SwapRecord>) -> Result<()> {
    
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    info!("✅ Connected! Waiting for Swaps...\n");

    let filter = Filter::new()
        .address(target)
        .event_signature(Swap::SIGNATURE_HASH);

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        if let Ok(decoded) = log.log_decode::<Swap>() {
            let data = decoded.inner.data;
            let tx_hash = log.transaction_hash.unwrap_or_default();

            let price_bd = calculate_price(U256::from(data.sqrtPriceX96), decimal_diff);

            let price_f64 = price_bd.to_f64().unwrap_or(0.0);
            let now = chrono::Utc::now();

            let record = SwapRecord {
                timestamp: now.timestamp_millis(),
                tx_hash: tx_hash.to_string(),
                pool_address: target.to_string(),
                sender: data.sender.to_string(),
                recipient: data.recipient.to_string(),
                price_usd: price_f64,
                liquidity: data.liquidity.to_string(),
                decimals_shift: decimal_diff
            };

            if let Err(e) = tx.send(record).await {
                error!("❌ Channel closed, receiver died: {:?}", e);
                break;
            }

            info!("🔄 Swap detected: ${:.2}", price_f64);   
        }
    }

    Ok(())
}