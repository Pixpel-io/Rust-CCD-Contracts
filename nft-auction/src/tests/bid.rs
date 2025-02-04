use crate::tests::{initialize_chain_and_auction, ALICE, CAROL, CAROL_ADDR, SIGNER};
use crate::{
    error::Error,
    params::{AddItemParameter, BidParams},
    state::ItemState,
};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID};
use concordium_smart_contract_testing::{Chain, Energy, UpdateContractPayload};
use concordium_std::{
    AccountAddress, Address, Amount, ContractAddress, OwnedParameter, OwnedReceiveName,
    Timestamp,
};

use super::{ALICE_ADDR, BOB, BOB_ADDR};

/// A helper function to invoke `viewItemState` in auction to get a specefic
/// item's current state in the auction contract
///
/// Returns the `ItemState` type or panics with error message
fn get_item_state(
    chain: &Chain,
    contract: ContractAddress,
    account: AccountAddress,
    item_index: u16,
) -> ItemState {
    let view_item_params = item_index;

    let payload = UpdateContractPayload {
        amount: Amount::from_ccd(0),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.viewItemState".to_string()),
        message: OwnedParameter::from_serial(&view_item_params)
            .expect("[Error] Unable to serialize view item params"),
    };

    let item: ItemState = chain
        .contract_invoke(
            account,
            Address::Account(account),
            Energy::from(10000),
            payload,
        )
        .expect("[Error] Invocation failed while invoking 'addItem' ")
        .parse_return_value()
        .expect("[Error] Unable to deserialize ItemState");

    item
}

/// A helper function to invoke `bid` function in auction contract to bid on an
/// item listed for auction
///
/// Returns the `Ok()` if the invocation succeeds or else `auction::Error`
fn bid_on_item(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    sender: Address,
    amount: Amount,
    bid_params: BidParams,
) -> Result<(), Error> {
    let payload = UpdateContractPayload {
        amount,
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.bid".to_string()),
        message: OwnedParameter::from_serial(&bid_params)
            .expect("[Error] Unable to serialize bid_params"),
    };

    // BOB bids on the item added by ALICE
    let invoke_result =
        chain.contract_update(SIGNER, invoker, sender, Energy::from(10000), payload);

    match invoke_result {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// A smoke test case implemented to verify the basic flow of whole bidding process in the contract expectin
/// no negatives. It verifies the following flow:
///
/// - ALICE adds an item for the auction in contract with minimum bid amount in CCD 10.
/// - Test then validates that initially there should be no highest bidder in the item state.
/// - BOB bids on that item with amount 15 in CCD, higher than minimum bid.
/// - Test then validates that now the highest bidder in itemstate is BOB and the highest bid is set to 15 CCD
/// - CAROL then bids on that same item with much higher amount than BOB 50 CCD.
/// - Test validates that now the highest bidder is CAROL and the amount is set to 50 CCD, then test further verify
///   that BOB has been refunded his bid amount in CCD by the contract.
///
/// Test end once all of the above assertions are varified
#[test]
fn bid_smoke() {
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

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    // Verify whether the item is added for the auction
    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bid, Amount::from_ccd(10));
    assert_eq!(item.highest_bidder, None);

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

    // Getting bid parameters
    let bid_params = BidParams {
        token_id: TokenID(1u8),
        item_index: 1,
    };

    // Now CAROL makes the highest bid on the same item added by ALICE
    let _ = bid_on_item(
        &mut chain,
        auction_contract,
        CAROL,
        CAROL_ADDR,
        Amount::from_ccd(50),
        bid_params,
    )
    .expect("[Error] Unable to place bid, invocation failed");

    let bob_balance_refunded = chain.account_balance(BOB);

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    // Verify whether the CAROL's bid has been placed and contract has refunded the BOB
    // his amount he placed in previous bid
    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bid, Amount::from_ccd(50));
    assert_eq!(item.highest_bidder, Some(CAROL));
    assert!(bob_balance_refunded > bob_balance_after_bid);
}
