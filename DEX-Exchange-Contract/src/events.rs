use concordium_std::*;

use crate::types::*;


#[derive(Debug, Serialize, SchemaType)]
pub enum SwapEventAction {
    BuyToken,
    SellToken,
}

#[derive(Debug, Serialize, SchemaType)]
pub struct SwapEvent {
    pub client: Address,
    pub action: SwapEventAction,
    pub double_swap: bool,
    pub token: TokenInfo,
    pub ccd_amount: ContractTokenAmount,
    pub token_amount: ContractTokenAmount,
    pub ccd_reserve: ContractTokenAmount,
    pub token_reserve: ContractTokenAmount,
    pub timestamp: Timestamp,
}

#[derive(Debug, Serial, SchemaType)]
pub enum Event {
    Swap(SwapEvent),
}