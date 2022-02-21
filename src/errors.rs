use thiserror::Error;

#[derive(Error, Debug)]
pub enum SolanaClientError {
    #[error("Background process exited")]
    BackgroundProcessExited,

    #[error("Responder closed")]
    ResponderClosed,

    #[error("RPC Error: {code}: {message}")]
    RpcError { code: i64, message: String },

    #[error("Websocket error: {0}")]
    Websocket(#[from] tungstenite::Error),

    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Subscription error: {0}")]
    Subscription(#[from] tokio::sync::broadcast::error::RecvError),
}
