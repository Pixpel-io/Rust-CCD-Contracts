//! # Implementation of an auction smart contract
//!
//! The contract is initialized with an empty stateof items.
//! CCD coins are used as payments for the CIS2 token token when auctioning
//! an item within this contract.
//!
//! To initiate a new auction, any account can call the `addItem` entry point.
//! The account initiating the auction (referred to as the creator) is required
//! to specify the start time, end time, minimum bid, and the `token_id`
//! associated with the item. At this stage, the item/auction is assigned the
//! next consecutive index for future reference.
//!
//! Any account can bid for an item.
//! The `bid` entry point in this contract can be incokved by anyone using concordium
//! however this function is 'payable', whoever bids for an item will have to pay the
//! amount to the contract. If the bidder exceeds the current highest bid amount, the bid
//! is accepted and the previous highest bidder is refunded its amount in CCD.
//!
//! The smart contract keeps track of the current highest bidder as well as
//! the CCD amount of the highest bid. The net balance of the smart
//! contract represent the sums of all highest bids from the items (that haven't
//! been finalized). When a new highest bid is accepted by the smart
//! contract, the smart contract refunds the old highest bidder.
//!
//! Bids have to be placed before the auction ends. The participant with the
//! highest bid (the last accepted bidder) wins the auction.
//!
//! After the auction ends for a specific item, only the creator or the contract owwner can
//! finalize the auction. The creator of that auction receives the highest bid amount when the
//! auction is finalized and the item is transfered and marked as sold to the highest bidder.
//! This can be done only once.
#![cfg_attr(not(feature = "std"), no_std)]

use concordium_cis2::{Cis2Client, *};
use concordium_std::{collections::BTreeMap, *};
use error::*;
#[allow(unused)]
use params::{AddItemParameter, AdditionalDataIndex, BidParams, ReturnParamView};
use state::{AuctionState, ItemState, State};

pub mod error;
pub mod params;
pub mod state;
pub mod view;

/// Contract token ID type. It has to be the `ContractTokenId` from the cis2
/// token contract.
pub type ContractTokenId = TokenIdU8;

/// Contract token amount. It has to be the `ContractTokenAmount` from the cis2
/// token contract.
pub type ContractTokenAmount = TokenAmountU64;

/// TransferParameter type that has a specific `ContractTokenId` and
/// `ContractTokenAmount` set.
pub type TransferParameter = TransferParams<ContractTokenId, ContractTokenAmount>;

/// Tagged event to be serialized for the event log.
#[derive(Serialize)]
pub enum Event {
    AddItem(AddItemEvent),
}

/// The AddItemEvent is logged when an item is added to the auction.
#[derive(Serialize)]
pub struct AddItemEvent {
    /// The index of the item added.
    pub item_index: u16,
}

// Implementing a custom schemaType for the `Event` struct.
// This custom implementation flattens the fields to avoid one
// level of nesting. Deriving the schemaType would result in e.g.:
// {"AddItemEvent": [{...fields}] }. In contrast, this custom schemaType
// implementation results in e.g.: {"AddItemEvent": {...fields} }
impl schema::SchemaType for Event {
    fn get_type() -> schema::Type {
        let mut event_map = BTreeMap::new();
        event_map.insert(
            0u8,
            (
                "AddItemEvent".to_string(),
                schema::Fields::Named(vec![(String::from("item_index"), u16::get_type())]),
            ),
        );
        schema::Type::TaggedEnum(event_map)
    }
}

/// Init entry point that creates a new auction contract with an empty State
/// and sets the counter to 0
#[init(contract = "cis2-auction", event = "Event")]
fn auction_init(_ctx: &InitContext, state_builder: &mut StateBuilder) -> InitResult<State> {
    // Creating `State`.
    let state = State {
        items: state_builder.new_map(),
        counter: 0,
    };
    Ok(state)
}

