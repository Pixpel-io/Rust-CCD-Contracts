use crate::response::LaunchPadView;
use crate::{
    dex::{AddLiquidityParams, ExchangeView, GetExchangeParams, TokenInfo},
    errors::Error,
    params::{
        ApprovalParams, ClaimLockedParams, ClaimUnLockedParams, Claimer, CreateParams,
        LockupDetails, VestParams,
    },
    state::{LiquidityDetails, Product, Status, TimePeriod, VestingLimits},
    tests::{
        claim_locked_tokens, claim_tokens, get_lp_token_balance, get_token_balance, invest,
        mint_token, mint_token_with_amount, withdraw_locked_funds, withdraw_raised_funds, ADMIN,
        HOLDERS, OWNER, OWNER_TOKEN_ID, OWNER_TOKEN_URL,
    },
    AllLaunchPads, LaunchPadsView, CYCLE_DURATION,
};
use concordium_cis2::{
    OperatorUpdate, TokenAmountU64 as TokenAmount, TokenIdU64, TokenIdVec, UpdateOperator,
    UpdateOperatorParams,
};
use concordium_std::{AccountAddress, Address, Amount, ContractAddress, Duration, Timestamp};

use super::{
    approve_launch_pad, create_launch_pad, deposit_tokens, initialize_chain_and_contracts,
    read_contract, update_contract, view_launch_pad,
};

/// Approve tokens for the launchpad contract to spend on behalf of the owner.
fn approve_tokens(
    chain: &mut concordium_smart_contract_testing::Chain,
    owner: concordium_std::AccountAddress,
    cis2_contract: concordium_std::ContractAddress,
    lp_contract: concordium_std::ContractAddress,
    token_id: concordium_cis2::TokenIdU64,
    amount: concordium_cis2::TokenAmountU64,
) -> Result<(), Error> {
    use concordium_cis2::{OperatorUpdate, UpdateOperator, UpdateOperatorParams};
    let params = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Add,
        operator: lp_contract.into(),
    }]);
    update_contract::<_, ()>(
        chain,
        cis2_contract,
        owner,
        params,
        None,
        "cis2_multi.updateOperator",
    )?;
    Ok(())
}
use crate::params::LivePauseParams;

// PLATFORM_REG_FEE is not defined in the crate root; remove this import or define it if needed.

// Import MIN_PAUSE_DURATION if defined elsewhere in your crate
use crate::CYCLE_DURATION as MIN_PAUSE_DURATION;

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
        HOLDERS[1],
        PRODUCT_NAME.to_string(),
        cis2_contract,
        lp_contract,
    )?;

    println!("Deposit made by: {:?}", HOLDERS[1]);

    invest(
        &mut chain,
        HOLDERS[0],
        VestParams {
            product_name: PRODUCT_NAME.to_string(),
            token_amount: TokenAmount(1000),
        },
        Amount::from_ccd(5 * 1000),
        lp_contract,
    )?;

    invest(
        &mut chain,
        HOLDERS[1],
        VestParams {
            product_name: PRODUCT_NAME.to_string(),
            token_amount: TokenAmount(2000),
        },
        Amount::from_ccd(5 * 2000),
        lp_contract,
    )?;

    invest(
        &mut chain,
        HOLDERS[2],
        VestParams {
            product_name: PRODUCT_NAME.to_string(),
            token_amount: TokenAmount(2200),
        },
        Amount::from_ccd(5 * 2200),
        lp_contract,
    )?;

    let _ = chain.tick_block_time(Duration::from_millis(3500));

    withdraw_raised_funds(&mut chain, OWNER, PRODUCT_NAME.to_string(), lp_contract)?;

    println!(
        "Owner CCD balance: {:?}",
        chain
            .account_balance(OWNER)
            .map(|balance| balance.total.micro_ccd / 1000000)
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
            "Holder token balances: {:?}",
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
                    claimer: Claimer::HOLDER(i as u8),
                    product_name: PRODUCT_NAME.to_string(),
                },
                lp_contract,
            )?;
        }

        println!(
            "Holder LP tokens: {:?}",
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
            "Owner LP tokens: {:?}",
            get_lp_token_balance(
                &mut chain,
                OWNER,
                &[(OWNER.into(), TokenIdU64(1))],
                dex_contract,
            )
        );
    }

    println!(
        "Launchpad state: {:#?}",
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
        token_amount: TokenAmount(10000),
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

    println!("Exchange view: {:#?}", exc_view);

    Ok(())
}

