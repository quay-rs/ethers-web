use yew::{function_component, html, Children, ContextProvider, Html, Properties};

use crate::ethereum::{use_ethereum, UseEthereum};

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
