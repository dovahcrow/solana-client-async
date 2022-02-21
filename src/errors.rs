use thiserror::Error;

#[derive(Error, Debug)]
pub enum SolanaClientError {
    #[error("Background process exited")]
    BackgroundProcessExited,

    #[error("Responder closed")]
    ResponderClosed,

    #[error("RPC Error: {code}: {message}")]
    RpcError { code: i64, message: String },

    #[error(transparent)]
    Websocket(#[from] tungstenite::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Subscription(#[from] tokio::sync::broadcast::error::RecvError),
}