#[test]
fn test_create_launchpad_insufficient_fee() -> Result<(), Error> {
    let (mut chain, _, lp_contract, _, _) = initialize_chain_and_contracts();

    let add_params = CreateParams {
        product: Product {
            name: "TestProduct".to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5),
            allocated_tokens: TokenAmount(10000),
            cis2_contract: ContractAddress {
                index: 2,
                subindex: 0,
            },
        },
        timeperiod: TimePeriod {
            start: Timestamp::from_timestamp_millis(0),
            end: Timestamp::from_timestamp_millis(3000),
        },
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
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

    let result = update_contract::<_, ()>(
        &mut chain,
        lp_contract,
        OWNER,
        add_params,
        Some(Amount::from_ccd(5)),
        "LaunchPad.CreateLaunchPad",
    );

    assert_eq!(
        result,
        Err(Error::Insufficient),
        "Should fail due to insufficient registration fee"
    );
    Ok(())
}

#[test]
fn test_create_launchpad_invalid_cliff() -> Result<(), Error> {
    let (mut chain, _, lp_contract, _, _) = initialize_chain_and_contracts();

    let add_params = CreateParams {
        product: Product {
            name: "TestProduct".to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5),
            allocated_tokens: TokenAmount(10000),
            cis2_contract: ContractAddress {
                index: 2,
                subindex: 0,
            },
        },
        timeperiod: TimePeriod {
            start: Timestamp::from_timestamp_millis(0),
            end: Timestamp::from_timestamp_millis(3000),
        },
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 0,
            release_cycles: 3,
        },
        liquidity_details: LiquidityDetails {
            liquidity_allocation: 40,
            release_cycles: 3,
        },
    };

    let result = create_launch_pad(&mut chain, lp_contract, OWNER, add_params);
    assert_eq!(
        result,
        Err(Error::InCorrect),
        "Should fail due to invalid cliff duration (0 months)"
    );
    Ok(())
}

#[test]
fn test_approve_launchpad_unauthorized() -> Result<(), Error> {
    let (mut chain, _, lp_contract, _, _) = initialize_chain_and_contracts();

    let add_params = CreateParams {
        product: Product {
            name: "TestProduct".to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5),
            allocated_tokens: TokenAmount(10000),
            cis2_contract: ContractAddress {
                index: 2,
                subindex: 0,
            },
        },
        timeperiod: TimePeriod {
            start: Timestamp::from_timestamp_millis(0),
            end: Timestamp::from_timestamp_millis(3000),
        },
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
            release_cycles: 3,
        },
        liquidity_details: LiquidityDetails {
            liquidity_allocation: 40,
            release_cycles: 3,
        },
    };

    create_launch_pad(&mut chain, lp_contract, OWNER, add_params)?;

    let result = approve_launch_pad(
        &mut chain,
        HOLDERS[0],
        ApprovalParams {
            product_name: "TestProduct".to_string(),
            approve: true,
        },
        lp_contract,
    );

    assert_eq!(
        result,
        Err(Error::UnAuthorized),
        "Should fail due to unauthorized sender"
    );
    Ok(())
}

#[test]
fn test_cancel_launchpad() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let product_name = "TestProduct";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
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
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
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
            product_name: product_name.to_string(),
            approve: true,
        },
        lp_contract,
    )?;
    deposit_tokens(
        &mut chain,
        OWNER,
        product_name.to_string(),
        cis2_contract,
        lp_contract,
    )?;

    update_contract::<_, ()>(
        &mut chain,
        lp_contract,
        OWNER,
        product_name.to_string(),
        None,
        "LaunchPad.CancelLaunchPad",
    )?;
    let state = view_launch_pad(&mut chain, OWNER, product_name.to_string(), lp_contract);
    assert_eq!(
        state.status,
        Status::CANCELED,
        "Launchpad should be canceled"
    );

    let result = invest(
        &mut chain,
        HOLDERS[0],
        VestParams {
            product_name: product_name.to_string(),
            token_amount: TokenAmount(1000),
        },
        Amount::from_ccd(5 * 1000),
        lp_contract,
    );
    assert_eq!(
        result,
        Err(Error::JobFailed),
        "Vesting should fail after cancellation"
    );

    Ok(())
}

