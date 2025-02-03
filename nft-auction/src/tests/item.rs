use crate::tests::{initialize_chain_and_auction, ALICE, SIGNER};
use crate::{error, params::AddItemParameter};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID};
use concordium_smart_contract_testing::{Energy, UpdateContractPayload};
use concordium_std::{Address, Amount, Duration, OwnedParameter, OwnedReceiveName, Timestamp};

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
    let update_result = chain.contract_update(
        SIGNER,
        ALICE,
        Address::Contract(cis2_contract),
        Energy::from(10000),
        UpdateContractPayload {
            amount: Amount::from_ccd(0),
            address: auction_contract,
            receive_name: OwnedReceiveName::new_unchecked("cis2-auction.addItem".to_string()),
            message: OwnedParameter::from_serial(&parameter).expect("Serialize parameter"),
        },
    );

    assert!(update_result.is_err());
    assert_eq!(
        error::Error::OnlyAccount,
        update_result.err().unwrap().into()
    )
}

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
    let update_result = chain.contract_update(
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
    );

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
    let update_result = chain.contract_update(
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
    );

    assert!(update_result.is_err());
    assert_eq!(
        error::Error::EndTimeError,
        update_result.err().unwrap().into()
    );
}