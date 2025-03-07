use core::marker::PhantomData;

use crate::{
    errors::LaunchPadError,
    params::{ApprovalParams, CreateParams, GetExchangeParams, LockupDetails, TokenInfo},
    response::{ExchangeView, LaunchPadView},
    state::{LiquidityDetails, Product, TimePeriod, VestingLimits},
    tests::{
        get_token_balance, initialize_chain_and_launch_pad, initialize_contract, view_state,
        MintParams, PLATFORM_REG_FEE,
    },
};
use concordium_cis2::{
    AdditionalData, OperatorUpdate, Receiver, TokenAmountU64 as TokenAmount, TokenIdU8, TokenIdVec,
    Transfer, TransferParams, UpdateOperator, UpdateOperatorParams,
};
use concordium_smart_contract_testing::UpdateContractPayload;
use concordium_std::{
    Address, Amount, ContractAddress, Deserial, OwnedEntrypointName, OwnedParameter,
    OwnedReceiveName, SchemaType, Serial, Timestamp,
};

use super::{
    mint_token, read_contract, update_contract, ADMIN, OWNER, OWNER_TOKEN_ID, OWNER_TOKEN_URL,
};

#[test]
fn launch_pad_smoke() -> Result<(), LaunchPadError> {
    let (mut chain, _, lp_contract, cis2_contract) = initialize_chain_and_launch_pad();

    let dex_contract = initialize_contract(
        &mut chain,
        "../nft-auction/test-build-artifacts/pixpel_swap.wasm.v1".into(),
        "pixpel_swap".into(),
        (),
    );

    update_contract::<MintParams, ()>(
        &mut chain,
        cis2_contract,
        OWNER,
        (OWNER, OWNER_TOKEN_ID, OWNER_TOKEN_URL.to_string()).into(),
        None,
        "cis2_multi.mint",
    )?;

    static PRODUCT_NAME: &str = "Pixpel Market-Place";

    let add_params = CreateParams {
        product: Product {
            name: PRODUCT_NAME.to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5),
            allocated_tokens: TokenAmount(10000),
            cis2_contract,
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

    update_contract::<_, ()>(
        &mut chain,
        lp_contract,
        OWNER,
        add_params,
        Some(PLATFORM_REG_FEE),
        "LaunchPad.CreateLaunchPad",
    )?;

    update_contract::<_, ()>(
        &mut chain,
        lp_contract,
        ADMIN,
        ApprovalParams {
            product_name: PRODUCT_NAME.to_string(),
            approve: true,
        },
        None,
        "LaunchPad.ApproveLaunchPad",
    )?;

    update_contract::<_, ()>(
        &mut chain,
        cis2_contract,
        OWNER,
        TransferParams::<TokenIdU8, TokenAmount>(vec![Transfer {
            token_id: OWNER_TOKEN_ID,
            amount: TokenAmount(10000),
            from: Address::Account(OWNER),
            to: Receiver::Contract(
                lp_contract,
                OwnedEntrypointName::new_unchecked("Deposit".to_string()),
            ),
            data: AdditionalData::from(PRODUCT_NAME.as_bytes().to_owned()),
        }]),
        None,
        "cis2_multi.transfer",
    )?;

    let response = read_contract::<_, LaunchPadView>(
        &mut chain,
        lp_contract,
        OWNER,
        PRODUCT_NAME.to_string(),
        "LaunchPad.viewLaunchPad",
    );

    println!("{:#?}", response);

    Ok(())
}

#[test]
fn error_codes() {
    ((-43)..=(-1)).for_each(|code| println!("Error::{:?}", LaunchPadError::from(code)));
}

#[derive(Serial, Deserial, SchemaType)]
struct AddLiquidityParams {
    pub token: TokenInfo,
    pub token_amount: TokenAmount,
}

#[test]
fn dex_liquid_smoke() -> Result<(), LaunchPadError> {
    let (mut chain, _, launch_pad_addr, cis2_addr) = initialize_chain_and_launch_pad();

    let dex_contract = initialize_contract(
        &mut chain,
        "../nft-auction/test-build-artifacts/pixpel_swap.wasm.v1".into(),
        "pixpel_swap".into(),
        (),
    );

    mint_token(
        &mut chain,
        OWNER,
        cis2_addr,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let update_operator_params = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Add,
        operator: dex_contract.into(),
    }]);

    update_contract::<_, ()>(
        &mut chain,
        cis2_addr,
        OWNER,
        update_operator_params,
        None,
        "cis2_multi.updateOperator",
    )?;

    let liquidity_params = AddLiquidityParams {
        token: TokenInfo {
            id: TokenIdVec(OWNER_TOKEN_ID.0.to_le_bytes().into()),
            address: cis2_addr,
        },
        token_amount: 10000.into(),
    };

    update_contract::<_, ()>(
        &mut chain,
        dex_contract,
        OWNER,
        liquidity_params,
        Some(Amount::from_ccd(10000)),
        "pixpel_swap.addLiquidity",
    )?;

    let exc_params = GetExchangeParams {
        holder: Address::Account(OWNER),
        token: TokenInfo {
            id: TokenIdVec(OWNER_TOKEN_ID.0.to_le_bytes().into()),
            address: cis2_addr,
        },
    };

    let exc_view = read_contract::<_, ExchangeView>(
        &mut chain,
        dex_contract,
        OWNER,
        exc_params,
        "pixpel_swap.getExchange",
    );

    println!("{:#?}", exc_view);

    Ok(())
}
