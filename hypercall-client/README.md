# hypercall-client

Rust API client for Hypercall.

## Responsibilities

- HTTP and websocket clients for Hypercall API surfaces.
- EIP-712 wallet helpers for public options, RFQ, and quote-provider signing.
- Request and response convenience types re-exported from `hypercall-sdk-types`.

## Getting Started

Add the client from the public Git repository:

```toml
[dependencies]
hypercall-client = { git = "https://github.com/hypercall-public/hypercall-rust" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Create a production API client:

```rust,no_run
use hypercall_client::HypercallClient;

#[tokio::main]
async fn main() -> hypercall_client::Result<()> {
    let client = HypercallClient::new("https://api.hypercall.xyz");
    let instruments = client.get_instrument_specs("BTC").await?;

    println!("BTC instruments: {}", instruments.len());
    Ok(())
}
```

## Wallet Signing

`HypercallWallet` supports local private-key signing in every build:

```rust,no_run
use hypercall_client::HypercallWallet;

fn main() -> hypercall_client::Result<()> {
    let wallet = HypercallWallet::from_private_key("0xYOUR_PRIVATE_KEY", 999)?;

    println!("wallet: {}", wallet.address());
    Ok(())
}
```

Enable the `kms` feature to use AWS KMS-backed signing through the AWS SDK
default credential chain:

```bash
cargo check -p hypercall-client --features kms
```

Private key export only works for local private-key wallets. KMS wallets never
expose private key material.

## Funding

Funding is not a Hypercall API write. Use `get_exchange_info()` to fetch the
production Exchange contract address, chain ID, and signing domain. For the
HyperEVM USDC route, approve USDC to the Exchange contract and call
`depositUsdcFor(account, amount)` with a normal EVM wallet transaction. Do not
send USDC directly to `exchange_address`.

```rust,no_run
use hypercall_client::HypercallClient;

#[tokio::main]
async fn main() -> hypercall_client::Result<()> {
    let client = HypercallClient::new("https://api.hypercall.xyz");
    let exchange = client.get_exchange_info().await?;

    println!("exchange contract: {}", exchange.exchange_address);
    println!("chain id: {}", exchange.chain_id);
    Ok(())
}
```

The on-chain call shape is:

```solidity
function depositUsdcFor(address account, uint256 amount) external;
```

`amount` is USDC token units. For example, `100_000_000` is 100 USDC for a
6-decimal USDC token. Integrators that do not want to manage EVM approval and
transaction submission should fund through the Hypercall app.

Example with Foundry `cast`:

```bash
export HYPEREVM_RPC_URL="https://..."
export PRIVATE_KEY="0x..."
export USDC_ADDRESS="0x..."
export EXCHANGE_ADDRESS="$(curl -s https://api.hypercall.xyz/exchange-info | jq -r .exchange_address)"
export HYPERCALL_ACCOUNT="0x..."
export AMOUNT_UNITS="100000000" # 100 USDC with 6 decimals

cast send \
  --rpc-url "$HYPEREVM_RPC_URL" \
  --private-key "$PRIVATE_KEY" \
  "$USDC_ADDRESS" \
  "approve(address,uint256)" \
  "$EXCHANGE_ADDRESS" \
  "$AMOUNT_UNITS"

cast send \
  --rpc-url "$HYPEREVM_RPC_URL" \
  --private-key "$PRIVATE_KEY" \
  "$EXCHANGE_ADDRESS" \
  "depositUsdcFor(address,uint256)" \
  "$HYPERCALL_ACCOUNT" \
  "$AMOUNT_UNITS"
```

## Development

Run the focused client checks before changing this crate:

```bash
cargo test -p hypercall-client
cargo test -p hypercall-client --no-default-features
cargo check -p hypercall-client --features kms
```

## Feature Flags

| feature | used by | why |
| --- | --- | --- |
| default | client users | Enables the Alloy rustls backend. |
| alloy-rustls | live integrations | Enables Alloy's reqwest rustls backend. |
| alloy-native-tls | native TLS users | Enables Alloy's reqwest native TLS backend. |
| kms | AWS/client KMS users | Enables Alloy AWS signer support. |
