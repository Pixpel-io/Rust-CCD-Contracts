use crate::tests::{
    add_item_for_auction, get_token_balance, initialize_chain_and_auction, mint_token, ALICE,
    ALICE_ADDR,
};

use crate::{
    params::{AddItemParameter, ReturnParamView},
    state::{AuctionState, ItemState},
};
use concordium_cis2::{TokenAmountU64, TokenIdU8};
use concordium_smart_contract_testing::{Energy, UpdateContractPayload};
use concordium_std::{Amount, OwnedParameter, OwnedReceiveName, Timestamp};

#[test]
fn auction_smoke() {
    let (mut chain, _, auction_contract, cis2_contract) = initialize_chain_and_auction();

    // Creating params for contract addItem invocation
    let parameter = AddItemParameter {
        name: "MyItem".to_string(),
        start: Timestamp::from_timestamp_millis(1000),
        end: Timestamp::from_timestamp_millis(5000),
        token_id: TokenIdU8(1),
        minimum_bid: Amount::from_ccd(10),
        cis2_contract,
        token_amount: TokenAmountU64(1),
    };

    // Adding the item for auction.
    let _ = add_item_for_auction(&mut chain, auction_contract, ALICE, ALICE_ADDR, parameter);

    // Invoke the view entry point and check that the item was added.
    let invoke = chain
        .contract_invoke(
            ALICE,
            ALICE_ADDR,
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                receive_name: OwnedReceiveName::new_unchecked("cis2-auction.view".to_string()),
                address: auction_contract,
                message: OwnedParameter::empty(),
            },
        )
        .expect("Invoke view");

    // Catching the return value
    let rv: ReturnParamView = invoke.parse_return_value().expect("View return value");

    // Asserting if we received the correct result
    assert_eq!(
        rv,
        ReturnParamView {
            item_states: vec![(
                1,
                ItemState {
                    auction_state: AuctionState::NotSoldYet,
                    highest_bidder: None,
                    name: "MyItem".to_string(),
                    start: Timestamp::from_timestamp_millis(1000),
                    end: Timestamp::from_timestamp_millis(5000),
                    token_id: TokenIdU8(1),
                    creator: ALICE,
                    highest_bid: Amount::from_ccd(10),
                    cis2_contract: cis2_contract,
                    token_amount: TokenAmountU64(1)
                }
            )],
            counter: 1,
        }
    );
}

#[test]
fn airdrop_mint_smoke() {
    let (mut chain, _, _, cis2_contract) = initialize_chain_and_auction();

    mint_token(
        &mut chain,
        ALICE,
        cis2_contract,
        TokenIdU8(1u8),
        "//some.example/token/0".to_string(),
    );

    let balance_of_alice = get_token_balance(&chain, ALICE, cis2_contract, TokenIdU8(1u8));

    assert_eq!(balance_of_alice.0, [TokenAmountU64(100)])
}
