use concordium_cis2::{TokenAmountU64, TokenIdU8};
use concordium_std::{Amount, ContractAddress, Deserial, SchemaType, Serial, Serialize, Timestamp};

use crate::state::ItemState;

/// The return_value for the entry point `view` which returns the contract
/// state.
#[derive(Serialize, SchemaType, Debug, Eq, PartialEq)]
pub struct ReturnParamView {
    /// A vector including all items that have been added to this contract.
    pub item_states: Vec<(u16, ItemState)>,
    /// A counter that is sequentially increased whenever a new item is added to
    /// the contract.
    pub counter: u16,
}

/// The parameter for the entry point `addItem` that adds a new item to this
/// contract.
#[derive(Serialize, SchemaType)]
pub struct AddItemParameter {
    /// The item name to be sold.
    pub name: String,
    /// The time when the auction ends.
    pub end: Timestamp,
    /// The time when the auction starts.
    pub start: Timestamp,
    // The minimum bid that the first bidder has to overbid.
    pub minimum_bid: Amount,
    // The `token_id` from the cis2 token contract that the item should be sold for.
    pub token_id: TokenIdU8,
    /// The cis2 token contract. Its tokens can be used to bid for items in this
    /// contract.
    pub cis2_contract: ContractAddress,
    /// Amount of tokens to placed for a bid in auction
    pub token_amount: TokenAmountU64,
}

/// The `additionData` that has to be passed to the `bid` entry point.
#[derive(Debug, Deserial, Serial, Clone, SchemaType)]
#[concordium(transparent)]
pub struct AdditionalDataIndex {
    /// The index of the item.
    pub item_index: u16,
}

/// The parameter for the entry point `bid` that is used to bid on an active
/// listed auction `NFT`
#[derive(Serialize, SchemaType, Clone)]
pub struct BidParams {
    /// The NFT token ID to be sold
    pub token_id: TokenIdU8,
    /// The item index of an item listed in the auction contract
    pub item_index: u16,
}
