use ethers::{
    providers::{HttpClientError, JsonRpcError, ProviderError, RpcError},
    types::SignatureError,
};
use hex::FromHexError;
use log::error;
use thiserror::Error;
use walletconnect_client::prelude::WalletConnectError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Missing RPC provider")]
    MissingProvider,

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    WalletConnectError(#[from] WalletConnectError),

    #[error(transparent)]
    HttpClientError(#[from] HttpClientError),

    #[error(transparent)]
    SignatureError(#[from] SignatureError),

    #[error(transparent)]
    HexError(#[from] FromHexError),

    #[error("Communication error")]
    CommsError,
}

impl RpcError for Error {
    fn as_error_response(&self) -> Option<&JsonRpcError> {
        match self {
            Error::WalletConnectError(e) => e.as_error_response(),
            Error::HttpClientError(e) => e.as_error_response(),
            _ => None,
        }
    }

    fn is_error_response(&self) -> bool {
        self.as_error_response().is_some()
    }

    fn as_serde_error(&self) -> Option<&serde_json::Error> {
        match self {
            Error::WalletConnectError(e) => e.as_serde_error(),
            Error::HttpClientError(e) => e.as_serde_error(),
            Error::SerdeJsonError(e) => Some(e),
            _ => None,
        }
    }

    fn is_serde_error(&self) -> bool {
        self.as_serde_error().is_some()
    }
}

impl From<Error> for ProviderError {
    fn from(src: Error) -> Self {
        ProviderError::JsonRpcClientError(Box::new(src))
    }
}
