pub mod background;
pub mod client;
pub mod errors;
pub mod rpc_message;

pub mod prelude {
    pub use crate::background::BackgroundProcess;
    pub use crate::client::{Client, ClientBuilder};
    pub use crate::errors::SolanaClientError;
}

use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
