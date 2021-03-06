use thiserror::Error;

pub type Result<T> = std::result::Result<T, SolanaClientError>;

#[derive(Error, Debug)]
pub enum SolanaClientError {
    #[error("Background process exited")]
    BackgroundProcessExited,

    #[error("Responder closed")]
    ResponderClosed,

    #[error("Subscription receiver is dropped")]
    SubscriptionDropped,

    #[error("Websocket closed, reason: {0:?}")]
    WsClosed(Option<String>),

    #[error("RPC Error: {code}: {message}")]
    RpcError { code: i64, message: String },

    #[error("No host name")]
    NoHostName,

    #[error("{0}")]
    Upstream(String),

    #[error(transparent)]
    Websocket(#[from] tungstenite::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Subscription(#[from] tokio::sync::broadcast::error::RecvError),

    #[error(transparent)]
    Http(#[from] http::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),
}

impl Clone for SolanaClientError {
    fn clone(&self) -> Self {
        use SolanaClientError::*;

        match self {
            Websocket(e) => Upstream(format!("Websocket error: {:?}", e)),
            Json(e) => Upstream(format!("Json error: {:?}", e)),
            Subscription(e) => Upstream(format!("Subscription error: {:?}", e)),
            Http(e) => Upstream(format!("Http error: {:?}", e)),
            Url(e) => Upstream(format!("Url parse error: {:?}", e)),
            BackgroundProcessExited => BackgroundProcessExited,
            ResponderClosed => ResponderClosed,
            WsClosed(s) => WsClosed(s.clone()),
            RpcError { code, message } => RpcError {
                code: *code,
                message: message.clone(),
            },
            Upstream(s) => Upstream(s.clone()),
            SubscriptionDropped => SubscriptionDropped,
            NoHostName => NoHostName,
        }
    }
}
