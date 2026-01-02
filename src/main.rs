use alloy::{
    primitives::{Address, I256, U256, address}, 
    providers::{Provider, ProviderBuilder, WsConnect}, 
    rpc::types::Filter, 
    sol, 
    sol_types::SolEvent
};
use bigdecimal::{BigDecimal,  FromPrimitive};
use num_traits::{One, Zero};
use eyre::Result;
use futures_util::StreamExt;
use std::env;
use std::str::FromStr;
use std::fs::OpenOptions;
use chrono::Local;

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
}

#[derive(serde::Serialize)]
struct SwapRecord {
    timestamp: String,
    tx_hash: String,
    price_usd: String,
    liquidity: String,
    sender: String,
    recipient: String,
}

const POOL_ADDRESS: Address = address!("88e6a0c2ddd26feeb64f039a2c41296fcb3f5640");
const Q96_STR: &str = "79228162514264337593543950336";

fn calculate_price_in_usdc(sqrt_price_x96: U256) -> BigDecimal {
    let price_bd = BigDecimal::from_str(&sqrt_price_x96.to_string()).unwrap_or_default();

    let q96_bd = BigDecimal::from_str(Q96_STR).unwrap();

    let sqrt_price = &price_bd / &q96_bd;
    
    let price_raw = &sqrt_price * &sqrt_price;

    // Корректировка на 12 знаков (ETH - 18 знаков, USDC - 6 знаков)
    let shift = BigDecimal::from(10u64.pow(12));

    let price_token1_per_token0 = &price_raw / &shift;

    if price_token1_per_token0.is_zero() {
        return BigDecimal::from(0);
    }

    let one = BigDecimal::one();

    one / price_token1_per_token0
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
    let csv_path = env::var("OUTPUT_FILE").unwrap_or_else(|_| "swaps.csv".to_string());

    println!("🦄 Starting Uniswap V3 Indexer...");
    println!("📄 Output: {}", csv_path);
    println!("🎯 Target Pool: USDC/WETH");
    println!("📡 Connecting to: {}", rpc_url);

    let file_exists = std::path::Path::new(&csv_path).exists();
    let file = OpenOptions::new().create(true).append(true).open(&csv_path)?;
    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(!file_exists)
        .from_writer(file);
    
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    println!("✅ Connected! Waiting for Swaps...\n");

    let filter = Filter::new()
        .address(POOL_ADDRESS)
        .event_signature(Swap::SIGNATURE_HASH);

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        if let Ok(decoded) = log.log_decode::<Swap>() {
            let data = decoded.inner.data;
            let tx_hash = log.transaction_hash.unwrap_or_default();

            let eth_price = calculate_price_in_usdc(U256::from(data.sqrtPriceX96));

            let price_display = eth_price.with_scale(2);

            let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

            let record = SwapRecord {
                timestamp: now.clone(),
                tx_hash: tx_hash.to_string(),
                price_usd: eth_price.to_string(),
                liquidity: data.liquidity.to_string(),
                sender: data.sender.to_string(),
                recipient: data.recipient.to_string(),
            };

            if let Err(e) = csv_writer.serialize(&record) {
                eprintln!("❌ CSV Error: {:?}", e);
            }
            csv_writer.flush()?;

            println!(
                "[{}] 🔄 Price: ${} | Tx: {:?}",
                now, price_display, tx_hash
            );   
        }
    }

    Ok(())
}