/// AddItem entry point to add an item to this contract. To initiate a new
/// auction, any account can call this entry point. The account initiating the
/// auction (referred to as the creator) is required to specify the start time,
/// end time, minimum bid, `token_id` associated with the item and `token_amount`
/// to be sold in the auction along with the contract address `cis2_contract`, that
/// will be utilized while transfering the token to the winner. At this stage, the 
/// item/auction is assigned the next consecutive index for future reference.
#[receive(
    contract = "cis2-auction",
    name = "addItem",
    parameter = "AddItemParameter",
    error = "Error",
    enable_logger,
    mutable
)]
fn add_item(
    ctx: &ReceiveContext,
    host: &mut Host<State>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    // Ensure that only accounts can add an item.
    let sender_address = match ctx.sender() {
        Address::Contract(_) => bail!(Error::OnlyAccount),
        Address::Account(account_address) => account_address,
    };

    // Getting input parameters.
    let item: AddItemParameter = ctx.parameter_cursor().get()?;

    // Ensure start < end.
    ensure!(item.start < item.end, Error::StartEndTimeError);

    let block_time = ctx.metadata().block_time();
    // Ensure the auction can run.
    ensure!(block_time <= item.end, Error::EndTimeError);

    // Assign an index to the item/auction.
    let item_index = host.state_mut().counter + 1;
    host.state_mut().counter = item_index;

    // Insert the item into the state.
    let _ = host.state_mut().items.insert(
        item_index,
        ItemState {
            auction_state: AuctionState::NotSoldYet,
            highest_bidder: None,
            name: item.name,
            end: item.end,
            start: item.start,
            highest_bid: item.minimum_bid,
            creator: sender_address,
            token_id: item.token_id,
            cis2_contract: item.cis2_contract,
            token_amount: item.token_amount,
        },
    );

    // Event for added item.
    logger.log(&Event::AddItem(AddItemEvent { item_index }))?;

    Ok(())
}

/// The `bid` entry point in this contract can be invoked by anyone. This
/// function is `payable`, means whoever is invoking this must pay the amount
/// to the contract that it is bidding. Contract will hold the amount, and if
/// any new highest bidder comes in, the amount will be refunded to the previous
/// bidder. However, it is to be noted that the item owner of an auction can not
/// bid on its own item.
///
/// While invoking this function, the bidder must provide the `item_index` and
/// `token_id` as the bid parameters.
#[receive(
    contract = "cis2-auction",
    name = "bid",
    mutable,
    payable,
    parameter = "BidParams",
    error = "Error"
)]
fn auction_bid(ctx: &ReceiveContext, host: &mut Host<State>, amount: Amount) -> ContractResult<()> {
    // Ensure the sender is the cis2 token contract
    let bidder_address = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };

    // Getting input parameters.
    let params: BidParams = ctx.parameter_cursor().get()?;

    let mut item = host
        .state_mut()
        .items
        .entry(params.item_index)
        .occupied_or(Error::NoItem)?;

    // Ensuring that the auction creator is not bidding on its own item
    ensure!(bidder_address != item.creator, Error::CreatorCanNotBid);

    // Ensure the token_id matches.
    ensure_eq!(item.token_id, params.token_id, Error::WrongTokenID);

    // Ensure the auction has not been finalized yet.
    ensure_eq!(
        item.auction_state,
        AuctionState::NotSoldYet,
        Error::AuctionAlreadyFinalized
    );

    let block_time = ctx.metadata().block_time();
    // Ensure the auction has not ended yet.
    ensure!(block_time <= item.end, Error::BidTooLate);

    // Ensure that the new bid exceeds the highest bid so far.
    let old_highest_bid = item.highest_bid;
    ensure!(amount > old_highest_bid, Error::BidNotGreaterCurrentBid);

    // Get the previous highest bid
    let previous_bid_amount = item.highest_bid;

    // Set the new highest_bid.
    item.highest_bid = amount;

    if let Some(previous_bidder) = item.highest_bidder.replace(bidder_address) {
        drop(item);

        let transfer_result = host.invoke_transfer(&previous_bidder, previous_bid_amount);
        ensure!(transfer_result.is_ok(), Error::TransferError);
    }

    Ok(())
}

/// The `finalize` entry point sends the highest bid
/// amount in CCD to the creator of the auction if the item is past its auction end
/// time, and transfers the token to the highest bidder.
///
/// This function is only meant to be invoked by the creator of the auction item, or
/// the owner of the auction contract
#[receive(
    contract = "cis2-auction",
    name = "finalize",
    parameter = "u16",
    error = "Error",
    mutable
)]
fn auction_finalize(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    let finalizer_address = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };

    // Getting input parameter.
    let item_index: u16 = ctx.parameter_cursor().get()?;

    // Get the item from state.
    let mut item = host
        .state_mut()
        .items
        .entry(item_index)
        .occupied_or(Error::NoItem)?;

    // Ensure that the finalizer is the creator of the auction or
    // the owner of the contract itself
    ensure!(
        finalizer_address == item.creator || finalizer_address == ctx.owner(),
        Error::UnAuthorized
    );

    // Ensure the auction has not been finalized yet.
    ensure_eq!(
        item.auction_state,
        AuctionState::NotSoldYet,
        Error::AuctionAlreadyFinalized
    );

    let block_time = ctx.metadata().block_time();
    // Ensure the auction has ended already.
    ensure!(block_time > item.end, Error::AuctionStillActive);

    if let Some(bidder_address) = item.highest_bidder {
        // Marking the highest bidder (the last accepted bidder) as winner of the
        // auction.
        item.auction_state = AuctionState::Sold(bidder_address);

        let auction_owner = item.creator;
        let highest_bid_amount = item.highest_bid;
        let token_id = item.token_id;
        let provided_contract = item.cis2_contract;
        let token_amount = item.token_amount;

        drop(item);

        // ENsure that the given contract is CIS2 compliant
        ensure_supports_cis2(host, &provided_contract)?;

        // Ensure that this contract is operator of Token (NFT) Owner in the
        // CIS2-contract
        ensure_is_operator(host, ctx, &provided_contract)?;

        // Creating a CIS-2 client to transfer the Token (NFT)
        let client = Cis2Client::new(provided_contract);

        // Transfering the highest bid amount (CCD) to the owner of the current auction
        let transfer_result = host.invoke_transfer(&auction_owner, highest_bid_amount);

        // Ensure that the amount (CCD) transfer is successful
        ensure!(transfer_result.is_ok(), Error::TransferError);

        // Transfering the token (NFT) listed for the auction to the highest  bidder
        // using the provided CIS-2 contract
        client.transfer::<State, ContractTokenId, ContractTokenAmount, Error>(
            host,
            Transfer {
                amount: token_amount,
                from: concordium_std::Address::Account(auction_owner),
                to: concordium_cis2::Receiver::Account(bidder_address),
                token_id,
                data: AdditionalData::empty(),
            },
        )?;
    }

    Ok(())
}

