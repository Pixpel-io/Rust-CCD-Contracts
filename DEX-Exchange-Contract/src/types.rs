use concordium_cis2::*;
use concordium_std::*;

use crate::errors::*;


//
pub type ContractTokenId = TokenIdU64;
pub type ContractTokenAmount = TokenAmountU64;

//
pub type ContractResult<A> = Result<A, ContractError>;

//
#[derive(Serial, Deserial, SchemaType, Clone, Debug)]
pub struct TokenInfo {
    pub id: TokenIdVec,
    pub address: ContractAddress,
}