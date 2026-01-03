# 🦄 Uniswap V3 Real-Time Indexer (Rust + ClickHouse)

A high-performance, asynchronous indexing engine designed to ingest Uniswap V3 swap events from the Ethereum blockchain in real-time. 

Built with **Rust** for low latency and **ClickHouse** for storage. The system handles WebSocket instability, performs precise on-chain math (Q96 decoding), and implements non-blocking data ingestion.

## 🚀 Key Features

- **⚡ Zero-Blocking Architecture:** Uses `tokio::sync::mpsc` channels to decouple blockchain listening (Producer) from database writes (Consumer).
- **🛡️ Fault Tolerance:** Implements a self-healing connection loop. Automatically reconnects to RPC nodes upon WebSocket disconnects or timeouts.
- **🧮 Precision Math:** Manually decodes `sqrtPriceX96` to human-readable prices using `BigDecimal`, ensuring no precision loss for financial data.
- **🔧 Dynamic Metadata:** Automatically fetches token decimals via HTTP RPC on startup to adjust price calculations for any Pool (USDC/ETH, WBTC/USDC, etc.).
- **💾 Batch Ingestion:** Buffers events in memory and writes to ClickHouse in batches to optimize I/O and network throughput.

## 🛠️ Tech Stack

- **Language:** Rust
- **Blockchain Lib:** Alloy
- **Async Runtime:** Tokio
- **Database:** ClickHouse
- **Containerization:** Docker & Docker Compose

## 🚀 How to Run

### 1. Prerequisites
Docker
Rust

### 2. Setup
Clone the repo.

```bash
git clone https://github.com/YOUR_USERNAME/uniswap-indexer.git
cd uniswap-indexer
```

### 3. Create .env

```text
# WebSocket
RPC_URL=wss://eth.llamarpc.com

# HTTP for fetching static data 
RPC_HTTP_URL=https://eth.llamarpc.com

# Target Uniswap V3 Pool Address (e.g., USDC/ETH)
POOL_ADDRESS=0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640
```

### 4. Start ClickHouse-server
docker-compose up -d

### 5. Run
cargo run --release

## 📸 Sample Output

```text
2026-01-02T16:36:15.078965Z  INFO uniswap_indexer: 🦄 Uniswap Indexer v0.2 Started
2026-01-02T16:36:15.079272Z  INFO uniswap_indexer: 📄 Output: uniswap_swaps.csv
2026-01-02T16:36:15.079418Z  INFO uniswap_indexer: 🎯 Pool: 0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640
2026-01-02T16:36:15.079599Z  INFO uniswap_indexer: Connecting to WebSocket...
2026-01-02T16:36:15.614713Z  INFO uniswap_indexer: ✅ Connected! Waiting for Swaps...
2026-01-02T16:36:25.864337Z  INFO uniswap_indexer: [2026-01-02 19:36:25] 🔄 Price: $3133.65
```

## Database Schema
```SQL
CREATE TABLE crypto_db.uniswap_swaps (
    timestamp DateTime64(3),
    tx_hash String,
    pool_address String,
    sender String,
    price_usd Float64,
    liquidity String,
    decimals_shift Int32
) 
ENGINE = MergeTree()
ORDER BY (pool_address, timestamp);
```

## 📜 License
MIT License.