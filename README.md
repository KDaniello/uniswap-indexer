# 🦄 Uniswap V3 High-Precision Indexer

A professional-grade blockchain indexer written in **Rust** that monitors Uniswap V3 pools in real-time with financial accuracy.

It connects to Ethereum nodes, decodes raw `sqrtPriceX96` data using **Arbitrary-Precision Arithmetic** (`BigDecimal`), and streams structured data to CSV for analysis.

## 🚀 Key Features

### Zero-Loss Financial Math
Floating-point numbers (`f64`) are dangerous for DeFi. This project implements Uniswap's Q64.96 math using `bigdecimal` to ensure 100% accuracy in price calculations.

### Production Resilience
- **Auto-Reconnect Strategy:** The indexer includes a self-healing loop that automatically recovers from WebSocket disconnects without data loss or manual intervention.
- **Structured Logging:** Uses `tracing` for clear, timestamped logs (INFO/WARN/ERROR).

### Performance
- **Async & Non-Blocking:** Built on `tokio` to handle thousands of events per second.
- **Direct RPC:** Uses `alloy-rs` (no bloated web3 wrappers) for minimal latency.

## 🛠️ Tech Stack

- **Core:** Rust, Tokio
- **Blockchain:** Alloy
- **Math:** `bigdecimal`, `num-bigint`
- **Data:** `csv`, `serde`
- **Observability:** `tracing`
  
## 🧮 The Math

Uniswap V3 stores prices as `sqrtPriceX96`. To get the human-readable price (e.g., $3,000.00), the indexer performs:

1.  **Decode** the raw `uint160` value from binary logs.
2.  **Calculate Raw Price:** $P = (sqrtPriceX96 / 2^{96})^2$
3.  **Decimal Adjustment:** $P_{adj} = P / 10^{12}$ (for USDC/WETH pool)
4.  **Inversion:** Convert *ETH per USDC* to *USDC per ETH*.

## 🚀 How to Run

### 1. Setup
Clone the repo.

```bash
git clone https://github.com/YOUR_USERNAME/uniswap-indexer.git
cd uniswap-indexer
```

### 2. Create .env

```text
# Ethereum Node WebSocket URL (Infura, Alchemy, QuickNode)
RPC_URL=wss://mainnet.infura.io/ws/v3/YOUR_API_KEY

# Output CSV file path
OUTPUT_FILE=swaps.csv

# Pool address
POOL_ADDRESS=88e6a0c2ddd26feeb64f039a2c41296fcb3f5640
```

### 4. Launch
cargo run --release

## 📸 Sample Output

```text
2026-01-02T16:36:15.078965Z  INFO uniswap_indexer: 🦄 Uniswap Indexer v0.2 Started
2026-01-02T16:36:15.079272Z  INFO uniswap_indexer: 📄 Output: uniswap_swaps.csv
2026-01-02T16:36:15.079418Z  INFO uniswap_indexer: 🎯 Pool: 0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640
2026-01-02T16:36:15.079599Z  INFO uniswap_indexer: Connecting to WebSocket...
2026-01-02T16:36:15.614713Z  INFO uniswap_indexer: ✅ Connected! Waiting for Swaps...
2026-01-02T16:36:25.864337Z  INFO uniswap_indexer: [2026-01-02 19:36:25] 🔄 Price: $3133.65 | Tx: 0x049221ada029dd5bf815c4c1e9ef1c39deacee9a612f314fe3a91dc0d4d9f904
```

CSV File
```text
timestamp,tx_hash,price_usd,liquidity,sender,recipient
2025-01-02 14:00:05,0xabc...,3150.22145...,145000000,0x123...,0x456...
```

## 📜 License
MIT License.