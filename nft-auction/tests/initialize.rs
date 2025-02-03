use common::{initialize_chain_and_auction, ALICE, ALICE_ADDR, SIGNER};

use concordium_cis2::{TokenAmountU64, TokenIdU8};
use concordium_smart_contract_testing::{Energy, UpdateContractPayload};
use concordium_std::{Address, Amount, OwnedParameter, OwnedReceiveName, Timestamp};
use nft_auction::{
    params::{AddItemParameter, ReturnParamView},
    state::{AuctionState, ItemState},
};

mod common;

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
    let _ = chain
        .contract_update(
            SIGNER,
            ALICE,
            Address::Account(ALICE),
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::from_ccd(0),
                address: auction_contract,
                receive_name: OwnedReceiveName::new_unchecked("cis2-auction.addItem".to_string()),
                message: OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
            },
        )
        .expect("Should be able to add Item");

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
