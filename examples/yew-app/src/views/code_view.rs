use ethers_web::yew::UseEthereum;
use qrcode_generator::QrCodeEcc;
use yew::prelude::*;

#[function_component(CodeView)]
pub fn code_view() -> Html {
    let ethereum = use_context::<UseEthereum>().expect(
        "No ethereum found. You must wrap your components in an <EthereumContextProvider />",
    );

    let code = use_state(|| String::new());

    use_effect_with((*ethereum.pairing_url).clone(), {
        let code = code.clone();
        move |url| match url {
            Some(url) => {
                let png_vec = qrcode_generator::to_png_to_vec(&url, QrCodeEcc::Low, 512).unwrap();
                code.set(format!(
                    "data:image/png;base64,{}",
                    data_encoding::BASE64.encode(&png_vec)
                ));
            }
            None => code.set(String::new()),
        }
    });

    html! {
        <img src={ (*code).clone() } />
    }
}
