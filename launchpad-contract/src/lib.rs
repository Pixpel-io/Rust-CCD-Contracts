#![cfg_attr(not(feature = "std"), no_std)]
use concordium_cis2::{
    AdditionalData, Cis2Client, OnReceivingCis2Params, TokenAmountU64 as TokenAmount, TokenIdU64,
    TokenIdU8 as TokenID, TokenIdVec, Transfer, TransferParams,
};
use concordium_std::{
    bail, ensure, init, receive, Address, Amount, DeserialWithState, Entry, ExternContext,
    ExternReceiveContext, ExternReturnValue, ExternStateApi, Get, HasChainMetadata, HasCommonData,
    HasHost, HasInitContext, HasLogger, HasReceiveContext, HasStateApi, HasStateEntry, Host,
    InitContext, InitResult, Logger, ReceiveContext, Reject, Serial, StateBuilder, UnwrapAbort,
    Write, *,
};
use dex::{DexClient, GetExchangeParams, TokenInfo};
use errors::Error;
use events::{ApproveEvent, CreateLaunchPadEvent, Event, RejectEvent, VestEvent};
use helper::update_operator_of;
use params::{
    ApprovalParams, ClaimLockedParams, ClaimUnLockedParams, Claimer, CreateParams, InitParams,
    LivePauseParams, VestParams,
};
use response::{AllLaunchPads, LaunchPadView, LaunchPadsView, StateView};
use state::{HolderInfo, LaunchPad, Release, State, Status, TimePeriod};

mod dex;
mod errors;
mod events;
mod helper;
mod params;
mod response;
mod state;

#[cfg(test)]
mod tests;

pub type ContractResult<A> = Result<A, Error>;

/// Alias for String as launch pad product name
pub type ProductName = String;

/// Minimum Cliff duration allowed for a product before vesting
/// in milliseconds.
///
/// Min duration for cliff is only 7 days.
const MIN_CLIFF_DURATION: u64 = 6.048e+8 as u64;

/// Minimum Pause duration allowed for a product to be pasued
/// before vesting in milliseconds.
///
/// Min pause duation allowed is 48 hrs
const MIN_PAUSE_DURATION: u64 = 1.728e+8 as u64;

/// Launch-Pad can only be pause at most three times
const MAX_PAUSE_COUNT: u8 = 3;

/// Single release cycle duration in milliseconds
///
/// release duration for each cycle is 1 month.
const CYCLE_DURATION: u64 = 2.678e9 as u64;

/// Alias for OnReceiveCIS2 ook params
type OnReceiveCIS2Params = OnReceivingCis2Params<TokenID, TokenAmount>;

/// Entry point which initializes the contract with new default state.
///
/// The state is empty except that the user must provide admin parameters
/// to be set while initialization.
#[init(contract = "LaunchPad", parameter = "InitParams")]
fn init(ctx: &InitContext, state_builder: &mut StateBuilder) -> InitResult<State> {
    // Getting the init params, which actually wraps
    // around admin information inside
    let param: InitParams = ctx.parameter_cursor().get()?;

    // Creating the default state with provided admin
    // information
    Ok(State {
        launchpads: state_builder.new_map(),
        investors: state_builder.new_map(),
        admin: param.admin,
        counter: 0,
    })
}

