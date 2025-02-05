use crate::{
    error::Error,
    params::{AddItemParameter, BidParams},
    state::AuctionState,
    tests::{
        add_item_for_auction, ensure_is_operator_of, finalize_auction, get_token_balance, mint_token,
        update_operator_of,
    },
};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID};
use concordium_smart_contract_testing::{Energy, UpdateContractPayload};
use concordium_std::{Address, Amount, Duration, OwnedParameter, OwnedReceiveName, Timestamp};

use super::{
    bid_on_item, get_item_state, initialize_chain_and_auction, ALICE, ALICE_ADDR, BOB, BOB_ADDR,
    CAROL, CAROL_ADDR, SIGNER,
};

const ALICE_TOKEN_ID: TokenID = TokenID(1);
const ALICE_TOKEN_URL: &str = "//some.example/token/0";

const BOB_TOKEN_ID: TokenID = TokenID(2);
const BOB_TOKEN_URL: &str = "//some.example/token/1";

const CAROL_TOKEN_ID: TokenID = TokenID(3);
const CAROL_TOKEN_URL: &str = "//some.example/token/2";

/// A smoke test case implemented to verify the basic flow of whole bidding process in the contract expecting
/// no negatives except bid finalization. It verifies the following flow:
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
fn finalize_auction_smoke() {
    let (mut chain, _, auction_contract, cis2_contract) = initialize_chain_and_auction();

    // Minting airdrop tokens for ALICE
    mint_token(
        &mut chain,
        ALICE,
        cis2_contract,
        ALICE_TOKEN_ID,
        ALICE_TOKEN_URL.to_string(),
    );

    // Getting the token balance of ALICE account
    let alice_tokens = get_token_balance(&chain, ALICE, cis2_contract, ALICE_TOKEN_ID);

    // Verify if tokens correctly minted for ALICE account
    assert_eq!(alice_tokens.0, [TokenAmount(100u64)]);

    update_operator_of(
        &mut chain,
        ALICE,
        ALICE_ADDR,
        Address::Contract(auction_contract),
        cis2_contract,
    );

    assert!(ensure_is_operator_of(
        &mut chain,
        ALICE,
        ALICE_ADDR,
        Address::Contract(auction_contract),
        cis2_contract
    ));

    // Creating params for contract addItem invocation
    let parameter = AddItemParameter {
        name: "ALICE-Token".to_string(),
        start: Timestamp::from_timestamp_millis(1000),
        end: Timestamp::from_timestamp_millis(5000),
        token_id: ALICE_TOKEN_ID,
        minimum_bid: Amount::from_ccd(10),
        cis2_contract,
        token_amount: TokenAmount(8),
    };

    let _ = add_item_for_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, parameter);

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    // Verify whether the item is added for the auction
    assert_eq!(item.creator, ALICE);
    assert_eq!(item.auction_state, AuctionState::NotSoldYet);
    assert_eq!(item.name, "ALICE-Token".to_string());
    assert_eq!(item.highest_bid, Amount::from_ccd(10));
    assert_eq!(item.highest_bidder, None);
    assert_eq!(item.token_amount, TokenAmount(8));
    assert_eq!(item.token_id, ALICE_TOKEN_ID);

    let bid_params = BidParams {
        token_id: ALICE_TOKEN_ID,
        item_index: 1,
    };

    let _ = bid_on_item(
        &mut chain,
        auction_contract,
        BOB,
        BOB_ADDR,
        Amount::from_ccd(50),
        bid_params,
    );
    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bid, Amount::from_ccd(50));
    assert_eq!(item.highest_bidder, Some(BOB));
    assert_eq!(item.token_amount, TokenAmount(8));

    let bid_params = BidParams {
        token_id: ALICE_TOKEN_ID,
        item_index: 1,
    };

    let _ = bid_on_item(
        &mut chain,
        auction_contract,
        CAROL,
        CAROL_ADDR,
        Amount::from_ccd(100),
        bid_params,
    );
    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bid, Amount::from_ccd(100));
    assert_eq!(item.highest_bidder, Some(CAROL));
    assert_eq!(item.token_amount, TokenAmount(8));

    let _ = chain.tick_block_time(Duration::from_seconds(10));

    let _ = finalize_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, 1)
        .expect("[Error] Unable to finalize Alice auction, invocation failed");

    let alice_cis2_tokens = get_token_balance(&chain, ALICE, cis2_contract, ALICE_TOKEN_ID);
    let carol_cis2_tokens = get_token_balance(&chain, CAROL, cis2_contract, ALICE_TOKEN_ID);

    assert_eq!(alice_cis2_tokens.0, [TokenAmount(92)]);
    assert_eq!(carol_cis2_tokens.0, [TokenAmount(8)]);

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bidder, Some(CAROL));
    assert_eq!(item.auction_state, AuctionState::Sold(CAROL));
}
