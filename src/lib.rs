use std::{sync::Arc, fmt::{Debug, Formatter, Result as FmtResult}};
use eip1193::{Eip1193, Eip1193Error};
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider, ProviderError},
    types::{Address, Signature, SignatureError, U256},
    utils::ConversionError,
};
use serde::Serialize;
use gloo_utils::format::JsValueSerdeExt;
use hex::FromHexError;
use thiserror::Error;
use walletconnect_client::prelude::Metadata;

pub mod eip1193;
pub mod walletconnect;

pub struct EthereumBuilder {
    pub name: String,
    pub description: String,
    pub url: String,
    pub wc_project_id: Option<String>,
    pub icons: Vec<String>,
    pub rpc_node: String,
}

impl EthereumBuilder {
    pub fn new() -> Self {
        Self {
            name: "Example dApp".to_string(),
            description: "An example dApp written in Rust".to_string(),
            url: "https://github.com/quay-rs/ethers-web".to_string(),
            wc_project_id: None,
            icons: Vec::new(),
            rpc_node: "".to_string(),
        }
    }

    pub fn name(&mut self, name: &str) -> &Self {
        self.name = name.to_string();
        self
    }

    pub fn description(&mut self, description: &str) -> &Self {
        self.description = description.to_string();
        self
    }

    pub fn url(&mut self, url: &str) -> &Self {
        self.url = url.to_string();
        self
    }

    pub fn walletconnect_id(&mut self, wc_project_id: &str) -> &Self {
        self.wc_project_id = Some(wc_project_id.to_string());
        self
    }

    pub fn rpc_node(&mut self, rpc_node: &str) -> &Self {
        self.rpc_node = rpc_node.to_string();
        self
    }

    pub fn add_icon(&mut self, icon_url: &str) -> &Self {
        self.icons.push(icon_url.to_string());
        self
    }

    pub fn build(&self) -> Ethereum {
        Ethereum::new(
            self.name.clone(),
            self.description.clone(),
            self.url.clone(),
            self.wc_project_id.clone(),
            self.icons.clone(),
            self.rpc_node.clone(),
        )
    }
}

#[derive(Clone, Debug, Copy)]
pub enum WalletType {
    Injected,
    WalletConnect,
}

#[derive(Clone, Debug)]
pub enum ConnectedProvider {
    None,
    Injected(Provider<Eip1193>),
    WalletConnect(SignerMiddleware<Provider<Http>, walletconnect::Signer>),
}

impl PartialEq for ConnectedProvider {
    fn ne(&self, other: &Self) -> bool {
        match self {
            ConnectedProvider::None => match other {
                ConnectedProvider::None => false,
                _ => true,
            },
            ConnectedProvider::Injected(_) => match other {
                ConnectedProvider::Injected(_) => false,
                _ => true,
            },
            ConnectedProvider::WalletConnect(_) => match other {
                ConnectedProvider::WalletConnect(_) => false,
                _ => true,
            },
        }
    }

    fn eq(&self, other: &Self) -> bool {
        !self.ne(other)
    }
}

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
}

#[derive(Clone)]
pub struct Ethereum {
    pub metadata: Metadata,
    pub wc_project_id: Option<String>,
    pub rpc_node: String,

    accounts: Option<Vec<Address>>,
    chain_id: Option<u64>,
    wallet: ConnectedProvider,

    listener: Option<Arc<dyn Fn(Event)>>,
}

impl Debug for Ethereum {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Ethereum with accounts: {:?}, chain_id: {:?} and wallet: {:?}",
            self.accounts, self.chain_id, self.wallet
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Connected,
    Disconnected,
    ChainIdChanged(Option<u64>),
    AccountsChanged(Option<Vec<Address>>),
}

impl Ethereum {
    fn new(
        name: String,
        description: String,
        url: String,
        wc_project_id: Option<String>,
        icons: Vec<String>,
        rpc_node: String,
    ) -> Self {
        Ethereum {
            metadata: Metadata::from(&name, &description, &url, icons),
            wc_project_id,
            rpc_node,
            accounts: None,
            chain_id: None,
            wallet: ConnectedProvider::None,
            listener: None
        }
    }