#[receive(
    contract = "LaunchPad",
    name = "CreateLaunchPad",
    mutable,
    parameter = "CreateParams",
    error = "Error",
    enable_logger,
    payable
)]
fn create_launchpad(
    ctx: &ReceiveContext,
    host: &mut Host<State>,
    amount: Amount,
    logger: &mut Logger,
) -> ContractResult<()> {
    // Ensure that the sender is an account.
    ensure!(ctx.sender().is_account(), Error::OnlyAccount);

    // parse the parameter
    let params: CreateParams = ctx.parameter_cursor().get()?;

    // Esnure that user pays the complete registeration Fee
    ensure!(
        amount >= host.state().admin_registeration_fee(),
        Error::Insufficient
    );

    // Ensure hard-cap is greater than the soft-cap
    if let Some(hard_cap) = params.hard_cap {
        ensure!(hard_cap > params.soft_cap, Error::Insufficient)
    }

    let time_now = ctx.metadata().block_time();

    // Ensure that the launch-pad active time period is valid
    params.timeperiod.ensure_is_period_valid(time_now)?;

    // Ensure that the provided cliff time period is valid.
    // Cliff is consdiered only if it starts after the vesting
    // and has minimum duration of 7 days
    ensure!(
        params.cliff().millis() > MIN_CLIFF_DURATION,
        Error::InCorrect
    );

    // Creating the Launch-pad from user defined params and
    // getting the launch-pad ID
    let (name, launch_pad) = LaunchPad::from_create_params(params, &mut host.state_builder);

    // Updating the contract State with new launchpad entry
    match host.state_mut().launchpads.entry(name) {
        // If the launch-pad with the same product name exists
        // it will not allow the launch-pad to be inserted
        Entry::Occupied(_) => {
            bail!(Error::Taken)
        }
        // Or else it will insert the launch-pad in State and
        // dispatch the launch pad creation event
        Entry::Vacant(entry) => {
            logger.log(&Event::CREATED(CreateLaunchPadEvent {
                launchpad_name: launch_pad.product_name(),
                owner: launch_pad.get_product_owner(),
                allocated_tokens: launch_pad.get_product_token_amount(),
                base_price: launch_pad.product_base_price(),
            }))?;

            entry.insert(launch_pad);
        }
    };

    // Incrementing the counter to track total launchpads
    host.state_mut().counter += 1;

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "ApproveLaunchPad",
    mutable,
    parameter = "ApprovalParams",
    error = "Error",
    enable_logger
)]
fn approve_launchpad(
    ctx: &ReceiveContext,
    host: &mut Host<State>,
    logger: &mut Logger,
) -> ContractResult<()> {
    // Ensure that the sender is an account.
    ensure!(ctx.sender().is_account(), Error::OnlyAccount);

    // Only admin is allowed to approve launch-pad for presale
    ensure!(
        ctx.sender() == host.state().admin_address().into(),
        Error::UnAuthorized
    );

    // Product name is passed as parameter to identify the
    // corresponding Launch-pad
    let params: ApprovalParams = ctx.parameter_cursor().get()?;

    // Getting the launch-pad to be approved and updating its
    // status to LIVE
    let mut launch_pad = host.state_mut().get_mut_launchpad(params.product_name)?;

    let transfer_to = if params.approve {
        // Updating the launch-pad status to approved
        launch_pad.status = Status::APPROVED;

        logger.log(&Event::APPROVED(ApproveEvent {
            launchpad_name: launch_pad.product_name(),
        }))?;

        drop(launch_pad);

        host.state().admin_address()
    } else {
        // Updating the launch-pad status to rejected if analyst
        // has rejected the launchpad
        launch_pad.status = Status::REJECTED;

        logger.log(&Event::REJECTED(RejectEvent {
            launchpad_name: launch_pad.product_name(),
        }))?;

        let owner = launch_pad.get_product_owner();
        drop(launch_pad);

        owner
    };

    // Refunding the product owner in case if the launch-pad
    // is rejected
    host.invoke_transfer(&transfer_to, host.state().admin_registeration_fee())?;

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "Deposit",
    mutable,
    parameter = "OnReceivingCis2Params<TokenID, TokenAmount>",
    error = "Error",
    enable_logger
)]
fn deposit_tokens(
    ctx: &ReceiveContext,
    host: &mut Host<State>,
    logger: &mut Logger,
) -> ContractResult<()> {
    // This entry point is only meant to be invoked by CIS2 contract
    // given by the product owner in launch-pad params
    let contract = match ctx.sender() {
        Address::Account(_) => bail!(Error::OnlyContract),
        Address::Contract(cis2_contract) => cis2_contract,
    };

    // Parsing the parameters caught by OnReceive hook,
    // We expect to receive additional data as the product
    // name string type in the params
    let OnReceiveCIS2Params {
        token_id,
        amount,
        from,
        data,
    } = ctx.parameter_cursor().get()?;

    let product_name = String::from_utf8(data.as_ref().to_owned()).unwrap();

    // Fetching the launch-pad from the state if the correct
    // product name is supplied
    let mut launch_pad = host.state_mut().get_mut_launchpad(product_name)?;

    // Making sure that the deposit is made by the product
    // owner
    ensure!(
        from == Address::Account(launch_pad.get_product_owner()),
        Error::UnAuthorized
    );

    // Ensure that the correct CIS2 contract invoked the
    // deposit entry point using OnReceive hook
    ensure!(
        contract == launch_pad.get_cis2_contract(),
        Error::UnAuthorized
    );

    // Ensure other details, such as the correct token amount
    // is received or we have received the correct tokens by
    // matching the token ID given in launch-pad params
    ensure!(
        amount == launch_pad.get_product_token_amount(),
        Error::InCorrect
    );
    ensure!(
        token_id == launch_pad.get_product_token_id(),
        Error::InCorrect
    );

    // If every claim is valid, Launch-pad is made LIVE for presale
    // for the current product
    launch_pad.status = Status::LIVE;

    // Dispatching the event as notification when the vesting start
    // as soon as the allocated tokens are deposited
    logger.log(&Event::VESTINGSTARTED(VestEvent {
        launchpad_name: launch_pad.product_name(),
        vesting_time: launch_pad.timeperiod,
        vesting_limits: launch_pad.vest_limits.clone(),
    }))?;

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "LivePause",
    mutable,
    parameter = "LivePauseParams",
    error = "Error"
)]
fn live_pause(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    ensure!(ctx.sender().is_account(), Error::OnlyAccount);

    // Reading parameters
    let params: LivePauseParams = ctx.parameter_cursor().get()?;

    // Getting the launch pad from state identified by the product name
    let mut launch_pad = host.state_mut().get_mut_launchpad(params.poduct_name)?;

    // Product owner (developer) is only allowed to pause
    // the launch pad
    ensure!(
        ctx.sender()
            .matches_account(&launch_pad.get_product_owner()),
        Error::UnAuthorized
    );

    // Launch pad can only be pause during vesting or before
    // reaching the soft cap
    ensure!(
        !launch_pad.reached_soft_cap() && !launch_pad.is_finished(ctx),
        Error::JobFailed
    );

    // Check if owner wants to pause the launch pad
    if params.to_pause {
        // Check if the launch pad is already paused
        ensure!(launch_pad.is_live(), Error::JobFailed);
        // Check if the pause limit reached, launch pad is allowed
        // to be paused 3 times
        ensure!(
            launch_pad.current_pause_count() < MAX_PAUSE_COUNT,
            Error::Limit
        );
        // Check if the pause duration given is not less than the
        // minimum allowed pause duration 48 hrs
        ensure!(
            params.pause_duration.duration_as_millis() >= MIN_PAUSE_DURATION,
            Error::Limit
        );

        // Pausing the launch pad
        launch_pad.status = Status::PAUSED;
        // Setting new pause details in launch pad
        launch_pad.pause.timeperiod = params.pause_duration;
        launch_pad.pause.count += 1;

        return Ok(());
    }

    // Whether the launch-pad is already live
    ensure!(launch_pad.is_paused(), Error::JobFailed);
    // Check if the time is still left for pause duration
    // to complete
    ensure!(
        launch_pad.is_pause_elapsed(ctx.metadata().block_time()),
        Error::NotElapsed
    );

    // Resuming the launch pad
    launch_pad.status = Status::LIVE;
    // Resetting the pause durations
    launch_pad.pause.timeperiod = TimePeriod::default();

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "Vest",
    mutable,
    parameter = "VestParams",
    error = "Error",
    payable
)]
fn vest(ctx: &ReceiveContext, host: &mut Host<State>, amount: Amount) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    let holder = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };

    // Reading parameters
    let params: VestParams = ctx.parameter_cursor().get()?;

    // Getting the contract's core state and its builder
    let (state, state_builder) = host.state_and_builder();

    // Getting the launch pad from state identified by the product name
    let mut launch_pad = state.get_mut_launchpad(params.product_name)?;

    // Make sure that the launch pad is not paused, is not canceled
    // or is not finished, either due to vesting duration elapsed or
    // due to hard cap limit reached
    ensure!(
        !launch_pad.is_paused() && !launch_pad.is_canceled() && !launch_pad.is_finished(ctx),
        Error::JobFailed
    );

    // Verify whether the payable vesting amount received is within the
    // min and max vesting allowed
    ensure!(
        params.token_amount >= launch_pad.vest_min()
            && params.token_amount <= launch_pad.vest_max(),
        Error::Insufficient
    );

    let vest_max = launch_pad.vest_max();

    // Updating or inserting the holder(investor) depending whether the
    // holder is new or existing in the launch pad state
    match launch_pad.holders.entry(holder) {
        // If holder is new to the launch pad, insert him to
        // the holders list along with his invested amount
        // and claimable tokens
        Entry::Vacant(entry) => {
            entry.insert(HolderInfo {
                tokens: params.token_amount,
                invested: amount,
                release_data: Release {
                    unlocked: state_builder.new_map(),
                    locked: state_builder.new_map(),
                },
            });
        }
        // If holder already exist in the launch pad, then
        // just update it's previous amount and claimable
        // tokens.
        Entry::Occupied(mut entry) => {
            let _ = entry.modify(|holder_info| {
                // Ensure that holder does not exceeds the max vesting
                // limit allowed
                ensure!(
                    holder_info.tokens + params.token_amount < vest_max,
                    Error::Limit
                );
                holder_info.invested += amount;
                holder_info.tokens += params.token_amount;
                Ok(())
            });
        }
    }

    // Updating the collected investment and allocated tokens sold so far
    // by the product
    launch_pad.collected += amount;
    launch_pad.sold_tokens += params.token_amount;
    launch_pad.available_tokens -= params.token_amount;

    // Get the amount of tokens allocated for presale by the owner
    let allocated_tokens = launch_pad.product.allocated_tokens;
    // Check if the product has acheived soft cap
    let reached_soft_cap = launch_pad.reached_soft_cap();
    // Check if the product has paid the soft cap share to the platform
    let allocation_paid = launch_pad.allocation_paid;

    let product_name = launch_pad.product_name();

    drop(launch_pad);

    // This is where the allocation share is transfered to the platform admin.
    // Allocation share is paid only once, if the product has just reached the
    // soft cap and the share is not yet paid.
    //
    // Allocation share is paid in terms of perecentile amount of tokens from the
    // product ICO (initial coin offering)
    if reached_soft_cap && !allocation_paid {
        let allocated_cut =
            ((allocated_tokens.0 * host.state().admin_allocation_share()) / 100).into();
        let admin_address = host.state().admin_address();
        let mut launchpad = host.state_mut().get_mut_launchpad(product_name.clone())?;
        let token_id = launchpad.get_product_token_id();

        let cis2_client = Cis2Client::new(launchpad.get_cis2_contract());

        launchpad.allocation_paid = true;
        launchpad.available_tokens -= allocated_cut;

        drop(launchpad);

        // Transfering the calculated amount of product tokens
        // as allocated cut based on the allocation share percent
        // to the platform admin.
        cis2_client.transfer(
            host,
            Transfer {
                token_id,
                amount: allocated_cut,
                from: ctx.self_address().into(),
                to: admin_address.into(),
                data: AdditionalData::empty(),
            },
        )?;
    }

    // Contract's core State maintains the list of all the holders(invesotrs)
    // from every launch pad with their associated launch pads in which they
    // are contributing. There may be more than one launch pad for a single
    // holder.
    match host.state_mut().investors.entry(holder) {
        // Insert the new holder to the state with launch pad ID
        Entry::Vacant(entry) => {
            entry.insert(vec![product_name]);
        }
        // Update the existing holder in the state with launch pad ID
        Entry::Occupied(mut entry) => {
            entry.modify(|launchpads| {
                if !launchpads.contains(&product_name) {
                    launchpads.push(product_name);
                }
            });
        }
    }

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "ClaimTokens",
    mutable,
    parameter = "ClaimUnLockedParams",
    error = "Error"
)]
fn claim_tokens(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    let holder = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };

    // Reading the product name to identify the launch pad
    let params: ClaimUnLockedParams = ctx.parameter_cursor().get()?;

    // Getting the launch pad from state identified by the product name
    let launch_pad = host.state().get_launchpad(params.product_name.clone())?;

    // Make sure that the launch pad is not paused, is not canceled
    // or is finished. As well as the cliff duration has elapsed
    ensure!(
        !launch_pad.is_canceled() && launch_pad.is_finished(ctx),
        Error::JobFailed
    );
    // ensure!(!launch_pad.is_canceled(), Error::Canceled);
    // ensure!(launch_pad.is_finished(ctx), Error::Vesting);

    if let Some(cycle_details) = launch_pad
        .get_holder_info(holder)?
        .release_data
        .unlocked
        .get(&params.cycle)
    {
        let (token_amount, timestamp, claimed) = *cycle_details;

        // Ensuring that this cycle is not already claimed and cycle
        // duration of 1 month is passed since the last cycle.
        ensure!(!claimed, Error::Claimed);
        ensure!(ctx.metadata().block_time() >= timestamp, Error::NotElapsed);

        let cis2_contract = launch_pad.get_cis2_contract();
        let token_id = launch_pad.get_product_token_id();

        // Updating the information regarding the current release cycle
        // and changing its claimed status to true
        host.state_mut()
            .get_mut_launchpad(params.product_name)?
            .set_holder_unlocked_release_info(holder, params.cycle, true);

        // Here are the allocated tokens transfered to the holder based on
        // the current release cycle count.
        Cis2Client::new(cis2_contract).transfer(
            host,
            Transfer {
                token_id,
                amount: token_amount,
                from: ctx.self_address().into(),
                to: holder.into(),
                data: AdditionalData::empty(),
            },
        )?;

        return Ok(());
    }

    // Return early with error if the cycle number supplied in
    // claim params does not exist.
    Err(Error::InCorrect)
}

