use crate::{
    error::Error,
    params::{AddItemParameter, BidParams},
    state::AuctionState,
};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID};
use concordium_smart_contract_testing::{Chain, Energy, UpdateContractPayload};
use concordium_std::{
    AccountAddress, Address, Amount, ContractAddress, OwnedParameter, OwnedReceiveName, Timestamp,
};

use super::{
    bid_on_item, get_item_state, initialize_chain_and_auction, ALICE, ALICE_ADDR, BOB, BOB_ADDR,
    SIGNER,
};

fn cancel_auction(
    chain: &mut Chain,
    auction_contract: ContractAddress,
    invoker: AccountAddress,
    sender: Address,
    item_index: u16,
) -> Result<(), Error> {
    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        address: auction_contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.cancel".to_string()),
        message: OwnedParameter::from_serial(&item_index)
            .expect("[Error] Unable to serialize bid_params"),
    };

    // ALICE cancels the item auction
    let cancel_result =
        chain.contract_update(SIGNER, invoker, sender, Energy::from(10000), payload);

    match cancel_result {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// A smoke test case implemented to verify the basic flow of auction cancelation by the creator expecting
/// no negatives except bid finalization. It verifies the following flow:
///
/// - ALICE adds an item for the auction in contract with minimum bid amount in CCD 10.
/// - BOB bids on that item with amount 15 in CCD, higher than minimum bid.
/// - ALICE cancels the auction she has initialized early, and the amount in CCD is refunded to BOB
///
/// Test end once all of the above assertions are varified
#[test]
fn cancel_auction_smoke() {
    let (mut chain, _, auction_contract, cis2_contract) = initialize_chain_and_auction();

    // Creating params for contract addItem invocation
    let parameter = AddItemParameter {
        name: "SomeItem".to_string(),
        start: Timestamp::from_timestamp_millis(1000),
        end: Timestamp::from_timestamp_millis(5000),
        token_id: TokenID(1),
        minimum_bid: Amount::from_ccd(10),
        cis2_contract,
        token_amount: TokenAmount(1),
    };

    let payload = UpdateContractPayload {
        amount: Amount::from_ccd(0),
        address: auction_contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.addItem".to_string()),
        message: OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
    };

    // ALICE adds some item in the contract
    let _ = chain
        .contract_update(SIGNER, ALICE, ALICE_ADDR, Energy::from(10000), payload)
        .expect("[Error] Invocation failed while invoking 'addItem' ");

    let bob_balance_before_bid = chain.account_balance(BOB);

    // Getting parameters for bidding
    let bid_params = BidParams {
        token_id: TokenID(1u8),
        item_index: 1,
    };

    // BOB bids on the item added by ALICE
    let _ = bid_on_item(
        &mut chain,
        auction_contract,
        BOB,
        BOB_ADDR,
        Amount::from_ccd(15),
        bid_params,
    )
    .expect("[Error] Unable to place bid, invocation failed");

    let bob_balance_after_bid = chain.account_balance(BOB);

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    // Verify whether the bid has been placed by BOB, the amount is transfered
    // by the BOB account to the contract
    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bid, Amount::from_ccd(15));
    assert_eq!(item.highest_bidder, Some(BOB));
    assert!(bob_balance_before_bid > bob_balance_after_bid);

    // ALICE cancels the item auction
    let _ = cancel_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, 1u16);

    let bob_balance_refunded = chain.account_balance(BOB);

    // Getting the item state for verification
    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    // Verify whether the auction has been canceled and contract has refunded the BOB
    // his amount he placed in previous bid
    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bid, Amount::from_ccd(15));
    assert_eq!(item.highest_bidder, Some(BOB));
    assert_eq!(
        Some(Amount::zero()),
        chain.contract_balance(auction_contract)
    );
    assert!(item.auction_state == AuctionState::Canceled);
    assert!(bob_balance_refunded > bob_balance_after_bid);
}

