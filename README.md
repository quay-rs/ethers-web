## Quickstart

Add this to your Cargo.toml:

```toml
[dependencies]
ethers-web = "0.1"
```

...and check `examples` folder for more tricks.
You use `ethers-web` to connect providers and then use bare `ethers` to interact with the blockchain.

## Documentation

In progress of creation.

## Features

- [X] EIP1193 injected wallet implementation
- [X] WalletConnect
- [ ] Documentation
- [ ] Better examples (plus leptos example)

## Note on WASM

This library currently needs WASM to work. There is a plan to support server-side implementations, though. For now, we focus on building robust solution for WASM implementations of websites.
