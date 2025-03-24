use concordium_std::{AccountAddress, Deserial, SchemaType, Serial};

use crate::{state::Price, ContractTokenId};


#[derive(Serial, Deserial, Debug, SchemaType)]
pub struct Token {
    pub id: ContractTokenId,
    pub price: Price,
    pub quantity: u64,
    pub owner: AccountAddress
}