/// This testcase is expected to verify multiple negatives of auction cancelation except bid finalization.
/// It verifies the following flow:
///
/// - ALICE establishes two auctions for two separate items (item_1 and item_2).
/// - BOB tries to cancel the item_1 auction, which then should be rejected by the contract
///   since either the auction creator or contract owner is only authorized for this task.
/// - Contract Owner tries to cancel the item_2 auction, which is then expected to be a successful
///   transaction.
///
/// Test end once all of the above assertions are varified
#[test]
fn cancel_auction_unauthorize() {
    let (mut chain, _, auction_contract, cis2_contract) = initialize_chain_and_auction();

    // Creating Two item parameters to add for auction
    let item_1 = AddItemParameter {
        name: "SomeItem-1".to_string(),
        start: Timestamp::from_timestamp_millis(1000),
        end: Timestamp::from_timestamp_millis(5000),
        token_id: TokenID(1),
        minimum_bid: Amount::from_ccd(10),
        cis2_contract,
        token_amount: TokenAmount(1),
    };

    let item_2 = AddItemParameter {
        name: "SomeItem-2".to_string(),
        start: Timestamp::from_timestamp_millis(1000),
        end: Timestamp::from_timestamp_millis(5000),
        token_id: TokenID(2),
        minimum_bid: Amount::from_ccd(10),
        cis2_contract,
        token_amount: TokenAmount(1),
    };

    let payload_1 = UpdateContractPayload {
        amount: Amount::from_ccd(0),
        address: auction_contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.addItem".to_string()),
        message: OwnedParameter::from_serial(&item_1).expect("Serialize parameter"),
    };

    let payload_2 = UpdateContractPayload {
        amount: Amount::from_ccd(0),
        address: auction_contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.addItem".to_string()),
        message: OwnedParameter::from_serial(&item_2).expect("Serialize parameter"),
    };

    // ALICE establishes two auctions with two items
    let _ = chain
        .contract_update(SIGNER, ALICE, ALICE_ADDR, Energy::from(10000), payload_1)
        .expect("[Error] Invocation failed while invoking 'addItem' ");

    let _ = chain
        .contract_update(SIGNER, ALICE, ALICE_ADDR, Energy::from(10000), payload_2)
        .expect("[Error] Invocation failed while invoking 'addItem' ");

    // Item indexes of the two items listed for two separate
    // auctions in contract
    let item_index_1 = 1u16;
    let item_index_2 = 2u16;

    // BOB tries to cancel the item_1 auction established by ALICE
    let cancel_result = cancel_auction(&mut chain, auction_contract, BOB, BOB_ADDR, item_index_1);
    let item_state = get_item_state(&chain, auction_contract, BOB, item_index_1);

    // Verify whether the transaction is reject since BOB is neither
    // the creator of auction nor is he owner of contract
    assert_eq!(item_state.creator, ALICE);
    assert_eq!(Some(Error::UnAuthorized), cancel_result.err());
    assert_eq!(item_state.auction_state, AuctionState::NotSoldYet);

    // Now ALICE tries to cancel the item_1 auction she which she
    // actually initialized
    let cancel_result = cancel_auction(
        &mut chain,
        auction_contract,
        ALICE,
        ALICE_ADDR,
        item_index_1,
    );
    let item_state = get_item_state(&chain, auction_contract, BOB, item_index_1);

    // Verify this time transaction is successful and the auction
    // is canceled, since ALICE is the actual creator
    assert!(cancel_result.is_ok());
    assert_eq!(item_state.creator, ALICE);
    assert_eq!(item_state.auction_state, AuctionState::Canceled);

    // Getting the auction contract owner address who initialized
    // the contract on chain
    let auction_contract_owner = chain.get_contract(auction_contract).unwrap().owner;

    // Now contract owner tries to cnacel item_2 auction whose creator
    // is actually ALICE
    let cancel_result = cancel_auction(
        &mut chain,
        auction_contract,
        auction_contract_owner,
        Address::Account(auction_contract_owner),
        item_index_2,
    );
    let item_state = get_item_state(&chain, auction_contract, BOB, item_index_2);

    // Verify this time transaction is successful since contract
    // owner is allowed to cnacel the auction
    assert!(cancel_result.is_ok());
    assert_eq!(item_state.creator, ALICE);
    assert_eq!(item_state.auction_state, AuctionState::Canceled);
}
