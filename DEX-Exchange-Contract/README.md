# Pixpel Swap - Decentralized Exchange (DEX) on Concordium

`pixpel_swap` is a decentralized exchange (DEX) smart contract built on the Concordium blockchain, implementing the CIS-2 standard for token interactions. It enables users to create liquidity pools, add/remove liquidity, and perform token swaps (CCD-to-token, token-to-CCD, and token-to-token). The contract supports LP token minting/burning and includes a fee mechanism for liquidity providers and swap operations.

## Features

- **Liquidity Pools**: Create and manage liquidity pools for CIS-2 tokens paired with CCD.
- **Add/Remove Liquidity**: Users can add liquidity (CCD and tokens) to mint LP tokens or remove liquidity to burn LP tokens and reclaim assets.
- **Swaps**: Supports three swap types:
  - CCD to Token
  - Token to CCD
  - Token to Token (via CCD as an intermediary)
- **Fee Structure**: 1% fee on swaps (configurable via `FEE` constant).
- **CIS-2 Compliance**: Supports CIS-2 token standards for interoperability.
- **View Functions**: Provides view entrypoints to query exchange states, balances, and swap amounts.

## Contract Details

- **Contract Name**: `pixpel_swap`
- **Blockchain**: Concordium
- **Language**: Rust with `concordium-std`
- **Standards**: CIS-0, CIS-2
- **Base URL for Token Metadata**: `https://concordium-servernode.dev-site.space/api/v1/metadata/swap/lp-tokens`

## Prerequisites

- **Rust Toolchain**: Install Rust and Cargo (https://rustup.rs/).
- **Concordium SDK**: Install `cargo-concordium` for building and testing (https://developer.concordium.software/en/mainnet/smart-contracts/guides/setup-tools.html).
- **Node Client**: Concordium node client for deployment and interaction.

## Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd pixpel_swap
   ```
2. Install dependencies:

   cargo build

3. Build the contract:

   cargo concordium build --out pixpel_swap.wasm.v1
   `

4. Testing

   cargo concordium test
