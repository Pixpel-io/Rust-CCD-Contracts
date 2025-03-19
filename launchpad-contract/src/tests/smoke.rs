use crate::{
    errors::Error,
    params::{
        AddLiquidityParams, ApprovalParams, ClaimLockedParams, ClaimUnLockedParams, Claimer,
        CreateParams, GetExchangeParams, LockupDetails, TokenInfo, VestParams,
    },
    response::ExchangeView,
    state::{LiquidityDetails, Product, TimePeriod, VestingLimits},
    tests::{
        claim_locked_tokens, claim_tokens, get_lp_token_balance, get_token_balance, invest,
        withdraw_raised_funds, HOLDERS,
    },
    CYCLE_DURATION,
};
use concordium_cis2::{
    OperatorUpdate, TokenAmountU64 as TokenAmount, TokenIdU64, TokenIdVec, UpdateOperator,
    UpdateOperatorParams,
};

use concordium_std::{Address, Amount, Duration, Timestamp};

use super::{
    approve_launch_pad, create_launch_pad, deposit_tokens, initialize_chain_and_contracts,
    mint_token, read_contract, update_contract, view_launch_pad, ADMIN, OWNER, OWNER_TOKEN_ID,
    OWNER_TOKEN_URL,
};

#[test]
fn launch_pad_smoke() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, dex_contract) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
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

    create_launch_pad(&mut chain, lp_contract, OWNER, add_params)?;

    approve_launch_pad(
        &mut chain,
        ADMIN,
        ApprovalParams {
            product_name: PRODUCT_NAME.to_string(),
            approve: true,
        },
        lp_contract,
    )?;

    deposit_tokens(
        &mut chain,
        OWNER,
        PRODUCT_NAME.to_string(),
        cis2_contract,
        lp_contract,
    )?;

    invest(
        &mut chain,
        HOLDERS[0],
        VestParams {
            product_name: PRODUCT_NAME.to_string(),
            token_amount: 1000.into(),
        },
        Amount::from_ccd(5 * 1000),
        lp_contract,
    )?;

    invest(
        &mut chain,
        HOLDERS[1],
        VestParams {
            product_name: PRODUCT_NAME.to_string(),
            token_amount: 2000.into(),
        },
        Amount::from_ccd(5 * 2000),
        lp_contract,
    )?;

    invest(
        &mut chain,
        HOLDERS[2],
        VestParams {
            product_name: PRODUCT_NAME.to_string(),
            token_amount: 2200.into(),
        },
        Amount::from_ccd(5 * 2200),
        lp_contract,
    )?;

    let _ = chain.tick_block_time(Duration::from_millis(3500));

    withdraw_raised_funds(&mut chain, OWNER, PRODUCT_NAME.to_string(), lp_contract)?;

    println!(
        "{:?}",
        chain
            .account_balance(OWNER)
            .and_then(|balance| Some(balance.total.micro_ccd / 1000000))
    );

    for i in 1..=3 {
        let _ = chain.tick_block_time(Duration::from_millis(3500 + i * CYCLE_DURATION));

        for holder in HOLDERS.iter() {
            claim_tokens(
                &mut chain,
                *holder,
                ClaimUnLockedParams {
                    cycle: i as u8,
                    product_name: PRODUCT_NAME.to_string(),
                },
                lp_contract,
            )?;
        }

        println!(
            "{:?}",
            get_token_balance(
                &mut chain,
                OWNER,
                &[
                    (HOLDERS[0].into(), OWNER_TOKEN_ID),
                    (HOLDERS[1].into(), OWNER_TOKEN_ID),
                    (HOLDERS[2].into(), OWNER_TOKEN_ID),
                ],
                cis2_contract,
            )
        );
    }

    for i in 1..=3 {
        for holder in HOLDERS.iter() {
            claim_locked_tokens(
                &mut chain,
                *holder,
                ClaimLockedParams {
                    claimer: Claimer::HOLDER(i),
                    product_name: PRODUCT_NAME.to_string(),
                },
                lp_contract,
            )?;
        }

        println!(
            "{:?}",
            get_lp_token_balance(
                &mut chain,
                OWNER,
                &[
                    (HOLDERS[0].into(), TokenIdU64(1)),
                    (HOLDERS[1].into(), TokenIdU64(1)),
                    (HOLDERS[2].into(), TokenIdU64(1)),
                ],
                dex_contract,
            )
        );
    }

    for i in 1..=3 {
        let _ = chain.tick_block_time(Duration::from_millis(3500 + 4 * i * CYCLE_DURATION));

        claim_locked_tokens(
            &mut chain,
            OWNER,
            ClaimLockedParams {
                claimer: Claimer::OWNER(i as u8),
                product_name: PRODUCT_NAME.to_string(),
            },
            lp_contract,
        )?;

        println!(
            "{:?}",
            get_lp_token_balance(
                &mut chain,
                OWNER,
                &[(OWNER.into(), TokenIdU64(1))],
                dex_contract,
            )
        );
    }

    println!(
        "{:#?}",
        view_launch_pad(&mut chain, OWNER, PRODUCT_NAME.to_string(), lp_contract)
    );

    Ok(())
}

#[test]
fn dex_liquid_smoke() -> Result<(), Error> {
    let (mut chain, _, _, cis2_addr, dex_contract) = initialize_chain_and_contracts();

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
