//! # Implementation of an auction smart contract
//!
//! The contract is initialized with a cis2 token contract.
//! Any `token_id` from this cis2 token contract can be used as a payment
//! token when auctioning an item within this contract.
//!
//! To initiate a new auction, any account can call the `addItem` entry point.
//! The account initiating the auction (referred to as the creator) is required
//! to specify the start time, end time, minimum bid, and the `token_id`
//! associated with the item. At this stage, the item/auction is assigned the
//! next consecutive index for future reference.
//!
//! Any account can bid for an item.
//! The `bid` entry point in this contract is not meant to be invoked directly
//! but rather through the `onCIS2Receive` hook mechanism in the cis2 token
//! contract. The `bid` entry point can be invoked via a sponsored transaction
//! mechanism (`permit` entry point) in case it is implemented in the cis2 token
//! contract. The bidding flow starts with an account invoking either the
//! `transfer` or the `permit` entry point in the cis2 token contract and
//! including the `item_index` in the `additionalData` section of the input
//! parameter. The `transfer` or the `permit` entry point will send some token
//! amounts to this contract from the bidder. If the token amount exceeds the
//! current highest bid, the bid is accepted and the previous highest bidder is
//! refunded its token investment.
//!
//! The smart contract keeps track of the current highest bidder as well as
//! the token amount of the highest bid. The token balances of the smart
//! contract represent the sums of all highest bids from the items (that haven't
//! been finalized). When a new highest bid is accepted by the smart
//! contract, the smart contract refunds the old highest bidder.
//!
//! Bids have to be placed before the auction ends. The participant with the
//! highest bid (the last accepted bidder) wins the auction.
//!
//! After the auction ends for a specific item, any account can finalize the
//! auction. The creator of that auction receives the highest bid when the
//! auction is finalized and the item is marked as sold to the highest bidder.
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

/// Init entry point that creates a new auction contract.
#[init(
    contract = "cis2-auction",
    event = "Event"
)]
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
/// end time, minimum bid, and the `token_id` associated with the item. At this
/// stage, the item/auction is assigned the next consecutive index for future
/// reference.
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
        },
    );

    // Event for added item.
    logger.log(&Event::AddItem(AddItemEvent { item_index }))?;

    Ok(())
}

/// The `bid` entry point in this contract is not meant to be invoked directly
/// but rather through the `onCIS2Receive` hook mechanism in the cis2 token
/// contract. Any account can bid for an item. The `bid` entry point can be
/// invoked via a sponsored transaction mechanism (`permit` entry point) in case
/// it is implemented in the cis2 token contract. The bidding flow starts with
/// an account invoking either the `transfer` or the `permit` entry point in the
/// cis2 token contract and including the `item_index` in the `additionalData`
/// section of the input parameter. The `transfer` or the `permit` entry point
/// will send some token amounts to this contract from the bidder. If the token
/// amount exceeds the current highest bid, the bid is accepted and the previous
/// highest bidder is refunded its token investment.
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

/// The `finalize` entry point can be called by anyone. It sends the highest bid
/// in tokens to the creator of the auction if the item is past its auction end
/// time.
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
                amount: 1.into(),
                from: concordium_std::Address::Account(auction_owner),
                to: concordium_cis2::Receiver::Account(bidder_address),
                token_id,
                data: AdditionalData::empty(),
            },
        )?;
    }

    Ok(())
}

/// View function that returns the content of the state.
#[receive(
    contract = "cis2-auction",
    name = "view",
    return_value = "ReturnParamView"
)]
fn view(_ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<ReturnParamView> {
    let state = host.state();

    let inner_state = state.items.iter().map(|x| (*x.0, x.1.clone())).collect();

    Ok(ReturnParamView {
        item_states: inner_state,
        counter: host.state().counter,
    })
}

/// ViewItemState function that returns the state of a specific item.
#[receive(
    contract = "cis2-auction",
    name = "viewItemState",
    return_value = "ItemState",
    parameter = "u16",
    error = "Error"
)]
fn view_item_state(ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<ItemState> {
    // Getting input parameter.
    let item_index: u16 = ctx.parameter_cursor().get()?;
    let item = host
        .state()
        .items
        .get(&item_index)
        .map(|x| x.to_owned())
        .ok_or(Error::NoItem)?;
    Ok(item)
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
