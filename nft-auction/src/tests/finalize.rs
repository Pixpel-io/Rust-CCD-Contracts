use crate::{
    error::Error,
    params::{AddItemParameter, BidParams},
    state::AuctionState,
    tests::{
        add_item_for_auction, ensure_is_operator_of, finalize_auction, get_token_balance,
        mint_token, update_operator_of,
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

/// A smoke testcase implemented to verify the full flow of whole auction process till finaliation expecting
/// no negatives. It verifies the following flow:
///
/// - ALICE adds 8 cis2 tokens for the auction in contract with minimum bid amount in CCD 10.
/// - BOB bids on that item with amount 50 in CCD, higher than minimum bid.
/// - CAROL then bids on that same item with much higher amount than BOB 100 CCD.
/// - Acution get past by the auction end time and the ALICE finalizes her auction.
/// - Contract transfers the highest bid in CCD to ALICE and the tokens are transfered to the
///   highest bidder CAROL.
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

    // Getting the account balances of ALICE and CAROL in CCD
    let alice_balance_before = chain.account_balance(ALICE);
    let carol_balance_before = chain.account_balance(CAROL);

    // Verify if tokens correctly minted for ALICE account
    assert_eq!(alice_tokens.0, [TokenAmount(100u64)]);

    // Updating CIS2 contract to make auction contract an operator
    // of ALICE
    update_operator_of(
        &mut chain,
        ALICE,
        ALICE_ADDR,
        Address::Contract(auction_contract),
        cis2_contract,
    );

    // Ensuring that the operator is successfully updated
    assert!(ensure_is_operator_of(
        &mut chain,
        ALICE,
        ALICE_ADDR,
        Address::Contract(auction_contract),
        cis2_contract
    ));

    // Creating params for ALICE to list 8 tokens in auction
    let parameter = AddItemParameter {
        name: "ALICE-Token".to_string(),
        start: Timestamp::from_timestamp_millis(1000),
        end: Timestamp::from_timestamp_millis(5000),
        token_id: ALICE_TOKEN_ID,
        minimum_bid: Amount::from_ccd(10),
        cis2_contract,
        token_amount: TokenAmount(8),
    };

    // ALICE lists the item for auction
    let _ = add_item_for_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, parameter);

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    // Verify whether the item is added for the auction correctly
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

    // BOB bid on ALICE's established auction
    let _ = bid_on_item(
        &mut chain,
        auction_contract,
        BOB,
        BOB_ADDR,
        Amount::from_ccd(50),
        bid_params,
    );

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    // Verify the BOB's bid placed correctly
    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bid, Amount::from_ccd(50));
    assert_eq!(item.highest_bidder, Some(BOB));
    assert_eq!(item.token_amount, TokenAmount(8));

    let bob_balance_after_bid = chain.account_balance(BOB);

    let bid_params = BidParams {
        token_id: ALICE_TOKEN_ID,
        item_index: 1,
    };

    // Carol placed much higher bid than BOB on the ALICE's
    // auction
    let _ = bid_on_item(
        &mut chain,
        auction_contract,
        CAROL,
        CAROL_ADDR,
        Amount::from_ccd(100),
        bid_params,
    );

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    let bob_balance_refunded = chain.account_balance(BOB);

    // Verify the CAROL's bid placed correctly
    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bid, Amount::from_ccd(100));
    assert_eq!(item.highest_bidder, Some(CAROL));
    assert_eq!(item.token_amount, TokenAmount(8));

    // Ensure that BOB is refunded after CAROL's highest bid
    assert!(bob_balance_refunded > bob_balance_after_bid);

    // Fast forwarding the chain in time by 10 seconds
    let _ = chain.tick_block_time(Duration::from_seconds(10));

    // ALICE finalies her auction
    let _ = finalize_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, 1)
        .expect("[Error] Unable to finalize Alice auction, invocation failed");

    let alice_cis2_tokens = get_token_balance(&chain, ALICE, cis2_contract, ALICE_TOKEN_ID);
    let carol_cis2_tokens = get_token_balance(&chain, CAROL, cis2_contract, ALICE_TOKEN_ID);

    let alice_balance_after = chain.account_balance(ALICE);
    let carol_balance_after = chain.account_balance(CAROL);

    // Verify that the 8 CIS2 tokens (NFT) is correctly transfered
    // to CAROL from ALICE after winning the auction
    assert_eq!(alice_cis2_tokens.0, [TokenAmount(92)]);
    assert_eq!(carol_cis2_tokens.0, [TokenAmount(8)]);

    // Verify that highest bid amount in CCD  is correctly transfered
    // to ALICE from CAROL after finalization
    assert!(alice_balance_after > alice_balance_before);
    assert!(carol_balance_after < carol_balance_before);

    let item = get_item_state(&chain, auction_contract, ALICE, 1);

    // Verify that the ALICE's auction is finalized and closed
    assert_eq!(item.creator, ALICE);
    assert_eq!(item.highest_bidder, Some(CAROL));
    assert_eq!(item.auction_state, AuctionState::Sold(CAROL));
}
