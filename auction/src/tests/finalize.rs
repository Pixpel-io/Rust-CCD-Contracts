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
use concordium_std::{Address, Amount, Duration, Timestamp};

use super::{
    bid_on_item, get_item_state, initialize_chain_and_auction, ALICE, ALICE_ADDR, BOB, BOB_ADDR,
    CAROL, CAROL_ADDR,
};

const ALICE_TOKEN_ID: TokenID = TokenID(1);
const ALICE_TOKEN_URL: &str = "//some.example/token/0";

const BOB_TOKEN_ID: TokenID = TokenID(2);
const BOB_TOKEN_URL: &str = "//some.example/token/1";

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

/// This testcase is intended to verify multiple negatives while the creator of the auction finalizes
/// the auction. It verifies the following claims:
///
/// - Creator can not finalize the auction before it is past its end time.
/// - Creator can not finalize the auction with wrong item index.
/// - Creator can not finalize his own auction more than once.
///
/// Test end once all of the above assertions are varified
#[test]
fn finalize_auction_prohibited() {
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
    let bob_balance_before = chain.account_balance(BOB);

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

    // ALICE finalizes auction before the end time
    let finalize_err = finalize_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, 1).err();

    // Verify that the transaction is rejected with reason
    // AuctionStillActive
    assert_eq!(Some(Error::AuctionStillActive), finalize_err);

    // Fast forwarding the chain in time by 10 seconds
    let _ = chain.tick_block_time(Duration::from_seconds(10));

    // ALICE finalies her auction with wrong item index
    let finalize_err = finalize_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, 2).err();

    // Verify that the transaction is rejected due to wrong
    // item index with reason NoItem
    assert_eq!(Some(Error::NoItem), finalize_err);

    // ALICE finallizes the auction
    let finalize_err = finalize_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, 1).err();

    // Ensure that the finalization is successful
    assert!(finalize_err.is_none());

    let alice_cis2_tokens = get_token_balance(&chain, ALICE, cis2_contract, ALICE_TOKEN_ID);
    let bob_cis2_tokens = get_token_balance(&chain, BOB, cis2_contract, ALICE_TOKEN_ID);

    let alice_balance_after = chain.account_balance(ALICE);
    let bob_balance_after = chain.account_balance(BOB);

    // Verify that the 8 CIS2 tokens (NFT) is correctly transfered
    // to BOB from ALICE after winning the auction
    assert_eq!(alice_cis2_tokens.0, [TokenAmount(92)]);
    assert_eq!(bob_cis2_tokens.0, [TokenAmount(8)]);

    // Verify that highest bid amount in CCD  is correctly transfered
    // to ALICE from BOB after finalization
    assert!(alice_balance_after > alice_balance_before);
    assert!(bob_balance_after < bob_balance_before);

    // ALICE tries to finalize her already finalized auction
    let finalize_err = finalize_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, 1).err();

    assert_eq!(Some(Error::AuctionAlreadyFinalized), finalize_err);
}

