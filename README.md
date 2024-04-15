## Quickstart

Add this to your Cargo.toml:

```toml
[dependencies]
ethers-web = "0.1"
```

Depending on the web framework you are using you might want to enable proper feature.

For `yew` you should simply enable its feature first:


```toml
[dependencies]
ethers-web = { version = "0.1", features = [ "yew" ] }
```

In `examples` folder you will find an example yew application that is using an `yew::UseEthereum` that will allow you to connect to chosen wallet and maintain its life cycle
by fetching messages from the message loop using `next()` function.

The library does not deliver own message loop bacause of the limitations that `yew` and `leptos` are inside their `WASM` lifecycle.

There are two wallet standards implemented inside `ethers-web`.

### EIP 1193
`Eip1193` is an embedded wallet standard such as `Metamask`. To connect to it you just need to call `connect()` method and attach connected context provider as any other provider in `ethers` calls.

### WalletConnect

`WalletConnect` requires a bit more setup than just making a connection. You will need `PROJECT_ID` and additional `RPC_URL` that will be handling generic rpc calls that wallet might not support.


### Examples
Simply check `examples` folder to find example implementations you can use in your app.

## Documentation

In progress of creation. For now check `examples` folder for implementation details for both `leptos` and `yew` frameworks

## Features

- [X] EIP1193 injected wallet implementation
- [X] WalletConnect
- [ ] Proper Leptos support
- [ ] Documentation

## Note on WASM

This library currently needs WASM to work. There is a plan to support server-side implementations, though. For now, we focus on building robust solution for WASM implementations of websites.
