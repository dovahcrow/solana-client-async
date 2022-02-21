use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Clone, Debug, Deserialize)]
pub struct RpcNotificationParams<T = Box<RawValue>> {
    pub result: T,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RpcNotification<T = Box<RawValue>> {
    pub jsonrpc: String,
    pub method: String,
    pub params: RpcNotificationParams<T>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RpcRequest<T = Box<RawValue>> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: T,
}

impl<T> RpcRequest<T>
where
    T: Serialize,
{
    pub fn new(id: u64, method: &str, params: T) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RpcResponse<T = Box<RawValue>> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: T,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RpcError {
    pub jsonrpc: String,
    pub id: u64,
    pub error: RpcErrorBody,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RpcErrorBody {
    pub code: i64,
    pub message: String,
}
