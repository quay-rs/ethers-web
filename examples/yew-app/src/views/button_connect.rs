use crate::ethereum::UseEthereum;
use ethers_web::WalletType;
use yew::prelude::*;

#[function_component(WalletButton)]
pub fn wallet_button() -> Html {
    let ethereum = use_context::<UseEthereum>().expect(
        "No ethereum found. You must wrap your components in an <EthereumContextProvider />",
    );

    let label = if ethereum.is_connected() {
        ethereum.main_account()
    } else {
        "Connect wallet".into()
    };
    let onclick_ethereum = {
        Callback::from(move |_: MouseEvent| {
            if ethereum.is_connected() {
                ethereum.clone().disconnect();
            } else {
                ethereum.clone().connect(WalletType::Injected);
            }
        })
    };
    html! {
        <button onclick={onclick_ethereum}>{label}</button>
    }
}