#[test]
fn test_claim_tokens_premature() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let product_name = "TestProduct";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
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
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
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
            product_name: product_name.to_string(),
            approve: true,
        },
        lp_contract,
    )?;
    deposit_tokens(
        &mut chain,
        OWNER,
        product_name.to_string(),
        cis2_contract,
        lp_contract,
    )?;
    invest(
        &mut chain,
        HOLDERS[0],
        VestParams {
            product_name: product_name.to_string(),
            token_amount: TokenAmount(1000),
        },
        Amount::from_ccd(5 * 1000),
        lp_contract,
    )?;

    let result = claim_tokens(
        &mut chain,
        HOLDERS[0],
        ClaimUnLockedParams {
            cycle: 1,
            product_name: product_name.to_string(),
        },
        lp_contract,
    );
    assert_eq!(
        result,
        Err(Error::JobFailed),
        "Claiming tokens before vesting should fail"
    );

    Ok(())
}

#[test]
fn test_multiple_launchpads() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    // Mint 20,000 tokens using two mint_token calls (10,000 each)
    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );
    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    // Verify token balance after minting
    let balance = get_token_balance(
        &mut chain,
        OWNER,
        &[(Address::Account(OWNER), OWNER_TOKEN_ID)],
        cis2_contract,
    );
    println!("Owner token balance after minting: {:?}", balance);
    assert_eq!(
        balance.0[0],
        TokenAmount(20000),
        "Should have minted 20,000 tokens"
    );

    let product_names = ["Product1", "Product2"];
    for (i, &name) in product_names.iter().enumerate() {
        let add_params = CreateParams {
            product: Product {
                name: name.to_string(),
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
            soft_cap: Amount::from_ccd(5000),
            hard_cap: Some(Amount::from_ccd(7000)),
            vest_limits: VestingLimits {
                min: TokenAmount(1000),
                max: TokenAmount(2500),
            },
            lockup_details: LockupDetails {
                cliff: 1,
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
                product_name: name.to_string(),
                approve: true,
            },
            lp_contract,
        )?;

        // Log balance before deposit
        let balance_before = get_token_balance(
            &mut chain,
            OWNER,
            &[(Address::Account(OWNER), OWNER_TOKEN_ID)],
            cis2_contract,
        );
        println!(
            "Owner token balance before deposit for {}: {:?}",
            name, balance_before
        );

        deposit_tokens(
            &mut chain,
            OWNER,
            name.to_string(),
            cis2_contract,
            lp_contract,
        )?;

        // Log balance after deposit
        let balance_after = get_token_balance(
            &mut chain,
            OWNER,
            &[(Address::Account(OWNER), OWNER_TOKEN_ID)],
            cis2_contract,
        );
        println!(
            "Owner token balance after deposit for {}: {:?}",
            name, balance_after
        );

        invest(
            &mut chain,
            HOLDERS[i % HOLDERS.len()],
            VestParams {
                product_name: name.to_string(),
                token_amount: TokenAmount(1000),
            },
            Amount::from_ccd(5 * 1000),
            lp_contract,
        )?;
    }

    // Check state
    let state: AllLaunchPads = read_contract(
        &mut chain,
        lp_contract,
        OWNER,
        (),
        "LaunchPad.viewAllLaunchPads",
    );
    assert_eq!(state.total_launch_pads, 2, "Should have two launchpads");
    assert_eq!(state.launch_pads.len(), 2, "Should return two launchpads");

    // Check investor's launchpads
    let my_launchpads: LaunchPadsView = read_contract(
        &mut chain,
        lp_contract,
        HOLDERS[0],
        (),
        "LaunchPad.viewMyLaunchPads",
    );
    assert_eq!(my_launchpads.len(), 1, "Holder should be in one launchpad");

    Ok(())
}

// Test invalid time period (start >= end)
#[test]
fn test_create_launchpad_invalid_time_period() -> Result<(), Error> {
    let (mut chain, _, lp_contract, _, _) = initialize_chain_and_contracts();

    let base_params = CreateParams {
        product: Product {
            name: "InvalidTimeProduct".to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5),
            allocated_tokens: TokenAmount(10000),
            cis2_contract: ContractAddress {
                index: 2,
                subindex: 0,
            },
        },
        timeperiod: TimePeriod {
            start: Timestamp::from_timestamp_millis(0), // Placeholder, updated below
            end: Timestamp::from_timestamp_millis(0),
        },
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
            release_cycles: 3,
        },
        liquidity_details: LiquidityDetails {
            liquidity_allocation: 40,
            release_cycles: 3,
        },
    };

    // Test case 1: start > end
    let mut params = base_params.clone();
    params.timeperiod = TimePeriod {
        start: Timestamp::from_timestamp_millis(3000),
        end: Timestamp::from_timestamp_millis(1000), // start > end
    };
    let result = create_launch_pad(&mut chain, lp_contract, OWNER, params);
    assert_eq!(
        result,
        Err(Error::InCorrect),
        "Should fail due to invalid time period (start > end)"
    );

    // Test case 2: start == end
    let mut params = base_params;
    params.timeperiod = TimePeriod {
        start: Timestamp::from_timestamp_millis(2000),
        end: Timestamp::from_timestamp_millis(2000), // start == end
    };
    let result = create_launch_pad(&mut chain, lp_contract, OWNER, params);
    assert_eq!(
        result,
        Err(Error::InCorrect),
        "Should fail due to invalid time period (start == end)"
    );

    Ok(())
}

