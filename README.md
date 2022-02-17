# Cipher Examples

This repo contains a collection of example contracts that run on Cipher,
the confidential Oasis Wasm ParaTime.

## The Examples

* `vigil` - A dead-person's switch that confidentially stores secrets until a (refreshable) set time.

## How to use

1. Grab the [Oasis CLI](https://github.com/oasisprotocol/oasis-sdk/tree/main/cli).
   It can be used to upload, instantiate, and call contracts on Cipher.
2. Compile a contract using `cargo build --release --target wasm32-unknown-unknown`
3. Optionally, optimize the contract using [wasm-opt](https://github.com/WebAssembly/binaryen)
   ```
   TARGET_DIR="target/wasm32-unknown-unknown/release/"
   wasm-opt "${TARGET_DIR}/<contract>.wasm" -o "${TARGET_DIR}/<contract>.opt.wasm" -O3 -c
   ```
4. Upload the contract: `oasis contracts upload "${TARGET_DIR}/<contract>[.opt].wasm"`
5. Instantiate the contract: `oasis contracts instantiate <the-instance-id> <yaml-args>`
5. Call the contract: `oasis contract call <the-instance-id> <yaml-args>`
