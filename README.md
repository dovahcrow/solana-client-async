# Solana Websocket Client With Async ![CI](https://github.com/dovahcrow/solana-client-async/actions/workflows/ci.yml/badge.svg)

# Motivation

The official solana websocket client is good but lacks important features like
async/await and heartbeat to keep the stream alive.

This implementation of the client fixes these problem. 

# Usage

## Subscribe to slot

```rust
use solana_client::rpc_response::SlotInfo;
use solana_client_async::prelude::*;

#[tokio::main]
async fn main() {
    let mut client = ClientBuilder::new()
        .ws_url("wss://api.mainnet-beta.solana.com")
        .build()
        .await
        .unwrap();

    let _subscription_id: usize = client.slot_subscribe().await.unwrap().await.unwrap(); // Double await because the first await is for `Send` and the second one for `Receive`. It is fine to drop the second one.

    loop {
        let slot = client.recv::<SlotInfo>().await.unwrap();
        println!("slot {:?}", slot);
    }
}
```

Take a look at the [examples](/examples) or [tests](/tests) for more examples.