#[receive(
    contract = "LaunchPad",
    name = "WithdrawFunds",
    mutable,
    parameter = "String",
    error = "Error"
)]
fn withdraw_raised(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    let owner = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };

    // Reading the product name to identify the launch pad
    let product_name: ProductName = ctx.parameter_cursor().get()?;

    // Getting the launch pad from state identified by the product name
    let launch_pad = host.state().get_launchpad(product_name.clone())?;

    // Make sure that the transaction is authorized
    ensure!(owner == launch_pad.get_product_owner(), Error::UnAuthorized);

    // Make sure that the launch pad is not paused, is not canceled
    // or is finished.
    ensure!(
        !launch_pad.is_canceled() && !launch_pad.is_completed() && launch_pad.is_finished(ctx),
        Error::JobFailed
    );

    // Owner can only withdraw collected funds if and only if
    // the product has acheived soft cap and the funds are not
    // already raised
    if !launch_pad.withdrawn && launch_pad.reached_soft_cap() {
        ensure!(launch_pad.reached_soft_cap(), Error::SoftCap);

        // Calculating the amount of funds in CCD to be locked
        // in liquidity according to the percentage provided by
        // the owner
        let ccd_lp_alloc = Amount::from_micro_ccd(
            (launch_pad.collected.micro_ccd * launch_pad.liquidity_details.liquidity_allocation)
                / 100,
        );

        // Tokens from the ICO of product will also be locked in liquidity
        // and the amount of tokens will be designated reflected by the base
        // price of the token in ccd and the amount of CCD being locked.
        let tokens_for_lp = ccd_lp_alloc.micro_ccd / launch_pad.product_base_price().micro_ccd;

        // Remaining amount in CCD that can be withdrawn after the liquidity
        // allocation
        let withdrawable = launch_pad.collected - ccd_lp_alloc;

        let token_id = launch_pad.get_product_token_id();
        let cis2_contract = launch_pad.get_cis2_contract();
        let dex_contract = host.state().dex_address();
        let raised_funds_ccd = launch_pad.collected;
        let liquidity_details = launch_pad.liquidity_details.clone();
        let product_sold_tokens = launch_pad.sold_tokens;
        let lock_up_release_cycles = launch_pad.lock_up.release_cycles;

        // Making DEX as an operator of Launch pad in CIS2 contract
        update_operator_of(host, cis2_contract, dex_contract.into())?;

        // Ensure that DEX has been added as the oprators
        let response = Cis2Client::new(cis2_contract).operator_of(
            host,
            ctx.self_address().into(),
            host.state().dex_address().into(),
        )?;
        ensure!(response, Error::JobFailed);

        // Adding the liquidity to the Platform's DEX and invoking
        DexClient::new(dex_contract).add_liquidity(
            host,
            token_id,
            tokens_for_lp.into(),
            ccd_lp_alloc,
            cis2_contract,
        )?;

        let exchange = DexClient::new(dex_contract).get_exchange(
            host,
            &GetExchangeParams {
                holder: Address::Contract(ctx.self_address()),
                token: TokenInfo {
                    id: TokenIdVec(token_id.0.to_ne_bytes().into()),
                    address: cis2_contract,
                },
            },
        )?;

        // Platform will charge a certain amount from allocated liquidity
        // in exchange of DEX services it provides to the product.
        // Amount that is charged will be according to the launch pad policies
        // and it will be charge from the received LPTokens.
        let platform_lp_share =
            (exchange.lp_tokens_supply * host.state().admin_liquidity_share()).0 / 100;

        // Calculating the remaining LPTokens after platform's cut from the
        // received LPTokens.
        // Allocated LPTokens are divided in half because, equally half of the
        // LPTokens dividend belongs to the product owner and the other half
        // is distributed among the holders in accordance with their percentage
        // contribution in the product's launch pad.
        // This is all aligned with the platform's policies to prevent rug-pull
        // as much as possible.
        let lp_allocated: TokenAmount =
            ((exchange.lp_tokens_supply.0 - platform_lp_share) / 2).into();

        // Transfering the DEX service charges to the platform as the LPTokens.
        DexClient::new(host.state().dex_address()).transfer(
            host,
            TransferParams::<TokenIdU64, TokenAmount>(vec![Transfer {
                token_id: exchange.lp_token_id,
                amount: platform_lp_share.into(),
                from: ctx.self_address().into(),
                to: concordium_cis2::Receiver::Account(host.state().admin_address()),
                data: AdditionalData::empty(),
            }]),
        )?;

        // Transfering the withdrawable amount to the owner in CCD
        host.invoke_transfer(&owner, withdrawable)?;
        // Set the withdrawn flag in launchpad state
        host.state_mut()
            .get_mut_launchpad(product_name.clone())
            .unwrap()
            .withdrawn = true;

        // Updating each holder's information regarding the locked release
        // cycles. LPTokens will be linearly released over the number of
        // months provided by the product owner.
        // Amount of LPTokens, being released in each cycle for any holder,
        // solely depends on its percentage contribution to the ICO.
        for (_, mut holder_info) in host
            .state_mut()
            .get_mut_launchpad(product_name.clone())
            .unwrap()
            .get_holders_mut()
        {
            let holder_contribution =
                (holder_info.invested.micro_ccd * 100) / raised_funds_ccd.micro_ccd;

            let holder_lpts = (lp_allocated * holder_contribution).0 / 100;

            let holder_ico_tokens =
                ((product_sold_tokens - tokens_for_lp.into()) * holder_contribution).0 / 100;

            for i in 0..lock_up_release_cycles {
                let cycle_count = i + 1;
                let tokens_release_amount = (holder_ico_tokens / lock_up_release_cycles).into();
                let timestamp =
                    ((ctx.metadata().block_time().millis + CYCLE_DURATION) * cycle_count).into();

                holder_info.insert_unlocked_cycle(
                    cycle_count as u8,
                    tokens_release_amount,
                    timestamp,
                );
            }

            for i in 0..liquidity_details.release_cycles {
                let lpt_amount = (holder_lpts / liquidity_details.release_cycles).into();
                let cycle_count = i + 1;
                let timestamp =
                    ((ctx.metadata().block_time().millis + CYCLE_DURATION) * cycle_count).into();

                holder_info.insert_locked_cycle(
                    cycle_count as u8,
                    lpt_amount,
                    exchange.lp_token_id,
                    timestamp,
                );
            }
        }

        let mut launch_pad = host.state_mut().get_mut_launchpad(product_name).unwrap();

        // Pre-computing the release cycle information for the product owner
        // locked funds release (LPTokens). For product owner, locked funds
        // are released over a year from now in 3 cycles which are equally
        // separated by 4 months interval.
        // This is all aligned with the platform's policies to prevent rug-pull
        // as much as possible.
        for i in 0..3 {
            let cycle_count = i + 1;
            let lp_amount: TokenAmount = (lp_allocated.0 / 3).into();

            let _ = launch_pad.locked_release.insert(
                cycle_count as u8,
                (
                    lp_amount,
                    exchange.lp_token_id,
                    ((ctx.metadata().block_time().millis + CYCLE_DURATION * 4) * cycle_count)
                        .into(),
                    false,
                ),
            );
        }

        return Ok(());
    }

    Err(Error::Claimed)
}

