# NFT-Auction

Smart contract based on concordium blockchain to implement a NFT (CIS2-Tokens) auction mechanism. This repository
contains the core logic for auction flow to be executed on concordium blockchain written in rust.

# Getting Started

To start using this contract, you would first require all the toolchains to build and deploy this contract.

### Rust Installation

We first have to download the rust toolchain for compilation, concordium requires rust toolchain version 1.81.0.
So once the toolchain is installed, we need to lock the version, you can use the following commands for rust setup.

```bash
# Installing rust tools 'rustc', 'cargo', 'cargo-clippy' and 'cargo-fmt'
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Sourcing the env variables of the toolchain globally
source "$HOME/.cargo/env"

# Lock the rustc verions 1.81.0 as the defaul compiler
rustup default 1.81.0

# Setting the wasm target for the concordium build process
rustup target add wasm32-unknown-unknown
```

Verify if the rust tools are installed and the correct version is locked

```bash
ructc --version

# Or you can run this command to see the active toolchain
rustup show
```

### Concordium-client Installation

Once the rust toolchain is setup, now we have to install and prepare the concordium client and build tools to build
and interact with the concordium smart contract on the blockchain. Following commands illustrates the overall setup.

```bash
# Installing the concordium build utility to compile and build rust smart contracts to wasm-32
cargo install --locked cargo-concordium
```

Once it is setup, then we need to install concordium-client by followind the guide [here](https://docs.concordium.com/en/mainnet/docs/installation/downloads.html)