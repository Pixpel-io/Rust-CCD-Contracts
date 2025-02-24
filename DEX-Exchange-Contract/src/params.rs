use concordium_std::*;
use concordium_cis2::*;

use crate::types::*;


// Exchanges

#[derive(Serial, Deserial, SchemaType)]
pub struct GetExchangeParams {
    pub holder: Address,
    pub token: TokenInfo,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct GetExchangesParams {
    pub holder: Address,
}


// LP tokens

#[derive(Serial, Deserial, SchemaType)]
pub struct MintParams {
    pub owner:  Address,
    pub tokens: collections::BTreeMap<ContractTokenId, ContractTokenAmount>,
}


pub type TransferParameter = TransferParams<ContractTokenId, ContractTokenAmount>;


pub type ContractBalanceOfQueryParams = BalanceOfQueryParams<ContractTokenId>;


pub type ContractTokenMetadataQueryParams = TokenMetadataQueryParams<ContractTokenId>;


// Liquidity pools

#[derive(Serial, Deserial, SchemaType)]
pub struct AddLiquidityParams {
    pub token: TokenInfo,
    pub token_amount: ContractTokenAmount,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct RemoveLiquidityParams {
    pub token: TokenInfo,
    pub lp_token_amount: ContractTokenAmount,
}


// Swaps

#[derive(Serial, Deserial, SchemaType)]
pub struct GetCcdToTokenSwapAmountParams {
    pub token: TokenInfo,
    pub ccd_sold: ContractTokenAmount,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct GetTokenToCcdSwapAmountParams {
    pub token: TokenInfo,
    pub token_sold: ContractTokenAmount,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct GetTokenToTokenSwapAmountParams {
    pub token: TokenInfo,
    pub purchased_token: TokenInfo,
    pub token_sold: ContractTokenAmount,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct CcdToTokenSwapParams {
    pub token: TokenInfo,
    pub min_token_amount: ContractTokenAmount,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct TokenToCcdSwapParams {
    pub token: TokenInfo,
    pub token_sold: ContractTokenAmount,
    pub min_ccd_amount: ContractTokenAmount,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct TokenToTokenSwapParams {
    pub token: TokenInfo,
    pub purchased_token: TokenInfo,
    pub token_sold: ContractTokenAmount,
    pub min_purchased_token_amount: ContractTokenAmount,
}