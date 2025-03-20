# NFT-Auction

Smart contract based on concordium blockchain to implement a NFT (CIS2-Tokens) auction mechanism. This repository
contains the core logic for auction flow to be executed on concordium blockchain written in rust.

## Setup

To build and test the contract, we must have done the prerequisite setup defined in the repository [README](../README.md) 

## Build and Run

Once everything is setup, now we can build the contract and deploy it on the concodium testnet. To build, deploy and
interact with the contract, you can use these [commands](./commands.md) with these [schema-artifacts](./schema-artifacts/) using `concordium-client` cli tool.

Or we can use the official concorium frontend tool for deploying and interacting the concordium smart contracts found here [sctool](https://sctools.mainnet.concordium.software/?__hstc=206253644.9e573ad0dcf77e4d730f208e53ab0481.1736862510663.1737015924307.1737026584228.5&__hssc=206253644.4.1737026584228&__hsfp=706028811)

## Testing

Several unit tests are implemented for funcntional and logical testing of the contract with aid of rust integration
testing framework. Unit tests can be run as:

```bash
# To run all available unit tests
cargo test tests

# To run a specefic module of unit tests
cargo test tests::bid

# To run a specific unit test of a certain module
cargo test tests::bid::bid_smoke
```
All of the availble unit tests are found here [Tests](./src/tests)


**Note:** This contract might fail for the following [reasons](./src/error.rs) listed as the errors.