    pub fn is_available(&self, wallet_type: WalletType) -> bool {
        match wallet_type {
            WalletType::Injected => self.injected_available(),
            WalletType::WalletConnect => self.walletconnect_available()
        }
    }

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

    pub fn injected_available(&self) -> bool {
        Eip1193::is_available()
    }

    pub fn walletconnect_available(&self) -> bool {
        self.wc_project_id.is_some()
    }

    pub async fn connect(&mut self, wallet: WalletType, listener: Option<Arc<dyn Fn(Event)>>
) -> Result<(), EthereumError> {
        if self.wallet != ConnectedProvider::None {
            return Err(EthereumError::AlreadyConnected);
        }
        self.listener = listener;
        match wallet {
            WalletType::Injected => self.connect_injected().await,
            WalletType::WalletConnect => self.connect_wc().await,
        }
    }

    pub fn disconnect(&mut self) {
        self.wallet = ConnectedProvider::None;
        self.accounts = None;
        self.chain_id = None;

        self.emit_event(Event::ChainIdChanged(None));
        self.emit_event(Event::AccountsChanged(None));
    }

    async fn connect_injected(&mut self) -> Result<(), EthereumError> {
        if !self.injected_available() {
            return Err(EthereumError::Unavailable);
        }

        let provider = Provider::<Eip1193>::new(Eip1193::new());
        self.wallet = ConnectedProvider::Injected(provider.clone());

        self.accounts = Some(self.request_accounts().await?);
        self.chain_id = Some(self.request_chain_id().await?);

        {
            let mut this = self.clone();
            _ = provider.as_ref().clone().on(
                "disconnected",
                Box::new(move |_| {
                    this.disconnect();
                    this.emit_event(Event::Disconnected);
                }),
            );
        }
        {
            let mut this = self.clone();
            _ = provider.as_ref().clone().on(
                "chainChanged",
                Box::new(move |chain_id| {
                    this.chain_id = chain_id.into_serde::<U256>().ok().map(|c| c.low_u64());
                    this.emit_event(Event::ChainIdChanged(this.chain_id));
                }),
            );
        }
        {
            let mut this = self.clone();
            _ = provider.as_ref().clone().on(
                "accountsChanged",
                Box::new(move |accounts| {
                    this.accounts = accounts.into_serde::<Vec<Address>>().ok();
                    this.emit_event(Event::AccountsChanged(this.accounts.clone()));
                }),
            );
        }
        self.emit_event(Event::Connected);
        if self.chain_id.is_some() {
            self.emit_event(Event::ChainIdChanged(self.chain_id));
        }
        if self.accounts.is_some() {
            self.emit_event(Event::AccountsChanged(self.accounts.clone()));
        }

        Ok(())
    }

    pub async fn get_provider(&self) -> ConnectedProvider {
        self.wallet.clone()
    }

    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, EthereumError> {
        match &self.wallet {
            ConnectedProvider::Injected(provider) => {
                Ok((*provider.as_ref()).sign_typed_data(data, from).await?)
            }
            ConnectedProvider::WalletConnect(_) => Err(EthereumError::Unavailable),
            ConnectedProvider::None => Err(EthereumError::NotConnected),
        }
    }

    async fn connect_wc(&mut self) -> Result<(), EthereumError> {
        if !self.walletconnect_available() {
            return Err(EthereumError::Unavailable);
        }
        Ok(())
    }

    async fn request_accounts(&self) -> Result<Vec<Address>, EthereumError> {
        match &self.wallet {
            ConnectedProvider::Injected(provider) => {
                Ok(provider.request("eth_requestAccounts", ()).await?)
            }
            ConnectedProvider::WalletConnect(_) => Err(EthereumError::Unavailable),
            ConnectedProvider::None => Err(EthereumError::NotConnected),
        }
    }

    async fn request_chain_id(&self) -> Result<u64, EthereumError> {
        match &self.wallet {
            ConnectedProvider::Injected(provider) => Ok(provider.get_chainid().await?.low_u64()),
            ConnectedProvider::WalletConnect(_) => Err(EthereumError::Unavailable),
            ConnectedProvider::None => Err(EthereumError::NotConnected),
        }
    }

    fn emit_event(&mut self, event: Event) {
        if event == Event::Disconnected {
            self.disconnect();
        }
        if let Some(listener) = &self.listener {
            listener(event);
        }
    }
}
