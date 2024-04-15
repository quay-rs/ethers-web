//! A simple dApp client library for Eip1193 and WalletConnect wallet connections with ethers
//! library.
#![doc = include_str!("../README.md")]

pub mod explorer;

mod eip1193;
mod event;

#[cfg(feature = "leptos")]
pub mod leptos;

mod walletconnect;
#[cfg(feature = "yew")]
pub mod yew;

use async_trait::async_trait;
use eip1193::{error::Eip1193Error, Eip1193};
use ethers::{
    providers::{JsonRpcClient, JsonRpcError, ProviderError, RpcError},
    types::{Address, Signature, SignatureError, U256},
    utils::ConversionError,
};
use gloo_storage::{LocalStorage, Storage};
use gloo_utils::format::JsValueSerdeExt;
use hex::FromHexError;
use log::{debug, error};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use walletconnect_client::{prelude::Metadata, WalletConnect, WalletConnectState};
use wasm_bindgen_futures::spawn_local;

const STATUS_KEY: &str = "ETHERS_WEB_STATE";

use crate::event::WalletEvent;
use walletconnect::WalletConnectProvider;

/// Ethereum builder for Ethereum object
pub struct EthereumBuilder {
    pub chain_id: u64,
    pub name: String,
    pub description: String,
    pub url: String,
    pub wc_project_id: Option<String>,
    pub icons: Vec<String>,
    pub rpc_node: Option<String>,
}

impl Default for EthereumBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EthereumBuilder {
    /// Simple builder constructor
    pub fn new() -> Self {
        Self {
            chain_id: 1,
            name: "Example dApp".to_string(),
            description: "An example dApp written in Rust".to_string(),
            url: "https://github.com/quay-rs/ethers-web".to_string(),
            wc_project_id: None,
            icons: Vec::new(),
            rpc_node: None,
        }
    }

    /// Setting defaults chain id
    pub fn chain_id(&mut self, chain_id: u64) -> &Self {
        self.chain_id = chain_id;
        self
    }

    /// Setting dApp name
    pub fn name(&mut self, name: &str) -> &Self {
        self.name = name.to_string();
        self
    }

    /// Setting dApp description
    pub fn description(&mut self, description: &str) -> &Self {
        self.description = description.to_string();
        self
    }

    /// Setting dApp url
    pub fn url(&mut self, url: &str) -> &Self {
        self.url = url.to_string();
        self
    }

    /// Setting WalletConnects ProjectId
    pub fn walletconnect_id(&mut self, wc_project_id: &str) -> &Self {
        self.wc_project_id = Some(wc_project_id.to_string());
        self
    }

    /// Setting RPC node working together with WalletConnect connection for non-signer interactions
    pub fn rpc_node(&mut self, rpc_node: &str) -> &Self {
        self.rpc_node = Some(rpc_node.to_string());
        self
    }

    /// Setting dApp icon url
    pub fn add_icon(&mut self, icon_url: &str) -> &Self {
        self.icons.push(icon_url.to_string());
        self
    }

    /// Building final Ethereum object
    pub fn build(&self) -> Ethereum {
        Ethereum::new(
            self.chain_id,
            self.name.clone(),
            self.description.clone(),
            self.url.clone(),
            self.wc_project_id.clone(),
            self.icons.clone(),
            self.rpc_node.clone(),
        )
    }
}

/// Available wallet types
#[derive(Clone, Debug, Copy)]
pub enum WalletType {
    Injected,
    WalletConnect,
}

/// Error struct
#[derive(Error, Debug)]
pub enum EthereumError {
    #[error("Wallet unavaibale")]
    Unavailable,

    #[error("Not connected")]
    NotConnected,

    #[error("Already connected")]
    AlreadyConnected,

