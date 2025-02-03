# NFT-Auction

Smart contract based on concordium blockchain to implement a NFT (CIS2-Tokens) auction mechanism. This repository
contains the core logic for auction flow to be executed on concordium blockchain written in rust.

### Setup

To build and test the contract, we must have done the prerequisite setup defined in the repository [README](../README.md) 

### Build and Run

Once everything is setup, now we can build the contract and deploy it on the concodium testnet. To build, deploy and
interact with the contract, you can use these [commands](./commands.md) with these [schema-artifacts](./schema-artifacts/) using `concordium-client` cli tool.

Or we can use the official concorium frontend tool for deploying and interacting the concordium smart contracts found here [sctool](https://sctools.mainnet.concordium.software/?__hstc=206253644.9e573ad0dcf77e4d730f208e53ab0481.1736862510663.1737015924307.1737026584228.5&__hssc=206253644.4.1737026584228&__hsfp=706028811)


**Note:** This contract might fail for the following [reasons](./src/error.rs) listed as the errors.