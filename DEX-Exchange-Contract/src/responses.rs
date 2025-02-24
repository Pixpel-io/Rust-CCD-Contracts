use concordium_std::*;
use concordium_cis2::*;

use crate::types::*;
use crate::state::*;


// Common

#[derive(Serialize, SchemaType)]
pub struct AddressStateView {
    #[concordium(size_length = 4)]
    pub balances:  Vec<(ContractTokenId, ContractTokenAmount)>,
    #[concordium(size_length = 4)]
    pub operators: Vec<Address>,
}

#[derive(Serialize, SchemaType)]
pub struct ExchangeStateView {
    pub token_info:  TokenInfo,
    pub exchange_state: ExchangeState,
    pub token_balance: ContractTokenAmount,

}

#[derive(Serialize, SchemaType)]
pub struct StateView {
    #[concordium(size_length = 4)]
    pub exchanges:  Vec<ExchangeStateView>,
    #[concordium(size_length = 4)]
    pub lp_tokens_state:  Vec<(Address, AddressStateView)>,
    #[concordium(size_length = 4)]
    pub lp_tokens_supply: Vec<(ContractTokenId, ContractTokenAmount)>,
    pub last_lp_token_id: ContractTokenId,
    pub contract_ccd_balance: Amount,
}


// Exchanges

#[derive(Serialize, SchemaType, Debug)]
pub struct ExchangeView {
    pub token:  TokenInfo,
    pub token_balance: ContractTokenAmount,
    pub ccd_balance: ContractTokenAmount,
    pub lp_token_id: ContractTokenId,
    pub lp_tokens_supply: ContractTokenAmount,
    pub lp_tokens_holder_balance: ContractTokenAmount,
}

#[derive(Serialize, SchemaType, Debug)]
pub struct ExchangesView {
    #[concordium(size_length = 4)]
    pub exchanges:  Vec<ExchangeView>,
}


// LP tokens

pub type ContractBalanceOfQueryResponse = BalanceOfQueryResponse<ContractTokenAmount>;


// Swaps

#[derive(Serialize, SchemaType)]
pub struct SwapAmountResponse {
    pub amount: ContractTokenAmount,
}
