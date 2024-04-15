use ethers::{
    prelude::{JsonRpcError, ProviderError, RpcError, SignatureError},
    utils::ConversionError,
};
use gloo_utils::format::JsValueSerdeExt;
use hex::FromHexError;
use thiserror::Error;
use wasm_bindgen::JsValue;

#[derive(Error, Debug)]
/// Error thrown when sending an HTTP request
pub enum Eip1193Error {
    /// Thrown if the request failed
    #[error("JsValue error")]
    JsValueError(String),

    /// Thrown if no window.ethereum is found in DOM
    #[error("No ethereum found")]
    JsNoEthereum,

    #[error("Cannot parse ethereum response")]
    JsParseError,

    #[error("Not implemented yet")]
    Unimplemented,

    #[error(transparent)]
    /// Thrown if the response could not be parsed
    JsonRpcError(#[from] JsonRpcError),

    #[error(transparent)]
    /// Serde JSON Error
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    ConversionError(#[from] ConversionError),

    #[error(transparent)]
    SignatureError(#[from] SignatureError),

    #[error(transparent)]
    HexError(#[from] FromHexError),

    #[error("Communication error")]
    CommunicationError,
}

impl RpcError for Eip1193Error {
    fn as_error_response(&self) -> Option<&JsonRpcError> {
        match self {
            Eip1193Error::JsonRpcError(e) => Some(e),
            _ => None,
        }
    }

    fn is_error_response(&self) -> bool {
        self.as_error_response().is_some()
    }

    fn as_serde_error(&self) -> Option<&serde_json::Error> {
        match self {
            Eip1193Error::SerdeJson(e) => Some(e),
            _ => None,
        }
    }

    fn is_serde_error(&self) -> bool {
        self.as_serde_error().is_some()
    }
}

impl From<JsValue> for Eip1193Error {
    fn from(src: JsValue) -> Self {
        if let Ok(message) = src.into_serde::<JsonRpcError>() {
            Eip1193Error::JsonRpcError(message)
        } else {
            Eip1193Error::JsValueError(format!("{:?}", src))
        }
    }
}

impl From<Eip1193Error> for ProviderError {
    fn from(src: Eip1193Error) -> Self {
        ProviderError::JsonRpcClientError(Box::new(src))
    }
}
