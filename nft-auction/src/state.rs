use concordium_cis2::{TokenAmountU64, TokenIdU8};
use concordium_std::{
    AccountAddress, Amount, ContractAddress, DeserialWithState, SchemaType, Serial, Serialize,
    StateApi, StateMap, Timestamp,
};

/// The state of an item up for auction.
/// This state can be viewed by querying the node with the command
/// `concordium-client contract invoke` using the `view_item_state` function as
/// entry point.
#[derive(Debug, Serialize, SchemaType, Clone, PartialEq, Eq)]
pub struct ItemState {
    /// State of the auction.
    pub auction_state: AuctionState,
    /// The highest bidder so far. The variant `None` represents
    /// that no bidder has taken part in the auction yet.
    pub highest_bidder: Option<AccountAddress>,
    /// The item name to be sold.
    pub name: String,
    /// The time when the auction ends.
    pub end: Timestamp,
    /// The time when the auction starts.
    pub start: Timestamp,
    /// In case `highest_bidder==None`, the minimum bid as set by the creator.
    /// In case `highest_bidder==Some(AccountAddress)`, the highest bid that a
    /// bidder has bid so far.
    pub highest_bid: Amount,
    /// The `token_id` from the cis2 token contract used as payment token.
    pub token_id: TokenIdU8,
    /// The account address that created the auction for this item.
    pub creator: AccountAddress,
    /// The cis2 token contract. Its tokens can be used to bid for items in this
    /// contract.
    pub cis2_contract: ContractAddress,
    /// Total amount of tokens placed for auction
    pub token_amount: TokenAmountU64,
}

/// The state of the smart contract.
/// This state can be viewed by querying the node with the command
/// `concordium-client contract invoke` using the `view` function as entry
/// point.
#[derive(Serial, DeserialWithState, Debug)]
#[concordium(state_parameter = "S")]
pub struct State<S = StateApi> {
    /// A mapping including all items that have been added to this contract.
    pub items: StateMap<u16, ItemState, S>,
    /// A counter that is sequentially increased whenever a new item is added to
    /// the contract.
    pub counter: u16,
}

/// The state of the auction.
#[derive(Debug, Serialize, SchemaType, Eq, PartialEq, PartialOrd, Clone)]
pub enum AuctionState {
    /// The auction is either
    /// - still accepting bids or
    /// - not accepting bids because it's past the auction end, but nobody has
    ///   finalized the auction yet.
    NotSoldYet,
    /// The auction has been finalized and the item has been sold to the
    /// winning `AccountAddress`.
    Sold(AccountAddress),
}
