# Solana Websocket Client With Async

This library is in its early stage, not all the subscription method are implemented.

However, you can still enjoy the async/await support and auto keep-alive feature provided by this library,
with a little additional effort on using `Client::request` and `Client::recv_raw` to do the request
contructing and message parsing.

# Motivation

The original solana websocket client is good but lacks some important features, including
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

    client.slot_subscribe().await.unwrap().await.unwrap(); // Double await because the first await is for `Send` and the second one for `Receive`. It is fine to drop the second one.

    loop {
        let slot = client.recv::<SlotInfo>().await.unwrap();
        println!("slot {:?}", slot);
    }
}

```

