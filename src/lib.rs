pub mod background;
pub mod client;
pub mod errors;
pub mod rpc_message;

pub mod prelude {
    pub use crate::background::BackgroundProcess;
    pub use crate::client::{Client, ClientBuilder};
    pub use crate::errors::SolanaClientError;
}

use crate::rpc_message::{RpcError, RpcResponse};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Responder = oneshot::Sender<Result<RpcResponse, RpcError>>;
