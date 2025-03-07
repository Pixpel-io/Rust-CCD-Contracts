use core::{marker::PhantomData, str::FromStr};

use crate::{
    errors::LaunchPadError,
    params::{ApprovalParams, CreateParams, LockupDetails},
    state::{LiquidityDetails, Lockup, Product, TimePeriod, VestingLimits},
    tests::{
        add_launch_pad, approve_launch_pad, deposit_tokens_to_launch_pad, get_launch_pad,
        get_token_balance, initialize_chain_and_launch_pad, initialize_contract,
    },
};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdVec, TransferParams};
use concordium_smart_contract_testing::UpdateContractPayload;
use concordium_std::{schema::SchemaType, Address, Amount, ContractAddress, Deserial, OwnedParameter, OwnedReceiveName, SchemaType, Serial, Timestamp};

use super::{mint_token, view_state, ADMIN, OWNER, OWNER_TOKEN_ID, OWNER_TOKEN_URL};

#[test]
fn launch_pad_smoke() {
    let (mut chain, _, launch_pad_addr, cis2_addr) = initialize_chain_and_launch_pad();

    let dex_contract = initialize_contract::<PhantomData<u8>>(
        &mut chain,
        "module_path".into(),
        "pixpel_swap".into(),
        None,
    );

    mint_token(
        &mut chain,
        OWNER,
        cis2_addr,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    static PRODUCT_NAME: &str = "Pixpel Market-Place";

    let add_params = CreateParams {
        product: Product {
            name: PRODUCT_NAME.to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5),
            allocated_tokens: TokenAmount(10000),
            cis2_contract: cis2_addr,
        },
        timeperiod: TimePeriod {
            start: Timestamp::from_timestamp_millis(0),
            end: Timestamp::from_timestamp_millis(3000),
        },
        soft_cap: Amount::from_ccd(5 * 5000),
        hard_cap: Some(Amount::from_ccd(5 * 7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 3,
            release_cycles: 3,
        },
        liquidity_details: LiquidityDetails {
            liquidity_allocation: 40,
            release_cycles: 3,
        },
    };

    add_launch_pad(&mut chain, launch_pad_addr, OWNER, add_params).unwrap();

    let launch_pad_state =
        get_launch_pad(&mut chain, launch_pad_addr, ADMIN, PRODUCT_NAME.to_string());

    println!("{:#?}", launch_pad_state);

    approve_launch_pad(
        &mut chain,
        launch_pad_addr,
        ADMIN,
        ApprovalParams {
            product_name: PRODUCT_NAME.to_string(),
            approve: true,
        },
    )
    .unwrap();

    deposit_tokens_to_launch_pad(
        &mut chain,
        OWNER,
        PRODUCT_NAME.to_string(),
        cis2_addr,
        launch_pad_addr,
    );

    let response = get_token_balance(
        &chain,
        OWNER,
        Address::Contract(launch_pad_addr),
        cis2_addr,
        OWNER_TOKEN_ID,
    );

    println!("{:#?}", response);

    // let state = view_state(&chain, launch_pad_addr, ADMIN);
    // println!("{:#?}", state);
}

#[test]
fn error_codes() {
    ((-43)..=(-1)).for_each(|code| println!("Error::{:?}", LaunchPadError::from(code)));
}

#[derive(Serial, Deserial, SchemaType, Clone, Debug)]
struct TokenInfo {
    pub id: TokenIdVec,
    pub address: ContractAddress,
}

#[derive(Serial, Deserial, SchemaType)]
struct AddLiquidityParams {
    pub token: TokenInfo,
    pub token_amount: TokenAmount,
}

#[test]
fn dex_liqui_smoke() {
    let (mut chain, _, launch_pad_addr, cis2_addr) = initialize_chain_and_launch_pad();

    let dex_contract = initialize_contract::<PhantomData<u8>>(
        &mut chain,
        "../nft-auction/test-build-artifacts/pixpel_swap.wasm.v1".into(),
        "pixpel_swap".into(),
        None,
    );

    mint_token(
        &mut chain,
        OWNER,
        cis2_addr,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let liquidity_params = AddLiquidityParams {
        token: TokenInfo {
            id: TokenIdVec(OWNER_TOKEN_ID.0.to_ne_bytes().into()),
            address: cis2_addr
        },
        token_amount: 10000.into()
    };

    let payload = UpdateContractPayload {
        amount: Amount::from_ccd(10000),
        receive_name: OwnedReceiveName::new_unchecked("pixpel_swap.addLiquidity".to_string()),
        address: dex_contract,
        message: OwnedParameter::from_serial(&liquidity_params)
            .expect("[Error] Unable to serialize UpdateOperator params"),
    };
}