use crate::{
    error::Error,
    params::{AddItemParameter, BidParams},
    state::AuctionState,
};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID};
use concordium_smart_contract_testing::{Energy, UpdateContractPayload};
use concordium_std::{Amount, Duration, OwnedParameter, OwnedReceiveName, Timestamp};

use super::{
    bid_on_item, get_item_state, initialize_chain_and_auction, ALICE, ALICE_ADDR, BOB, BOB_ADDR,
    CAROL, CAROL_ADDR, SIGNER,
};

/// A smoke test case implemented to verify the basic flow of auction cancelation by the creator expecting
/// no negatives except bid finalization. It verifies the following flow:
///
/// - ALICE adds an item for the auction in contract with minimum bid amount in CCD 10.
/// - BOB bids on that item with amount 15 in CCD, higher than minimum bid.
/// - Test then validates that now the highest bidder in itemstate is BOB and the highest bid is set to 15 CCD
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

    // Item index to be sent as parameter for auction cancelation
    let item_index = 1u16;

    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        address: auction_contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.cancel".to_string()),
        message: OwnedParameter::from_serial(&item_index)
            .expect("[Error] Unable to serialize bid_params"),
    };

    // ALICE cancels the item auction
    let _ = chain
        .contract_update(SIGNER, ALICE, ALICE_ADDR, Energy::from(10000), payload)
        .expect("[Error] Unable to cancel auction, invocation error");

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
