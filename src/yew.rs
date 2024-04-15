use crate::{Ethereum, EthereumBuilder, EthereumError, Event, WalletType};
use ethers::{
    providers::Provider,
    types::{Address, Signature},
};
use log::error;
use serde::Serialize;
use yew::{
    function_component, html, platform::spawn_local, prelude::*, Children, ContextProvider, Html,
    Properties,
};

#[derive(Clone, PartialEq)]
pub struct EthereumProviderState {
    pub ethereum: UseEthereum,
}

#[derive(Properties, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(EthereumContextProvider)]
pub fn ethereum_context_provider(props: &Props) -> Html {
    let ethereum = use_ethereum();

    html! {
        <ContextProvider<UseEthereum> context={ethereum}>
            {for props.children.iter()}
        </ContextProvider<UseEthereum>>
    }
}
#[derive(Clone, Debug)]
pub struct UseEthereum {
    pub ethereum: UseStateHandle<Ethereum>,
    pub connected: UseStateHandle<bool>,
    pub accounts: UseStateHandle<Option<Vec<Address>>>,
    pub chain_id: UseStateHandle<Option<u64>>,
    pub pairing_url: UseStateHandle<Option<String>>,
}

impl PartialEq for UseEthereum {
    fn eq(&self, other: &Self) -> bool {
        self.ethereum == other.ethereum
            && self.connected == other.connected
            && self.accounts == other.accounts
            && self.chain_id == other.chain_id
            && self.pairing_url == other.pairing_url
    }
}

impl UseEthereum {
    /// Connect to the wallet (defined by type)
    pub fn connect(&mut self, wallet_type: WalletType) {
        // We check if it is possible to connect
        let mut this = self.clone();
        this.disconnect();
        if (*self.ethereum).is_available(wallet_type) {
            spawn_local(async move {
                let mut eth = (*this.ethereum).clone();
                if eth.connect(wallet_type).await.is_ok() {
                    this.ethereum.set(eth);
                }
            });
        } else {
            error!("This wallet type is unavailable!");
        }
    }

    /// Gets a provider you can feed to ethers constructors to start interaction with wallet and
    /// the network
    pub fn provider(&self) -> Provider<Ethereum> {
        let eth = (*self.ethereum).clone();
        Provider::<Ethereum>::new(eth)
    }

    /// Disconnect from wallet
    pub fn disconnect(&mut self) {
        if self.is_connected() {
            let mut eth = (*self.ethereum).clone();
            let ethereum = self.ethereum.clone();
            spawn_local(async move {
                let _ = eth.disconnect().await;
                ethereum.set(eth);
            });
        }
    }

    /// Checks if any wallet is currently connected
    pub fn is_connected(&self) -> bool {
        *self.connected
    }

    /// Checks if injected wallet is available in current context
    pub fn injected_available(&self) -> bool {
        (*self.ethereum).injected_available()
    }

    /// Checks if wallet connect is available in current configuration
    pub fn walletconnect_available(&self) -> bool {
        (*self.ethereum).walletconnect_available()
    }

    /// Gets current chain id of connected wallet
    pub fn chain_id(&self) -> u64 {
        (*self.chain_id).unwrap_or(0)
    }

    /// Gets a list of all accounts from connected wallet for chosen (and set) network
    pub fn accounts(&self) -> Option<&Vec<Address>> {
        (*self.accounts).as_ref()
    }

    /// Signs typed data with the wallet
    pub async fn sign_typed_data<T: Send + Sync + Serialize>(
        &self,
        data: T,
        from: &Address,
    ) -> Result<Signature, EthereumError> {
        (*self.ethereum).sign_typed_data(data, from).await
    }
}

#[hook]
pub fn use_ethereum() -> UseEthereum {
    let mut builder = EthereumBuilder::new();

    if let Some(project_id) = std::option_env!("PROJECT_ID") {
        builder.walletconnect_id(project_id);
    }
    if let Some(rpc_url) = std::option_env!("RPC_URL") {
        builder.rpc_node(rpc_url);
    }
    let connected = use_state(move || false);
    let accounts = use_state(move || None as Option<Vec<Address>>);
    let chain_id = use_state(move || None as Option<u64>);
    let pairing_url = use_state(move || None as Option<String>);

    let ethereum = use_state(move || builder.url("http://localhost").build());

    let con = connected.clone();
    let acc = accounts.clone();
    let cid = chain_id.clone();
    let purl = pairing_url.clone();

    use_effect_with(ethereum.clone(), move |ethereum| {
        if ethereum.has_provider() {
            let eth = ethereum.clone();
            spawn_local(async move {
                let mut keep_looping = true;
                while keep_looping {
                    match eth.next().await {
                        Ok(Some(event)) => match event {
                            Event::ConnectionWaiting(url) => {
                                purl.set(Some(url));
                            }
                            Event::Connected => {
                                con.set(true);
                                purl.set(None)
                            }
                            Event::Disconnected => {
                                con.set(false);
                                acc.set(None);
                                cid.set(None);
                                keep_looping = false;
                            }
                            Event::Broken => { /* we swallow this event and waiting for restart */ }
                            Event::ChainIdChanged(chain_id) => cid.set(chain_id),
                            Event::AccountsChanged(accounts) => acc.set(accounts),
                        },
                        Ok(None) => {}
                        Err(err) => {
                            keep_looping = false;
                            error!("Error on fetching event message {err:?}");
                        }
                    }
                }
                debug!("Listener loop ended");
            });
        }
    });

    let eth = ethereum.clone();
    yew_hooks::use_effect_once(move || {
        spawn_local(async move {
            let mut e = (*eth).clone();
            if e.restore().await {
                eth.set(e);
            }
        });
        || {}
    });

    UseEthereum { ethereum, connected, accounts, chain_id, pairing_url }
}