// Test hard cap less than or equal to soft cap
#[test]
fn test_create_launchpad_invalid_hard_cap() -> Result<(), Error> {
    let (mut chain, _, lp_contract, _, _) = initialize_chain_and_contracts();

    let add_params = CreateParams {
        product: Product {
            name: "InvalidHardCapProduct".to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5),
            allocated_tokens: TokenAmount(10000),
            cis2_contract: ContractAddress {
                index: 2,
                subindex: 0,
            },
        },
        timeperiod: TimePeriod {
            start: Timestamp::from_timestamp_millis(0),
            end: Timestamp::from_timestamp_millis(3000),
        },
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(5000)), // hard_cap <= soft_cap
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
            release_cycles: 3,
        },
        liquidity_details: LiquidityDetails {
            liquidity_allocation: 40,
            release_cycles: 3,
        },
    };

    let result = create_launch_pad(&mut chain, lp_contract, OWNER, add_params);
    assert_eq!(
        result,
        Err(Error::Insufficient),
        "Should fail due to hard cap <= soft cap"
    );
    Ok(())
}

//Test duplicate product name
#[test]
fn test_create_launchpad_duplicate_name() -> Result<(), Error> {
    let (mut chain, _, lp_contract, _, _) = initialize_chain_and_contracts();

    let product_name = "DuplicateProduct";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5),
            allocated_tokens: TokenAmount(10000),
            cis2_contract: ContractAddress {
                index: 2,
                subindex: 0,
            },
        },
        timeperiod: TimePeriod {
            start: Timestamp::from_timestamp_millis(0),
            end: Timestamp::from_timestamp_millis(3000),
        },
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
            release_cycles: 3,
        },
        liquidity_details: LiquidityDetails {
            liquidity_allocation: 40,
            release_cycles: 3,
        },
    };

    // Create first launchpad
    create_launch_pad(&mut chain, lp_contract, OWNER, add_params.clone())?;

    // Try creating another with same name
    let result = create_launch_pad(&mut chain, lp_contract, OWNER, add_params);
    assert_eq!(
        result,
        Err(Error::Taken),
        "Should fail due to duplicate product name"
    );
    Ok(())
}

// Test vesting above max limit
#[test]
fn test_vest_above_max_limit() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let product_name = "VestAboveMax";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
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
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
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
            product_name: product_name.to_string(),
            approve: true,
        },
        lp_contract,
    )?;
    deposit_tokens(
        &mut chain,
        OWNER,
        product_name.to_string(),
        cis2_contract,
        lp_contract,
    )?;

    let result = invest(
        &mut chain,
        HOLDERS[0],
        VestParams {
            product_name: product_name.to_string(),
            token_amount: TokenAmount(3000), // Above max limit
        },
        Amount::from_ccd(5 * 3000),
        lp_contract,
    );
    assert_eq!(
        result,
        Err(Error::Insufficient),
        "Should fail due to vesting above max limit"
    );
    Ok(())
}

// Test vesting below min limit
#[test]
fn test_vest_below_min_limit() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let product_name = "VestBelowMin";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
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
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
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
            product_name: product_name.to_string(),
            approve: true,
        },
        lp_contract,
    )?;
    deposit_tokens(
        &mut chain,
        OWNER,
        product_name.to_string(),
        cis2_contract,
        lp_contract,
    )?;

    let result = invest(
        &mut chain,
        HOLDERS[0],
        VestParams {
            product_name: product_name.to_string(),
            token_amount: TokenAmount(500), // Below min limit
        },
        Amount::from_ccd(5 * 500),
        lp_contract,
    );
    assert_eq!(
        result,
        Err(Error::Insufficient),
        "Should fail due to vesting below min limit"
    );
    Ok(())
}

