use crate::tests::{add_item_for_auction, initialize_chain_and_auction, ALICE, ALICE_ADDR};
use crate::{error, params::AddItemParameter};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID};
use concordium_std::{Address, Amount, Duration, Timestamp};

/// This testcase is to test negative by trying to invoke `addItem` through some valid contract.
/// Auction contract should in principle reject the invocation with reason -4 (Error::OnlyAccount).
///
/// The result is then verified by asserting the error received after invocation
#[test]
fn add_item_by_contract() {
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

    // Adding the item for auction.
    let update_result = add_item_for_auction(
        &mut chain,
        auction_contract,
        ALICE,
        Address::Contract(cis2_contract),
        parameter,
    );

    assert!(update_result.is_err());
    assert_eq!(
        error::Error::OnlyAccount,
        update_result.err().unwrap().into()
    )
}

/// This testcase is to test negative by trying to invoke `addItem` with expired auction timeline.
/// This test case checks two negatives:
///
/// - First condition is that the item is added with inconsistent timelline for auction, end time of auction
///   is less than the auction start time. In principle, the invocation should fail with reason -2
///
/// - Second condition is that the chain is fast forwarded in time by 1 second since unix epoch, and then
///   the item is added with end time less than 0.5 secs than block time. This invocation should fail with
///   reason -3
///
/// The result is then verified by asserting the error received after invocation
#[test]
fn add_item_expired() {
    let (mut chain, _, auction_contract, cis2_contract) = initialize_chain_and_auction();

    // Creating params for contract addItem invocation
    let parameter = AddItemParameter {
        name: "SomeItem".to_string(),
        start: Timestamp::from_timestamp_millis(5000),
        end: Timestamp::from_timestamp_millis(1000),
        token_id: TokenID(1),
        minimum_bid: Amount::from_ccd(10),
        cis2_contract,
        token_amount: TokenAmount(1),
    };

    // Adding the item for auction.
    let update_result =
        add_item_for_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, parameter);

    assert!(update_result.is_err());
    assert_eq!(
        error::Error::StartEndTimeError,
        update_result.err().unwrap().into()
    );

    // Fast forwarding the chain block time by 1 second
    let _ = chain.tick_block_time(Duration::from_millis(1000));

    // Creating params for contract addItem invocation
    let parameter = AddItemParameter {
        name: "SomeItem".to_string(),
        start: Timestamp::from_timestamp_millis(300),
        end: Timestamp::from_timestamp_millis(chain.block_time().millis - 500),
        token_id: TokenID(1),
        minimum_bid: Amount::from_ccd(10),
        cis2_contract,
        token_amount: TokenAmount(1),
    };

    // Adding the item for auction.
    let update_result =
        add_item_for_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, parameter);

    assert!(update_result.is_err());
    assert_eq!(
        error::Error::EndTimeError,
        update_result.err().unwrap().into()
    );
}