/// The `cancel` entrypoint use to cancel any active auction for an item.
/// It can only be invoke by creator of the auction or the owner of the
/// auction contract itself.
///
/// When it is invoked, the bid amount is then refunded to the bidder and
/// action is made inactive by setting the `AuctionState::Canceled`.
///
/// While invoking this entrypoint, invoker must provide a valid item index
/// for the listed item in the auction
#[receive(
    contract = "cis2-auction",
    name = "cancel",
    mutable,
    parameter = "u16",
    error = "Error"
)]
fn auction_cancel(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Ensure the sender is the cis2 token contract
    let canceler = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };

    // Getting input parameters.
    let item_index: u16 = ctx.parameter_cursor().get()?;

    let mut item = host
        .state_mut()
        .items
        .entry(item_index)
        .occupied_or(Error::NoItem)?;

    // Fetching the amount related to the highest bidder to be
    // refunded
    let bidder_amount = item.highest_bid;

    // Ensure that the canceler is the creator of the auction or
    // the owner of the contract itself
    ensure!(
        canceler == item.creator || canceler == ctx.owner(),
        Error::UnAuthorized
    );

    // Ensure the auction has not been finalized yet.
    ensure_eq!(
        item.auction_state,
        AuctionState::NotSoldYet,
        Error::AuctionAlreadyFinalized
    );

    // Updating the auction state
    item.auction_state = AuctionState::Canceled;

    // Refunding the amount in ccd if there is any highest bidder
    if let Some(previous_bidder) = item.highest_bidder {
        drop(item);

        let transfer_result = host.invoke_transfer(&previous_bidder, bidder_amount);
        ensure!(transfer_result.is_ok(), Error::TransferError);
    }

    Ok(())
}

/// Helper function that can be invoked at the front-end to serialize the
/// `AdditionalDataIndex` before generating the message to be signed in the
/// wallet.
#[receive(
    contract = "cis2-auction",
    name = "serializationHelper",
    parameter = "AdditionalDataIndex"
)]
fn contract_serialization_helper(_ctx: &ReceiveContext, _host: &Host<State>) -> ContractResult<()> {
    Ok(())
}

/// A helper function to make sure that the contract address provided by the
/// creator of auction at the time of listing the item is whether the CIS2
/// compliant or not.
fn ensure_supports_cis2(
    host: &mut Host<State>,
    cis2_contract: &ContractAddress,
) -> ContractResult<()> {
    let client = Cis2Client::new(*cis2_contract);

    match client.supports_cis2(host) {
        Err(cis2_err) => {
            bail!(Error::from(cis2_err))
        }
        Ok(supports_result) => match supports_result {
            SupportResult::Support | SupportResult::SupportBy(_) => Ok(()),
            SupportResult::NoSupport => bail!(Error::CIS2NotSupported),
        },
    }
}

/// A helper function to make sure that this contract is made operator of the
/// Token (NFT) owner in the provide CIS2 contract.
fn ensure_is_operator(
    host: &mut Host<State>,
    ctx: &ReceiveContext,
    cis2_contract: &ContractAddress,
) -> ContractResult<()> {
    let client = Cis2Client::new(*cis2_contract);

    match client.operator_of(host, ctx.sender(), Address::Contract(ctx.self_address())) {
        Ok(is_operator) => {
            ensure!(is_operator, Error::NotOperator)
        }
        Err(cis2_err) => {
            bail!(Error::from(cis2_err))
        }
    }
    Ok(())
}
