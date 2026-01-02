# 🦄 Uniswap V3 High-Precision Indexer

A blockchain indexer written in **Rust** that monitors the **Uniswap V3 USDC/WETH** pool in real-time.

Unlike standard wrappers, this project implements manual **mathematical decoding** of Uniswap's `sqrtPriceX96` format using arbitrary-precision arithmetic (`BigDecimal`) to ensure zero floating-point errors.

## Features

- **Real-time Event Streaming:** Connects directly to Ethereum nodes via WebSocket (WSS) using `alloy`.
- **Zero-Loss Math:** Implements `Q64.96` fixed-point conversion using `bigdecimal` to handle financial data accurately.
- **Data Persistence:** Automatically logs every swap to a structrued CSV file (`timestamp`, `tx_hash`, `price`, `liquidity`).
- **Low Latency:** Built on `tokio` async runtime for high-performance event processing.

## 🧮 The Logic (Math Deep Dive)

Uniswap V3 stores prices as `sqrtPriceX96`. To get the human-readable price, the indexer performs the following transformation:

1.  **Decode** the raw `uint160` value from the EVM log.
2.  **Calculate Raw Price:**
    $$ P = \left( \frac{sqrtPriceX96}{2^{96}} \right)^2 $$
3.  **Decimal Adjustment:** Adjust for token decimals (USDC=6, WETH=18):
    $$ P_{adj} = \frac{P}{10^{18-6}} $$
4.  **Inversion:** Since Uniswap tracks *Token1 per Token0*, we invert the result to get **USD per ETH**.

## 🛠️ Tech Stack

*   **Language:** Rust 🦀
*   **Blockchain Client:** Alloy
*   **Math:** `bigdecimal`, `num-traits`
*   **Async:** Tokio
*   **Data:** CSV, Serde

## 🚀 How to Run

### 1. Prerequisites
*   Rust & Cargo installed.
*   An Ethereum Node URL (Infura/Alchemy/QuickNode...).

### 2. Setup
Clone the repo and create a `.env` file:

```bash
git clone https://github.com/YOUR_USERNAME/uniswap-indexer.git
cd uniswap-indexer
```

### 3. Create .env
for example:

```text
RPC_URL=wss://mainnet.infura.io/ws/v3/YOUR_API_KEY
OUTPUT_FILE=swaps.csv
```

### 4. Launch
cargo run --release