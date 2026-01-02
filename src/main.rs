use alloy::{
    primitives::{Address, U256}, 
    providers::{Provider, ProviderBuilder, WsConnect}, 
    rpc::types::Filter, 
    sol, 
    sol_types::SolEvent
};
use bigdecimal::{BigDecimal};
use num_traits::{One, Zero};
use eyre::Result;
use futures_util::StreamExt;
use std::env;
use std::str::FromStr;
use std::fs::OpenOptions;
use std::time::Duration;
use chrono::Local;
use tracing::{error, warn, info};

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
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    dotenv::dotenv().ok();

    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
    let csv_path = env::var("OUTPUT_FILE").unwrap_or_else(|_| "swaps.csv".to_string());

    let pool_str = env::var("POOL_ADDRESS").unwrap_or_else(|_| "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640".to_string());
    let pool_address = Address::from_str(&pool_str).expect("Invalid pool address");
    
    info!("🦄 Uniswap Indexer v0.2 Started");
    info!("📄 Output: {}", csv_path);
    info!("🎯 Pool: {:?}", pool_address);

    loop {
        info!("Connecting to WebSocket...");

        match run_indexer(&rpc_url, pool_address, &csv_path).await {
            Ok(_) => warn!("⚠️ Connection closed. Reconnecting in 5s..."),
            Err(e) => error!("❌ Error: {:?}. Reconnecting in 5s...", e),
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }

}

async fn run_indexer(rpc_url: &str, target: Address, csv_path: &str) -> Result<()> {

    let file_exists = std::path::Path::new(csv_path).exists();
    let file = OpenOptions::new().create(true).append(true).open(csv_path)?;

    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(!file_exists)
        .from_writer(file);
    
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
                error!("❌ CSV Error: {:?}", e);
            }
            csv_writer.flush()?;

            info!(
                "[{}] 🔄 Price: ${} | Tx: {:?}",
                now, price_display, tx_hash
            );   
        }
    }

    Ok(())
}