/// This testcase is intended to verify multiple negatives while someone other than creator of the auction
/// finalizes the auction. It verifies the following claims:
///
/// - Only creator of the auction is allowed to finalize his auction.
/// - Only contract owner is allowed to finalize someone else's auction.
///
/// Test end once all of the above assertions are varified
#[test]
fn finalize_auction_unauthorized() {
    let (mut chain, _, auction_contract, cis2_contract) = initialize_chain_and_auction();

    // Minting airdrop tokens for ALICE and BOB
    for (account, token_id, url) in [
        (ALICE, ALICE_TOKEN_ID, ALICE_TOKEN_URL),
        (BOB, BOB_TOKEN_ID, BOB_TOKEN_URL),
    ] {
        mint_token(
            &mut chain,
            account,
            cis2_contract,
            token_id,
            url.to_string(),
        );
    }

    // Getting the token balance of ALICE account
    let alice_tokens = get_token_balance(&chain, ALICE, cis2_contract, ALICE_TOKEN_ID);
    let bob_tokens = get_token_balance(&chain, BOB, cis2_contract, BOB_TOKEN_ID);

    // Verify if tokens correctly minted for ALICE and BOB
    assert!(alice_tokens.0 == [TokenAmount(100u64)] && bob_tokens.0 == [TokenAmount(100u64)]);

    // Updating CIS2 contract to make auction contract an operator
    // of ALICE and BOB
    for (invoker, sender) in [(ALICE, ALICE_ADDR), (BOB, BOB_ADDR)] {
        update_operator_of(
            &mut chain,
            invoker,
            sender,
            Address::Contract(auction_contract),
            cis2_contract,
        );
    }

    assert!(ensure_is_operator_of(
        &mut chain,
        ALICE,
        ALICE_ADDR,
        Address::Contract(auction_contract),
        cis2_contract
    ));

    assert!(ensure_is_operator_of(
        &mut chain,
        BOB,
        BOB_ADDR,
        Address::Contract(auction_contract),
        cis2_contract
    ));

    // Creating params for ALICE and BOB to establish their auctions
    let auction_item_params = vec![
        (
            ALICE,
            ALICE_ADDR,
            AddItemParameter {
                name: "ALICE-Token".to_string(),
                start: Timestamp::from_timestamp_millis(1000),
                end: Timestamp::from_timestamp_millis(5000),
                token_id: ALICE_TOKEN_ID,
                minimum_bid: Amount::from_ccd(100),
                cis2_contract,
                token_amount: TokenAmount(5),
            },
        ),
        (
            BOB,
            BOB_ADDR,
            AddItemParameter {
                name: "BOB-Token".to_string(),
                start: Timestamp::from_timestamp_millis(1000),
                end: Timestamp::from_timestamp_millis(5000),
                token_id: BOB_TOKEN_ID,
                minimum_bid: Amount::from_ccd(50),
                cis2_contract,
                token_amount: TokenAmount(1),
            },
        ),
    ];

    // BOB and ALICE list their items for auctions
    for (invoker, sender, add_item_params) in auction_item_params {
        let _ = add_item_for_auction(
            &mut chain,
            auction_contract,
            invoker,
            sender,
            add_item_params,
        );
    }

    let bid_parameters = vec![
        (
            BOB,
            BOB_ADDR,
            Amount::from_ccd(200),
            BidParams {
                token_id: ALICE_TOKEN_ID,
                item_index: 1,
            },
        ),
        (
            ALICE,
            ALICE_ADDR,
            Amount::from_ccd(100),
            BidParams {
                token_id: BOB_TOKEN_ID,
                item_index: 2,
            },
        ),
    ];

    // BOB bids on ALICE and ALICE bis on BOB
    for (invoker, sender, amount, bid_params) in bid_parameters {
        let _ = bid_on_item(
            &mut chain,
            auction_contract,
            invoker,
            sender,
            amount,
            bid_params,
        );
    }

    let alice_item = get_item_state(&chain, auction_contract, CAROL, 1);
    let bob_item = get_item_state(&chain, auction_contract, CAROL, 2);

    assert!(
        alice_item.highest_bidder == Some(BOB) && alice_item.highest_bid == Amount::from_ccd(200)
    );

    assert!(
        bob_item.highest_bidder == Some(ALICE) && bob_item.highest_bid == Amount::from_ccd(100)
    );

    let _ = chain.tick_block_time(Duration::from_seconds(10));

    // BOB finalizes his auction
    let _ = finalize_auction(&mut chain, auction_contract, BOB, BOB_ADDR, 2);

    let alice_cis2_tokens = get_token_balance(&chain, ALICE, cis2_contract, BOB_TOKEN_ID);
    let bob_item = get_item_state(&chain, auction_contract, BOB, 2);

    // Verify that the BOB's auction has been finalized
    // and sold to ALICE
    assert_eq!(alice_cis2_tokens.0, [TokenAmount(1)]);
    assert_eq!(bob_item.auction_state, AuctionState::Sold(ALICE));

    let auction_contract_owner = chain.get_contract(auction_contract).unwrap().owner;

    // Updating CIS2 contract to make auction contract operator
    update_operator_of(
        &mut chain,
        auction_contract_owner,
        Address::Account(auction_contract_owner),
        Address::Contract(auction_contract),
        cis2_contract,
    );

    // For some reason ALICE did not finalized her auction and BOB
    // then requests owner of the contract to finalize it for him
    // so that he can get his winning tokens
    let _ = finalize_auction(
        &mut chain,
        auction_contract,
        auction_contract_owner,
        Address::Account(auction_contract_owner),
        1,
    );

    let bob_cis2_tokens = get_token_balance(&chain, BOB, cis2_contract, ALICE_TOKEN_ID);
    let alice_item = get_item_state(&chain, auction_contract, BOB, 1);

    // Verify that ALICE's auction has been finalized by the owner
    // and sold to BOB
    assert_eq!(bob_cis2_tokens.0, [TokenAmount(5)]);
    assert_eq!(alice_item.auction_state, AuctionState::Sold(BOB));
}