#[receive(
    contract = "LaunchPad",
    name = "WithDrawLockedFunds",
    mutable,
    parameter = "ClaimLockedParams",
    error = "Error"
)]
fn withdraw_locked_funds(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    let sender = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };

    let claim_params: ClaimLockedParams = ctx.parameter_cursor().get()?;

    let (tokend_id, token_amount) = match claim_params.claimer {
        Claimer::OWNER(cycle) => {
            let launch_pad = host
                .state()
                .get_launchpad(claim_params.product_name.clone())?;

            ensure!(
                sender == launch_pad.get_product_owner(),
                Error::UnAuthorized
            );

            if let Some(cycle_details) = launch_pad.locked_release.get(&cycle) {
                let (token_amount, lp_token_id, timestamp, claimed) = *cycle_details;

                ensure!(!claimed, Error::Claimed);
                ensure!(ctx.metadata().block_time() >= timestamp, Error::NotElapsed);

                host.state_mut()
                    .get_mut_launchpad(claim_params.product_name)?
                    .set_locked_release_info(cycle, true);

                (lp_token_id, token_amount)
            } else {
                return Err(Error::InCorrect);
            }
        }
        Claimer::HOLDER(cycle) => {
            if let Some(locked_cycle_details) = host
                .state()
                .get_launchpad(claim_params.product_name.clone())?
                .get_holder_info(sender)?
                .release_data
                .locked
                .get(&cycle)
            {
                let (token_amount, lp_token_id, timestamp, claimed) = *locked_cycle_details;

                ensure!(!claimed, Error::Claimed);
                ensure!(ctx.metadata().block_time() >= timestamp, Error::NotElapsed);

                host.state_mut()
                    .get_mut_launchpad(claim_params.product_name)?
                    .set_holder_locked_release_info(sender, cycle, true);

                (lp_token_id, token_amount)
            } else {
                return Err(Error::InCorrect);
            }
        }
    };

    DexClient::new(host.state().dex_address()).transfer(
        host,
        TransferParams(vec![Transfer {
            token_id: tokend_id,
            amount: token_amount,
            from: ctx.self_address().into(),
            to: concordium_cis2::Receiver::Account(sender),
            data: AdditionalData::empty(),
        }]),
    )?;
    // let result = host.invoke_contract(
    //     &host.state().dex_address(),
    //     &TransferParams::<TokenIdU64, TokenAmount>(vec![Transfer {
    //         token_id: tokend_id,
    //         amount: token_amount,
    //         from: ctx.self_address().into(),
    //         to: concordium_cis2::Receiver::Account(sender),
    //         data: AdditionalData::empty(),
    //     }]),
    //     EntrypointName::new_unchecked("transfer"),
    //     Amount::zero(),
    // );

    // match result {
    //     Ok(_) => Ok(()),
    //     Err(err) => {

    //     }
    // }

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "CancelLaunchPad",
    mutable,
    parameter = "String",
    error = "Error"
)]
fn cancel(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    let owner = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };

    // Reading the product name to identify the launch pad
    let product_name: ProductName = ctx.parameter_cursor().get()?;

    // Getting the launch pad from state identified by the product name
    let launch_pad = host.state().get_launchpad(product_name.clone())?;

    // Make sure that the transaction is authorized
    ensure!(owner == launch_pad.get_product_owner(), Error::UnAuthorized);

    // Make sure that the launch pad is not already canceled, did not
    //reach the soft cap, launch pad is not completed.
    ensure!(
        !launch_pad.is_canceled() && !launch_pad.is_completed() && !launch_pad.reached_soft_cap(),
        Error::JobFailed
    );

    host.state_mut()
        .get_mut_launchpad(product_name)
        .unwrap()
        .status = Status::CANCELED;

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "viewState",
    return_value = "StateView",
    error = "Error"
)]
fn view_state(_: &ReceiveContext, host: &Host<State>) -> ContractResult<StateView> {
    let state = host.state();

    let state_view = StateView {
        launch_pads: state.launchpads.iter().map(|(_, lp)| lp.into()).collect(),
        investors: state
            .investors
            .iter()
            .map(|(inv, lps)| (*inv, lps.clone()))
            .collect(),
        admin_info: state.admin.clone(),
        total_launch_pads: state.counter,
    };

    Ok(state_view)
}

