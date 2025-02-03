# RUST Concordium Smart Contracts

This repo contains all core smart contract based on concordium blockchain to be used in [Pixpel](https://pixpel.io/).
All of these contracts solely belongs to Pixpel.

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

Once it is setup, then we need to install concordium-client by followind the guide [here](https://docs.concordium.com/en/mainnet/docs/installation/downloads.html#concordium-client-client-version)

### Build and Run

Every contract in this repository contains their own commands in `commands.md` and schemas in `schema-artifacts` at the
root of the project. These commands and schemas along with concordium-client can be used to test the subsequent smart contracts available in this repository.

Or can use the official concordium frontend tool for deploying, interacting and testing the concordium smart contracts found here [sctool](https://sctools.mainnet.concordium.software/?__hstc=206253644.9e573ad0dcf77e4d730f208e53ab0481.1736862510663.1737015924307.1737026584228.5&__hssc=206253644.4.1737026584228&__hsfp=706028811)

