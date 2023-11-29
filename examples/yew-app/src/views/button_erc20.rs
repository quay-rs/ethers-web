use crate::ethereum::UseEthereum;
use ethers::{
    contract::abigen,
    types::{H160, U256},
};
use log::{error, info};
use yew::{platform::spawn_local, prelude::*};

abigen!(TokenContract, "abi/ERC20.json");

async fn transfer(ethereum: &UseEthereum, token_address: H160, to: H160, amount: U256) {
    let erc20_contract = TokenContract::new(token_address, ethereum.provider().into());
    info!("Trying to execute transaction...");
    if let Ok(tx) = erc20_contract
        .transfer(to, amount)
        .from(ethereum.account())
        .send()
        .await
    {
        info!("Transaction commited, awaiting blockchain verification");
        match tx.await {
            Ok(_) => info!("Token transfered"),
            Err(err) => error!("Token transfer failed {err:?}"),
        }
    }
}

#[function_component(TransferButton)]
pub fn transfer_button() -> Html {
    let ethereum = use_context::<UseEthereum>().expect(
        "No ethereum found. You must wrap your components in an <EthereumContextProvider />",
    );

    // This is simple - we use it the same way we use ethers

    let onclick = {
        let ethereum = ethereum.clone();
        Callback::from(move |_: MouseEvent| {
            if ethereum.is_connected() {
                let eth = ethereum.clone();
                spawn_local(async move {
                    transfer(
                        &eth,
                        "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
                            .parse::<H160>()
                            .unwrap(),
                        "0xBE565E3eEFcfd58920FfB5048292f67F431356eF"
                            .parse::<H160>()
                            .unwrap(),
                        U256::from(100000000),
                    )
                    .await;
                });
            } else {
                info!("Are we disconnected?");
            }
        })
    };
    html! {
        <button {onclick} disabled={!ethereum.is_connected()}>{"Test transfer"}</button>
    }
}