#[receive(
    contract = "LaunchPad",
    name = "viewAllLaunchPads",
    return_value = "AllLaunchPads",
    error = "Error"
)]
fn view_all_launch_pads(_: &ReceiveContext, host: &Host<State>) -> ContractResult<AllLaunchPads> {
    Ok(AllLaunchPads {
        total_launch_pads: host.state().counter,
        launch_pads: host
            .state()
            .launchpads
            .iter()
            .map(|(_, launch_pad)| launch_pad.into())
            .collect(),
    })
}

#[receive(
    contract = "LaunchPad",
    name = "viewLaunchPad",
    parameter = "String",
    return_value = "LaunchPadView",
    error = "Error"
)]
fn view_launch_pad(ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<LaunchPadView> {
    let product_name: ProductName = ctx.parameter_cursor().get()?;
    let inner_state = host.state().get_launchpad(product_name)?;

    Ok(inner_state.into())
}

#[receive(
    contract = "LaunchPad",
    name = "viewMyLaunchPads",
    return_value = "LaunchPadsView",
    error = "Error"
)]
fn view_my_launch_pads(ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<LaunchPadsView> {
    let holder = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(Error::OnlyAccount),
    };
    // Gets all the launch pads IDs in which this holder is contributing
    let ids = host.state().my_launch_pads(holder)?;

    // Creating the Return view for each launch pad for this holder and
    // returning the serialized result
    Ok(ids
        .iter()
        .map(|id| host.state().get_launchpad(id.clone()).unwrap().into())
        .collect())
}