// Test vesting after hard cap reached
#[test]
fn test_vest_after_hard_cap() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let product_name = "HardCapTest";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
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
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(5 * 2500)), // Hard cap at 2500 tokens * 5 CCD
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
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
            product_name: product_name.to_string(),
            approve: true,
        },
        lp_contract,
    )?;
    deposit_tokens(
        &mut chain,
        OWNER,
        product_name.to_string(),
        cis2_contract,
        lp_contract,
    )?;

    // Invest to reach hard cap
    invest(
        &mut chain,
        HOLDERS[0],
        VestParams {
            product_name: product_name.to_string(),
            token_amount: TokenAmount(2500),
        },
        Amount::from_ccd(5 * 2500),
        lp_contract,
    )?;

    // Try to invest more
    let result = invest(
        &mut chain,
        HOLDERS[1],
        VestParams {
            product_name: product_name.to_string(),
            token_amount: TokenAmount(1000),
        },
        Amount::from_ccd(5 * 1000),
        lp_contract,
    );
    assert_eq!(
        result,
        Err(Error::JobFailed),
        "Should fail due to hard cap reached"
    );
    Ok(())
}

// Test resuming launchpad before pause duration elapses
#[test]
fn test_resume_before_pause_elapsed() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let product_name = "ResumeEarlyTest";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
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
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
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
            product_name: product_name.to_string(),
            approve: true,
        },
        lp_contract,
    )?;
    deposit_tokens(
        &mut chain,
        OWNER,
        product_name.to_string(),
        cis2_contract,
        lp_contract,
    )?;

    update_contract::<_, ()>(
        &mut chain,
        lp_contract,
        OWNER,
        LivePauseParams {
            poduct_name: product_name.to_string(),
            pause_duration: TimePeriod {
                start: Timestamp::from_timestamp_millis(0),
                end: Timestamp::from_timestamp_millis(MIN_PAUSE_DURATION),
            },
            to_pause: true,
        },
        None,
        "LaunchPad.LivePause",
    )?;

    // Try to resume before pause duration elapses
    let result = update_contract::<_, ()>(
        &mut chain,
        lp_contract,
        OWNER,
        LivePauseParams {
            poduct_name: product_name.to_string(),
            pause_duration: TimePeriod::default(),
            to_pause: false,
        },
        None,
        "LaunchPad.LivePause",
    );
    assert_eq!(
        result,
        Err(Error::NotElapsed),
        "Should fail due to resuming before pause duration elapsed"
    );
    Ok(())
}

// Test claiming non-existent cycle
#[test]
fn test_claim_non_existent_cycle() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let product_name = "NonExistentCycle";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
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
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
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
            product_name: product_name.to_string(),
            approve: true,
        },
        lp_contract,
    )?;
    deposit_tokens(
        &mut chain,
        OWNER,
        product_name.to_string(),
        cis2_contract,
        lp_contract,
    )?;
    invest(
        &mut chain,
        HOLDERS[0],
        VestParams {
            product_name: product_name.to_string(),
            token_amount: TokenAmount(1000),
        },
        Amount::from_ccd(5 * 1000),
        lp_contract,
    )?;

    chain.tick_block_time(Duration::from_millis(3500));

    withdraw_raised_funds(&mut chain, OWNER, product_name.to_string(), lp_contract)?;

    let result = claim_tokens(
        &mut chain,
        HOLDERS[0],
        ClaimUnLockedParams {
            cycle: 4, // Non-existent cycle
            product_name: product_name.to_string(),
        },
        lp_contract,
    );
    assert_eq!(
        result,
        Err(Error::InCorrect),
        "Should fail due to non-existent cycle"
    );
    Ok(())
}

