use ethers_web::{yew::UseEthereum, WalletType};
use yew::prelude::*;

#[function_component(WalletButton)]
pub fn wallet_button() -> Html {
    let ethereum = use_context::<UseEthereum>().expect(
        "No ethereum found. You must wrap your components in an <EthereumContextProvider />",
    );

    let wc = use_state(|| false);

    let onclick = {
        let wc = wc.clone();
        Callback::from(move |_: MouseEvent| wc.set(!(*wc)))
    };

    let label = if ethereum.is_connected() {
        if let Some(acc) =
            ethereum.accounts().expect("Missing accounts! It's disconnected!").first()
        {
            format!("{acc}")
        } else {
            "Account missing".into()
        }
    } else {
        "Connect wallet".into()
    };
    let eth = ethereum.clone();
    let onclick_ethereum = {
        Callback::from(move |_: MouseEvent| {
            if ethereum.is_connected() {
                ethereum.clone().disconnect();
            } else {
                if *wc {
                    ethereum.clone().connect(WalletType::WalletConnect);
                } else {
                    ethereum.clone().connect(WalletType::Injected);
                }
            }
        })
    };
    html! {
        <>
        <input type="checkbox" {onclick} disabled={!eth.walletconnect_available()}/ ><label>{"Wallet connect"}</label>
        <button onclick={onclick_ethereum}>{label}</button>
        </>
    }
}