    #[error(transparent)]
    ConversionError(#[from] ConversionError),

    #[error(transparent)]
    ProviderError(#[from] ProviderError),

    #[error(transparent)]
    SignatureError(#[from] SignatureError),

    #[error(transparent)]
    HexError(#[from] FromHexError),

    #[error(transparent)]
    Eip1193Error(#[from] Eip1193Error),

    #[error(transparent)]
    WalletConnectError(#[from] crate::walletconnect::error::Error),

    #[error(transparent)]
    WalletConnectClientError(#[from] walletconnect_client::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

impl From<EthereumError> for ProviderError {
    fn from(src: EthereumError) -> Self {
        ProviderError::JsonRpcClientError(Box::new(src))
    }
}

impl RpcError for EthereumError {
    fn as_error_response(&self) -> Option<&JsonRpcError> {
        match self {
            EthereumError::Eip1193Error(e) => e.as_error_response(),
            EthereumError::WalletConnectError(e) => e.as_error_response(),
            EthereumError::WalletConnectClientError(e) => e.as_error_response(),
            _ => None,
        }
    }

    fn is_error_response(&self) -> bool {
        self.as_error_response().is_some()
    }

    fn as_serde_error(&self) -> Option<&serde_json::Error> {
        match self {
            EthereumError::Eip1193Error(e) => e.as_serde_error(),
            // EthereumError::ProviderError(e) => e.as_serde_error(),
            EthereumError::WalletConnectError(e) => e.as_serde_error(),
            EthereumError::WalletConnectClientError(e) => e.as_serde_error(),
            _ => None,
        }
    }

    fn is_serde_error(&self) -> bool {
        self.as_error_response().is_some()
    }
}

/// Currently connected provider
#[derive(Clone)]
pub(crate) enum WebProvider {
    None,
    Injected(Eip1193),
    WalletConnect(WalletConnectProvider),
}

impl WebProvider {
    fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
}

impl PartialEq for WebProvider {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::None, Self::None)
                | (Self::Injected(_), Self::Injected(_))
                | (Self::WalletConnect(_), Self::WalletConnect(_))
        )
    }
}

/// Ethereum's state to store or to be restored from
#[derive(Clone, Serialize, Deserialize)]
pub struct EthereumState {
    pub chain_id: Option<u64>,
    pub wc_state: Option<WalletConnectState>,
}

/// Ethereum's connection event
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ConnectionWaiting(String),
    Connected,
    Disconnected,
    Broken,
    ChainIdChanged(Option<u64>),
    AccountsChanged(Option<Vec<Address>>),
}

impl Event {
    fn is_connection_established(&self) -> bool {
        !matches!(
            self,
            Self::ConnectionWaiting(_)
                | Self::Disconnected
                | Self::ChainIdChanged(None)
                | Self::AccountsChanged(None)
        )
    }
}

impl From<walletconnect_client::event::Event> for Event {
    fn from(value: walletconnect_client::event::Event) -> Self {
        match value {
            walletconnect_client::event::Event::Disconnected => Self::Disconnected,
            walletconnect_client::event::Event::Connected => Self::Connected,
            walletconnect_client::event::Event::AccountsChanged(acc) => Self::AccountsChanged(acc),
            walletconnect_client::event::Event::ChainIdChanged(id) => {
                Self::ChainIdChanged(Some(id))
            }
            walletconnect_client::event::Event::Broken => Self::Broken,
        }
    }
}

/// Main wallet connectivity object to maintain connections
#[derive(Clone)]
pub struct Ethereum {
    pub metadata: Metadata,
    pub wc_project_id: Option<String>,
    pub rpc_node: Option<String>,

    accounts: Option<Vec<Address>>,
    chain_id: Option<u64>,

    sender: Sender<Event>,
    receiver: Arc<Mutex<Receiver<Event>>>,

    wallet: WebProvider,
}

impl PartialEq for Ethereum {
    fn eq(&self, other: &Self) -> bool {
        self.metadata == other.metadata
            && self.wc_project_id == other.wc_project_id
            && self.rpc_node == other.rpc_node
            && self.accounts == other.accounts
            && self.chain_id == other.chain_id
            && self.wallet == other.wallet
    }
}

impl Debug for Ethereum {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "Ethereum with accounts: {:?}, chain_id: {:?} ", self.accounts, self.chain_id)
    }
}

impl Ethereum {
    /// Ethereum constructor
    fn new(
        chain_id: u64,
        name: String,
        description: String,
        url: String,
        wc_project_id: Option<String>,
        icons: Vec<String>,
        rpc_node: Option<String>,
    ) -> Self {
        let (sender, receiver) = channel::<Event>(10);

        Ethereum {
            metadata: Metadata::from(&name, &description, &url, icons),
            wc_project_id,
            rpc_node,
            accounts: None,
            chain_id: Some(chain_id),
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            wallet: WebProvider::None,
        }
    }

    /// Checks if given wallet type is currently available to connect
    pub fn is_available(&self, wallet_type: WalletType) -> bool {
        match wallet_type {
            WalletType::Injected => self.injected_available(),
            WalletType::WalletConnect => self.walletconnect_available(),
        }
    }

    /// Checks if we have a provider connection
    pub fn has_provider(&self) -> bool {
        self.wallet.is_some()
    }

    /// Checks what type of the wallet is currently connected
    pub fn connected_wallet_type(&self) -> Option<WalletType> {
        match &self.wallet {
            WebProvider::None => None,
            WebProvider::Injected(_) => Some(WalletType::Injected),
            WebProvider::WalletConnect(_) => Some(WalletType::WalletConnect),
        }
    }

    /// Returns available wallets types
    pub fn available_wallets(&self) -> Vec<WalletType> {
        let mut types = Vec::new();

        if Eip1193::is_available() {
            types.push(WalletType::Injected);
        }

        if self.wc_project_id.is_some() {
            types.push(WalletType::WalletConnect);
        }

        types
    }

    /// Checkes if injected wallet is available in current context
    pub fn injected_available(&self) -> bool {
        Eip1193::is_available()
    }

    /// Checks if WalletConnect connection is available in current context (configuration)
    pub fn walletconnect_available(&self) -> bool {
        self.wc_project_id.is_some()
    }

    /// Fetching available wallets from WalletConnect explorer
    pub async fn fetch_available_wallets(
        &self,
    ) -> Result<Vec<explorer::WalletDescription>, EthereumError> {
        match &self.wc_project_id {
            None => Err(EthereumError::Unavailable),
            Some(p_id) => {
                let client = reqwest::Client::new();
                let resp: explorer::ExplorerResponse = client
                    .get(format!(
                        "https://explorer-api.walletconnect.com/v3/wallets?projectId={p_id}",
                    ))
                    .send()
                    .await?
                    .json()
                    .await?;
                Ok(resp.parse_wallets(p_id))
            }
        }
    }

    /// Performing connection to selected wallet
    pub async fn connect(&mut self, wallet: WalletType) -> Result<(), EthereumError> {
        if self.wallet != WebProvider::None {
            return Err(EthereumError::AlreadyConnected);
        }

        match wallet {
            WalletType::Injected => self.connect_injected().await,
            WalletType::WalletConnect => self.connect_wc(None).await,
        }
    }

    /// Disconnects from wallet
    pub async fn disconnect(&mut self) {
        if let WebProvider::WalletConnect(wc) = &self.wallet {
            wc.disconnect().await;
        }

        self.wallet = WebProvider::None;
        self.accounts = None;

        _ = self.sender.send(Event::Disconnected).await;
    }

    async fn connect_injected(&mut self) -> Result<(), EthereumError> {
        if !self.injected_available() {
            return Err(EthereumError::Unavailable);
        }

        let injected = Eip1193::new();

        {
            let s = self.sender.clone();
            _ = injected.clone().on(
                WalletEvent::Disconnect,
                Box::new(move |_| {
                    let sender = s.clone();
                    spawn_local(async move {
                        _ = sender.send(Event::Disconnected).await;
                    })
                }),
            );
        }
        {
            let s = self.sender.clone();
            _ = injected.clone().on(
                WalletEvent::ChainChanged,
                Box::new(move |chain_id| {
                    let sender = s.clone();
                    spawn_local(async move {
                        _ = sender
                            .send(Event::ChainIdChanged(
                                chain_id.into_serde::<U256>().ok().map(|c| c.low_u64()),
                            ))
                            .await;
                    });
                }),
            );
        }
        {
            let s = self.sender.clone();
            _ = injected.clone().on(
                WalletEvent::AccountsChanged,
                Box::new(move |accounts| {
                    let sender = s.clone();
                    spawn_local(async move {
                        let accounts = accounts.into_serde::<Vec<Address>>().ok();
                        _ = sender.send(Event::AccountsChanged(accounts.clone())).await;
                        match &accounts {
                            Some(acc) => {
                                _ = sender
                                    .send(if acc.is_empty() {
                                        Event::Disconnected
                                    } else {
                                        Event::Connected
                                    })
                                    .await;
                            }
                            None => _ = sender.send(Event::Disconnected).await,
                        }
                    });
                }),
            );
        }
        self.wallet = WebProvider::Injected(injected);
        self.accounts = Some(self.request_accounts().await?);
        self.chain_id = Some(self.request_chain_id().await?.low_u64());

        _ = self.sender.send(Event::Connected).await;
        if self.chain_id.is_some() {
            _ = self.sender.send(Event::ChainIdChanged(self.chain_id)).await;
        }
        if self.accounts.is_some() {
            _ = self.sender.send(Event::AccountsChanged(self.accounts.clone())).await;
        }

        Ok(())
    }

    /// Getting next available event from the event queue
    pub async fn next(&self) -> Result<Option<Event>, EthereumError> {
        let event = match &self.wallet {
            WebProvider::WalletConnect(provider) => {
                let mut recvr = self.receiver.lock().await;
                tokio::select! {
                    e = recvr.recv() => Ok(e),
                    e = provider.next() => Ok(e?.map(|e| e.into()))
                }
            }
            _ => Ok(self.receiver.lock().await.recv().await),
        };

        debug!("NEW EVENT {:?}", event);
        if let Ok(Some(e)) = &event {
            if e == &Event::Connected {
                if let WebProvider::WalletConnect(provider) = &self.wallet {
                    _ = self.sender.send(Event::ChainIdChanged(Some(provider.chain_id()))).await;
                    _ = self.sender.send(Event::AccountsChanged(provider.accounts())).await;
                }
            }

            if !e.is_connection_established() {
                LocalStorage::delete(STATUS_KEY);
            } else {
                _ = LocalStorage::set(STATUS_KEY, self.collect_state());
            }
        }

        event
    }

    /// Signs typed data using connected wallet
    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, EthereumError> {
        match &self.wallet {
            WebProvider::None => Err(EthereumError::NotConnected),
            WebProvider::Injected(provider) => Ok(provider.sign_typed_data(data, from).await?),
            WebProvider::WalletConnect(provider) => {
                Ok(provider.sign_typed_data(data, from).await?)
            }
        }
    }

    /// Performs network switch to other chain id
    pub async fn switch_network(&mut self, chain_id: u64) -> Result<(), EthereumError> {
        match self.wallet {
            WebProvider::WalletConnect(ref mut provider) => {
                // We need to check if we've got any accounts under that id
                if let Some(accounts) = provider.accounts_for_chain(chain_id) {
                    if !accounts.is_empty() {
                        self.accounts = Some(accounts.clone());
                        self.chain_id = Some(chain_id);
                        provider.set_chain_id(chain_id);
                        _ = self.sender.send(Event::ChainIdChanged(Some(chain_id))).await;
                        _ = self.sender.send(Event::AccountsChanged(Some(accounts))).await;
                        return Ok(());
                    }
                }
                Err(EthereumError::Unavailable)
            }
            _ => Err(EthereumError::Unavailable),
        }
    }

    async fn connect_wc(&mut self, state: Option<WalletConnectState>) -> Result<(), EthereumError> {
        if !self.walletconnect_available() {
            return Err(EthereumError::Unavailable);
        }

        let wc = WalletConnect::connect(
            self.wc_project_id.clone().unwrap().into(),
            self.chain_id.unwrap_or(1),
            self.metadata.clone(),
            state.clone(),
        )?;

        let url = wc
            .initiate_session(
                state
                    .as_ref()
                    .map(|s| s.keys.clone().into_iter().map(|(t, _)| t).collect::<Vec<_>>()),
            )
            .await?;

        self.wallet =
            WebProvider::WalletConnect(WalletConnectProvider::new(wc, self.rpc_node.clone()));

        if !url.is_empty() {
            _ = self.sender.send(Event::ConnectionWaiting(url)).await;
        } else {
            _ = self.sender.send(Event::Connected).await;
            _ = self.sender.send(Event::ChainIdChanged(self.chain_id)).await;
            _ = self.sender.send(Event::AccountsChanged(self.accounts.clone())).await;
        }

        Ok(())
    }

    async fn request_accounts(&self) -> Result<Vec<Address>, EthereumError> {
        match &self.wallet {
            WebProvider::None => Err(EthereumError::NotConnected),
            WebProvider::Injected(_) => Ok(self.request("eth_requestAccounts", ()).await?),
            WebProvider::WalletConnect(wc) => match wc.accounts() {
                Some(a) => Ok(a),
                None => Err(EthereumError::Unavailable),
            },
        }
    }

    async fn request_chain_id(&self) -> Result<U256, EthereumError> {
        match &self.wallet {
            WebProvider::None => Err(EthereumError::NotConnected),
            WebProvider::Injected(_) => Ok(self.request("eth_chainId", ()).await?),
            WebProvider::WalletConnect(wc) => Ok(wc.chain_id().into()),
        }
    }

    /// Restores connection state from local storage
    pub async fn restore(&mut self) -> bool {
        match LocalStorage::get::<EthereumState>(STATUS_KEY) {
            Ok(state) => {
                match state.wc_state {
                    None => _ = self.connect_injected().await,
                    Some(wc_settings) => _ = self.connect_wc(Some(wc_settings)).await,
                }
                true
            }
            Err(err) => {
                error!("Status not loaded {err:?}!");
                false
            }
        }
    }

    fn collect_state(&self) -> EthereumState {
        match &self.wallet {
            WebProvider::WalletConnect(p) => {
                EthereumState { chain_id: Some(p.chain_id()), wc_state: Some(p.get_state()) }
            }
            _ => EthereumState { chain_id: self.chain_id, wc_state: None },
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(? Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcClient for Ethereum {
    type Error = EthereumError;

    async fn request<T: Serialize + Send + Sync + std::fmt::Debug, R: DeserializeOwned + Send>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, Self::Error> {
        match &self.wallet {
            WebProvider::None => Err(EthereumError::NotConnected),
            WebProvider::Injected(provider) => Ok(provider.request(method, params).await?),
            WebProvider::WalletConnect(provider) => Ok(provider.request(method, params).await?),
        }
    }
}