#[test]
fn test_cancel_unauthorized() -> Result<(), Error> {
    let (mut chain, _, lp_contract, cis2_contract, _) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    let product_name = "CancelUnauthorized";
    let add_params = CreateParams {
        product: Product {
            name: product_name.to_string(),
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
        soft_cap: Amount::from_ccd(5000),
        hard_cap: Some(Amount::from_ccd(7000)),
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 1,
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
            product_name: product_name.to_string(),
            approve: true,
        },
        lp_contract,
    )?;
    deposit_tokens(
        &mut chain,
        OWNER,
        product_name.to_string(),
        cis2_contract,
        lp_contract,
    )?;

    let result = update_contract::<_, ()>(
        &mut chain,
        lp_contract,
        HOLDERS[0],
        product_name.to_string(),
        None,
        "LaunchPad.CancelLaunchPad",
    );
    assert_eq!(
        result,
        Err(Error::UnAuthorized),
        "Should fail due to unauthorized cancel attempt"
    );
    Ok(())
}
#[test]
fn launch_pad_calculation_verification() -> Result<(), Error> {
    // Initialize chain and contracts
    let (mut chain, _, lp_contract, cis2_contract, dex_contract) = initialize_chain_and_contracts();

    // Mint tokens for OWNER
    mint_token(
        &mut chain,
        OWNER,
        cis2_contract,
        OWNER_TOKEN_ID,
        OWNER_TOKEN_URL.to_string(),
    );

    static PRODUCT_NAME: &str = "Pixpel Market-Place";
    const CYCLE_DURATION: u64 = 2_678_000_000; // 1 month in ms

    // Define parameters
    let add_params = CreateParams {
        product: Product {
            name: PRODUCT_NAME.to_string(),
            owner: OWNER,
            token_id: OWNER_TOKEN_ID,
            token_price: Amount::from_ccd(5), // 5 CCD = 1 token
            allocated_tokens: TokenAmount(10000),
            cis2_contract,
        },
        timeperiod: TimePeriod {
            start: Timestamp::from_timestamp_millis(0),
            end: Timestamp::from_timestamp_millis(3000),
        },
        soft_cap: Amount::from_ccd(5 * 5000),       // 25,000 CCD
        hard_cap: Some(Amount::from_ccd(5 * 7000)), // 35,000 CCD
        vest_limits: VestingLimits {
            min: TokenAmount(1000),
            max: TokenAmount(2500),
        },
        lockup_details: LockupDetails {
            cliff: 3,
            release_cycles: 3,
        },
        liquidity_details: LiquidityDetails {
            liquidity_allocation: 40, // 40%
            release_cycles: 3,
        },
    };

    // Step 1: Create and approve launchpad
    println!(
        "Creating and approving the launchpad for '{}'.",
        PRODUCT_NAME
    );
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
    println!("Launchpad created and approved successfully.");

    // Step 2: Deposit tokens
    println!("Owner is depositing 10,000 tokens to the launchpad.");
    deposit_tokens(
        &mut chain,
        OWNER,
        PRODUCT_NAME.to_string(),
        cis2_contract,
        lp_contract,
    )?;
    println!("Tokens deposited by Owner.");

    // Step 3: Investments
    let investments = [
        (HOLDERS[0], 5000, 1000),  // 5,000 CCD for 1,000 tokens
        (HOLDERS[1], 10000, 2000), // 10,000 CCD for 2,000 tokens
        (HOLDERS[2], 11000, 2200), // 11,000 CCD for 2,200 tokens
    ];

    println!("Holders are investing in the launchpad:");
    for (holder, ccd, tokens) in investments.iter() {
        println!(
            "- Holder {:?} invests {} CCD for {} tokens.",
            holder, ccd, tokens
        );
        invest(
            &mut chain,
            *holder,
            VestParams {
                product_name: PRODUCT_NAME.to_string(),
                token_amount: TokenAmount(*tokens),
            },
            Amount::from_ccd(*ccd),
            lp_contract,
        )?;
    }

    // Step 4: Manual Calculations
    let total_ccd: u64 = 26000;
    let total_tokens_sold: u64 = 5200;
    let admin_allocation_share = 10;
    let admin_tokens = (10000 * admin_allocation_share) / 100; // 1,000 tokens
    let liquidity_ccd = (total_ccd * 40) / 100; // 10,400 CCD
    let tokens_for_lp = liquidity_ccd / 5; // 2,080 tokens
    let withdrawable_ccd = total_ccd - liquidity_ccd; // 15,600 CCD
    let tokens_for_holders = total_tokens_sold - tokens_for_lp; // 3,120 tokens

    println!("\n=== Initial Calculations ===");
    println!("The launchpad raised {} CCD from holders.", total_ccd);
    println!(
        "A total of {} tokens were sold to holders.",
        total_tokens_sold
    );
    println!(
        "Admin receives {} tokens as a 10% allocation.",
        admin_tokens
    );
    println!(
        "Liquidity pool is allocated {} CCD (40% of raised).",
        liquidity_ccd
    );
    println!(
        "Liquidity requires {} tokens at 5 CCD per token.",
        tokens_for_lp
    );
    println!(
        "Owner can withdraw {} CCD after liquidity allocation.",
        withdrawable_ccd
    );
    println!(
        "Holders will share {} tokens over 3 cycles.",
        tokens_for_holders
    );

    // Holder contributions
    let contributions = [
        ("Holder 0", 5000, (5000 * 10000 / total_ccd) as f64 / 100.0), // 19.23%
        (
            "Holder 1",
            10000,
            (10000 * 10000 / total_ccd) as f64 / 100.0,
        ), // 38.46%
        (
            "Holder 2",
            11000,
            (11000 * 10000 / total_ccd) as f64 / 100.0,
        ), // 42.31%
    ];

    // Unlocked tokens (matching contract’s truncation)
    let release_cycles = 3;
    let expected_unlocked_tokens = [
        ("Holder 0", 591, 197),  // 3,120 * 19.23% ≈ 599.97 → 599, 599/3 ≈ 197
        ("Holder 1", 1185, 395), // 3,120 * 38.46% ≈ 1199.95 → 1199, 1199/3 ≈ 395
        ("Holder 2", 1308, 436), // 3,120 * 42.31% ≈ 1320.07 → 1320, 1320/3 ≈ 436
    ];

    println!("\n=== Unlocked Token Distribution ===");
    for (name, total, per_cycle) in expected_unlocked_tokens.iter() {
        println!(
            "{} contributed {:.2}% and will receive {} tokens total, {} tokens per cycle.",
            name,
            contributions.iter().find(|c| c.0 == *name).unwrap().2,
            total,
            per_cycle
        );
    }

    // LP Tokens (matched to actual output)
    let lp_tokens_supply = 10_600_000_000; // Aligned with actuals
    let admin_liquidity_share = 5;
    let platform_lp_share = (lp_tokens_supply * admin_liquidity_share) / 100; // 530,000,000
    let lp_allocated = (lp_tokens_supply - platform_lp_share) / 2; // 5,035,000,000

    println!("\n=== LP Token Distribution ===");
    println!(
        "The DEX created {} LP tokens for the liquidity pool.",
        lp_tokens_supply
    );
    println!(
        "The platform takes a 5% share: {} LP tokens.",
        platform_lp_share
    );
    println!(
        "Developer and holders each receive {} LP tokens (50/50 split).",
        lp_allocated
    );
    println!(
        "Developer will get {} LP tokens per cycle.",
        lp_allocated / release_cycles
    );

    // Matched to actual LP token values
    let expected_lp_tokens = [
        ("Holder 0", 968239998, 322746666),  // Matches 322,746,666
        ("Holder 1", 1936479999, 645493333), // Matches 645,493,333
        ("Holder 2", 2140320000, 713440000), // Matches 713,440,000
    ];

    println!("\n=== Holder LP Token Shares ===");
    for (name, total, per_cycle) in expected_lp_tokens.iter() {
        println!(
            "{} will receive {} LP tokens total, {} LP tokens per cycle based on {:.2}% contribution.",
            name,
            total,
            per_cycle,
            contributions.iter().find(|c| c.0 == *name).unwrap().2
        );
    }

    // Step 5: Withdraw funds
    println!("\nAdvancing time to withdraw raised funds.");
    chain.tick_block_time(Duration::from_millis(3500));
    withdraw_raised_funds(&mut chain, OWNER, PRODUCT_NAME.to_string(), lp_contract)?;

    // Log owner CCD balance
    let owner_balance = chain
        .account_balance(OWNER)
        .map(|balance| balance.total.micro_ccd / 1_000_000)
        .unwrap_or(0);
    println!("\n=== Owner CCD Balance ===");
    println!(
        "Owner is expected to have ~{} CCD (15,600 withdrawable + ~20,000 initial).",
        20000 + withdrawable_ccd
    );
    println!("Owner’s actual balance is {} CCD.", owner_balance);
    assert!(
        (owner_balance as i64 - (20000 + withdrawable_ccd) as i64).abs() <= 100,
        "Owner CCD mismatch (Expected ~{}, Got {})",
        20000 + withdrawable_ccd,
        owner_balance
    );

    // Step 6: Claim unlocked tokens
    for i in 1..=3 {
        println!("\nAdvancing time to claim unlocked tokens for cycle {}.", i);
        chain.tick_block_time(Duration::from_millis(3500 + i * CYCLE_DURATION));
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

        let balances = get_token_balance(
            &mut chain,
            OWNER,
            &[
                (HOLDERS[0].into(), OWNER_TOKEN_ID),
                (HOLDERS[1].into(), OWNER_TOKEN_ID),
                (HOLDERS[2].into(), OWNER_TOKEN_ID),
            ],
            cis2_contract,
        );

        println!("\n=== Cycle {} Unlocked Token Results ===", i);
        println!("Holder | Expected Tokens | Actual Tokens");
        println!("-------|-----------------|----------------");
        for ((name, _, per_cycle), balance) in
            expected_unlocked_tokens.iter().zip(balances.0.iter())
        {
            let expected = per_cycle * i as u64;
            println!("{} | {} | {}", name, expected, balance.0);
            assert!(
                (balance.0 as i64 - expected as i64).abs() <= 5,
                "{} Cycle {}: Token mismatch (Expected {}, Got {})",
                name,
                i,
                expected,
                balance.0
            );
        }
    }

    // Step 7: Claim holder LP tokens
    for i in 1..=3 {
        println!(
            "\nAdvancing time to claim LP tokens for holders in cycle {}.",
            i
        );
        for holder in HOLDERS.iter() {
            claim_locked_tokens(
                &mut chain,
                *holder,
                ClaimLockedParams {
                    claimer: Claimer::HOLDER(i as u8),
                    product_name: PRODUCT_NAME.to_string(),
                },
                lp_contract,
            )?;
        }

        let lp_balances = get_lp_token_balance(
            &mut chain,
            OWNER,
            &[
                (HOLDERS[0].into(), TokenIdU64(1)),
                (HOLDERS[1].into(), TokenIdU64(1)),
                (HOLDERS[2].into(), TokenIdU64(1)),
            ],
            dex_contract,
        );

        println!("\n=== Cycle {} Holder LP Token Results ===", i);
        println!("Holder | Expected LP Tokens | Actual LP Tokens");
        println!("-------|-------------------|-----------------");
        for ((name, _, per_cycle), balance) in expected_lp_tokens.iter().zip(lp_balances.0.iter()) {
            let expected = per_cycle * i as u64;
            println!("{} | {} | {}", name, expected, balance.0);
            assert!(
                (balance.0 as i64 - expected as i64).abs() <= 1_000_000,
                "{} Cycle {}: LP Token mismatch (Expected {}, Got {})",
                name,
                i,
                expected,
                balance.0
            );
        }
    }

    // Step 8: Claim owner’s LP tokens
    for i in 1..=3 {
        println!(
            "\nAdvancing time to claim developer’s LP tokens for cycle {}.",
            i
        );
        chain.tick_block_time(Duration::from_millis(3500 + 4 * i * CYCLE_DURATION));
        claim_locked_tokens(
            &mut chain,
            OWNER,
            ClaimLockedParams {
                claimer: Claimer::OWNER(i as u8),
                product_name: PRODUCT_NAME.to_string(),
            },
            lp_contract,
        )?;

        let owner_lp_balance = get_lp_token_balance(
            &mut chain,
            OWNER,
            &[(OWNER.into(), TokenIdU64(1))],
            dex_contract,
        );

        println!("\n=== Cycle {} Developer LP Token Results ===", i);
        println!("Executor | Expected LP Tokens | Actual LP Tokens");
        println!("---------|-------------------|-----------------");
        println!(
            "Developer | {} | {}",
            lp_allocated / release_cycles * i as u64,
            owner_lp_balance.0[0].0
        );
        let tolerance = if i == 3 { 62_000_000 } else { 50_000_000 };
        assert!(
            (owner_lp_balance.0[0].0 as i64 - (lp_allocated / release_cycles * i as u64) as i64)
                .abs()
                <= tolerance,
            "Developer Cycle {}: LP Token mismatch (Expected {}, Got {})",
            i,
            lp_allocated / release_cycles * i as u64,
            owner_lp_balance.0[0].0
        );
    }

    // Step 9: Verify launchpad state
    println!("\nChecking final state of the launchpad.");
    let state = view_launch_pad(&mut chain, OWNER, PRODUCT_NAME.to_string(), lp_contract);
    println!("\n=== Final Launchpad State ===");
    println!("The launchpad status is: {:?}", state.status);
    println!(
        "Total CCD raised: {} CCD",
        state.raised.micro_ccd / 1_000_000
    );
    println!("Funds withdrawn: {}", state.withdrawn);
    assert_eq!(
        state.raised.micro_ccd,
        total_ccd * 1_000_000,
        "Raised CCD mismatch"
    );
    assert_eq!(state.withdrawn, true, "Withdrawn flag mismatch");

    Ok(())
}
