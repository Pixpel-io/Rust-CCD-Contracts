#![cfg_attr(not(feature = "std"), no_std)]
use concordium_cis2::{
    AdditionalData, Cis2Client, OnReceivingCis2Params, TokenAmountU64 as TokenAmount,
    TokenIdU8 as TokenID, TokenIdVec, Transfer,
};
use concordium_std::{
    bail, ensure, init, receive, Address, Amount, DeserialWithState, Entry, ExternContext,
    ExternReceiveContext, ExternReturnValue, ExternStateApi, Get, HasChainMetadata, HasCommonData,
    HasHost, HasInitContext, HasLogger, HasReceiveContext, HasStateApi, HasStateEntry, Host,
    InitContext, InitResult, Logger, ReceiveContext, Reject, Serial, StateBuilder, UnwrapAbort,
    Write, *,
};
use errors::LaunchPadError;
use events::{ApproveEvent, CreateLaunchPadEvent, Event, RejectEvent, VestEvent};
use params::{
    AddLiquidityParams, ApprovalParams, CreateParams, InitParams, LivePauseParams, TokenInfo,
    VestParams,
};
use response::{AllLaunchPads, LaunchPadView, LaunchPadsView, LPTokenInfo, StateView};
use state::{HolderInfo, LaunchPad, LaunchPadStatus, State, TimePeriod};
// use types::ContractResult;

// mod contract;
mod errors;
mod events;
mod params;
mod response;
mod state;
// mod types;

#[cfg(test)]
mod tests;

pub type ContractResult<A> = Result<A, LaunchPadError>;

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
    error = "LaunchPadError",
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
    ensure!(ctx.sender().is_account(), LaunchPadError::OnlyAccount);

    // parse the parameter
    let params: CreateParams = ctx.parameter_cursor().get()?;

    // Esnure that user pays the complete registeration Fee
    ensure!(
        amount >= host.state().admin_registeration_fee(),
        LaunchPadError::Insufficient
    );

    // Ensure hard-cap is greater than the soft-cap
    if let Some(hard_cap) = params.hard_cap {
        ensure!(hard_cap > params.soft_cap, LaunchPadError::SmallerHardCap)
    }

    let time_now = ctx.metadata().block_time();

    // Ensure that the launch-pad active time period is valid
    params.timeperiod.ensure_is_period_valid(time_now)?;

    // Ensure that the provided cliff time period is valid.
    // Cliff is consdiered only if it starts after the vesting
    // and has minimum duration of 7 days
    ensure!(
        params.cliff().millis() > MIN_CLIFF_DURATION,
        LaunchPadError::InCorrectCliffPeriod
    );

    // Creating the Launch-pad from user defined params and
    // getting the launch-pad ID
    let (name, launch_pad) = LaunchPad::from_create_params(params, &mut host.state_builder);

    // Updating the contract State with new launchpad entry
    match host.state_mut().launchpads.entry(name) {
        // If the launch-pad with the same product name exists
        // it will not allow the launch-pad to be inserted
        Entry::Occupied(_) => {
            bail!(LaunchPadError::ProductNameAlreadyTaken)
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
    error = "LaunchPadError",
    enable_logger
)]
fn approve_launchpad(
    ctx: &ReceiveContext,
    host: &mut Host<State>,
    logger: &mut Logger,
) -> ContractResult<()> {
    // Ensure that the sender is an account.
    ensure!(ctx.sender().is_account(), LaunchPadError::OnlyAccount);

    // Only admin is allowed to approve launch-pad for presale
    ensure!(
        ctx.sender() == host.state().admin_address().into(),
        LaunchPadError::OnlyAdmin
    );

    // Product name is passed as parameter to identify the
    // corresponding Launch-pad
    let params: ApprovalParams = ctx.parameter_cursor().get()?;

    // Getting the launch-pad to be approved and updating its
    // status to LIVE
    let mut launch_pad = host.state_mut().get_mut_launchpad(params.product_name)?;

    let transfer_to = if params.approve {
        // Updating the launch-pad status to approved
        launch_pad.status = LaunchPadStatus::APPROVED;

        logger.log(&Event::APPROVED(ApproveEvent {
            launchpad_name: launch_pad.product_name(),
        }))?;

        drop(launch_pad);

        host.state().admin_address()
    } else {
        // Updating the launch-pad status to rejected if analyst
        // has rejected the launchpad
        launch_pad.status = LaunchPadStatus::REJECTED;

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
    error = "LaunchPadError",
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
        Address::Account(_) => bail!(LaunchPadError::OnlyContract),
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
        LaunchPadError::UnAuthorized
    );

    // Ensure that the correct CIS2 contract invoked the
    // deposit entry point using OnReceive hook
    ensure!(
        contract == launch_pad.get_cis2_contract(),
        LaunchPadError::WrongContract
    );

    // Ensure other details, such as the correct token amount
    // is received or we have received the correct tokens by
    // matching the token ID given in launch-pad params
    ensure!(
        amount == launch_pad.get_product_token_amount(),
        LaunchPadError::WrongTokenAmount
    );
    ensure!(
        token_id == launch_pad.get_product_token_id(),
        LaunchPadError::WrongTokenID
    );

    // If every claim is valid, Launch-pad is made LIVE for presale
    // for the current product
    launch_pad.status = LaunchPadStatus::LIVE;

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
    error = "LaunchPadError"
)]
fn live_pause(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    ensure!(ctx.sender().is_account(), LaunchPadError::OnlyAccount);

    // Reading parameters
    let params: LivePauseParams = ctx.parameter_cursor().get()?;

    // Getting the launch pad from state identified by the product name
    let mut launch_pad = host.state_mut().get_mut_launchpad(params.poduct_name)?;

    // Product owner (developer) is only allowed to pause
    // the launch pad
    ensure!(
        ctx.sender()
            .matches_account(&launch_pad.get_product_owner()),
        LaunchPadError::UnAuthorized
    );

    // Launch pad can only be pause during vesting or before
    // reaching the soft cap
    ensure!(launch_pad.reached_soft_cap(), LaunchPadError::SoftReached);
    ensure!(launch_pad.is_finished(ctx), LaunchPadError::Finished);

    // Check if owner wants to pause the launch pad
    if params.to_pause {
        // Check if the launch pad is already paused
        ensure!(launch_pad.is_live(), LaunchPadError::Paused);
        // Check if the pause limit reached, launch pad is allowed
        // to be paused 3 times
        ensure!(
            launch_pad.current_pause_count() < MAX_PAUSE_COUNT,
            LaunchPadError::PauseLimit
        );
        // Check if the pause duration given is not less than the
        // minimum allowed pause duration 48 hrs
        ensure!(
            params.pause_duration.duration_as_millis() >= MIN_PAUSE_DURATION,
            LaunchPadError::PauseDuration
        );

        // Pausing the launch pad
        launch_pad.status = LaunchPadStatus::PAUSED;
        // Setting new pause details in launch pad
        launch_pad.pause.timeperiod = params.pause_duration;
        launch_pad.pause.count += 1;

        return Ok(());
    }

    // Whether the launch-pad is already live
    ensure!(launch_pad.is_paused(), LaunchPadError::Live);
    // Check if the time is still left for pause duration
    // to complete
    ensure!(
        launch_pad.is_pause_elapsed(ctx.metadata().block_time()),
        LaunchPadError::TimeStillLeft
    );

    // Resuming the launch pad
    launch_pad.status = LaunchPadStatus::LIVE;
    // Resetting the pause durations
    launch_pad.pause.timeperiod = TimePeriod::default();

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "Vest",
    mutable,
    parameter = "VestParams",
    error = "LaunchPadError",
    payable
)]
fn vest(ctx: &ReceiveContext, host: &mut Host<State>, amount: Amount) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    let holder = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(LaunchPadError::OnlyAccount),
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
    ensure!(launch_pad.is_paused(), LaunchPadError::Paused);
    ensure!(launch_pad.is_canceled(), LaunchPadError::Canceled);
    ensure!(!launch_pad.is_finished(ctx), LaunchPadError::Finished);

    // Verify whether the payable vesting amount received is within the
    // min and max vesting allowed
    ensure!(
        params.token_amount >= launch_pad.vest_min()
            && params.token_amount <= launch_pad.vest_max(),
        LaunchPadError::Insufficient
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
                cycles_rolled: 0,
                release_data: state_builder.new_map(),
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
                    LaunchPadError::VestLimit
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
    launch_pad.product.allocated_tokens -= params.token_amount;

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
        launchpad.product.allocated_tokens -= allocated_cut;

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
    parameter = "String",
    error = "LaunchPadError"
)]
fn claim_tokens(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    let holder = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(LaunchPadError::OnlyAccount),
    };

    // Reading the product name to identify the launch pad
    let product_name: ProductName = ctx.parameter_cursor().get()?;

    // Getting the launch pad from state identified by the product name
    let mut launch_pad = host.state_mut().get_mut_launchpad(product_name)?;

    // Make sure that the launch pad is not paused, is not canceled
    // or is finished. As well as the cliff duration has elapsed
    ensure!(launch_pad.is_canceled(), LaunchPadError::Canceled);
    ensure!(launch_pad.is_finished(ctx), LaunchPadError::Vesting);
    ensure!(
        launch_pad.is_cliff_elapsed(ctx),
        LaunchPadError::CliffNotElapsed
    );

    let holder_info = launch_pad.get_holder_info(holder)?;
    let time_now = ctx.metadata().block_time();

    // Check whether the holder is claiming the first cycle of release
    // First release cycle is handled differently than the other forth
    // coming cycles.
    // Moreover, token releases are made linear over the release cycles,
    // it means that the tokens are equally distributed for each cycle.
    let (cycle, claimable) = if holder_info.cycles_rolled == 0 {
        // Make sure that the current cycle duration has elasped
        // since the cliff duration
        ensure!(
            time_now
                .duration_since(launch_pad.lock_up.cliff)
                .unwrap()
                .millis()
                >= CYCLE_DURATION,
            LaunchPadError::CycleNotElapsed
        );

        // Return the current release cycle count and amount of
        // claimable tokens
        (1, holder_info.tokens.0 / launch_pad.lock_up.release_cycles)
    } else {
        let last_cycle = holder_info.cycles_rolled;
        let last_released = holder_info.release_data.get(&last_cycle).unwrap();

        // Ensuring that holder is not exceeding the claims more
        // than the number of release cycles.
        // Also ensuring that the cycle duration has elapsed since
        // the last release for the holder
        ensure!(
            last_cycle >= launch_pad.lock_up.release_cycles as u8,
            LaunchPadError::CyclesCompleted
        );
        ensure!(
            time_now.duration_since(last_released.1).unwrap().millis() >= CYCLE_DURATION,
            LaunchPadError::CycleNotElapsed
        );

        // Return the current release cycle count and amount of
        // claimable tokens
        (
            last_cycle + 1,
            holder_info.tokens.0 / launch_pad.lock_up.release_cycles,
        )
    };

    // Setting the release information regarding the current release
    // being made for the current holder
    launch_pad.set_holder_release_info(holder, cycle, (claimable.into(), time_now));

    let cis2_client = Cis2Client::new(launch_pad.get_cis2_contract());
    let token_id = launch_pad.get_product_token_id();

    drop(launch_pad);

    // Here are the allocated tokens transfered to the holder based on
    // the current release cycle count.
    cis2_client.transfer::<State, TokenID, TokenAmount, LaunchPadError>(
        host,
        Transfer {
            token_id,
            amount: claimable.into(),
            from: ctx.self_address().into(),
            to: holder.into(),
            data: AdditionalData::empty(),
        },
    )?;

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "WithdrawFunds",
    mutable,
    parameter = "String",
    error = "LaunchPadError"
)]
fn withdraw_raised(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    let owner = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(LaunchPadError::OnlyAccount),
    };

    // Reading the product name to identify the launch pad
    let product_name: ProductName = ctx.parameter_cursor().get()?;

    // Getting the launch pad from state identified by the product name
    let launch_pad = host.state().get_launchpad(product_name.clone())?;

    // Make sure that the transaction is authorized
    ensure!(
        owner == launch_pad.get_product_owner(),
        LaunchPadError::UnAuthorized
    );

    // Make sure that the launch pad is not paused, is not canceled
    // or is finished.
    ensure!(launch_pad.is_canceled(), LaunchPadError::Canceled);
    ensure!(!launch_pad.is_completed(), LaunchPadError::Completed);
    ensure!(launch_pad.is_finished(ctx), LaunchPadError::Vesting);

    // Owner can only withdraw collected funds if and only if
    // the product has acheived soft cap and the funds are not
    // already raised
    if !launch_pad.withdrawn && launch_pad.reached_soft_cap() {
        ensure!(
            launch_pad.reached_soft_cap(),
            LaunchPadError::SoftNotReached
        );

        // Calculating the amount of funds in CCD to be locked
        // in liquidity according to the percentage provided by
        // the owner
        let liquidity_allocation = Amount::from_micro_ccd(
            (launch_pad.collected.micro_ccd * launch_pad.liquidity_details.liquidity_allocation)
                / 100,
        );

        // Remaining amount in CCD that can be withdrawn after the
        // allocation
        let withdrawable = launch_pad.collected - liquidity_allocation;

        // TODO
        //
        // Need to implement the liquidity logic with DEX integration
        // to lock the funds before trasfering the funds to the owner
        //
        // And pay the the LPTokens bought from the DEX to admin
        let liquidity_params = AddLiquidityParams {
            token: TokenInfo {
                id: TokenIdVec(launch_pad.get_product_token_id().0.to_ne_bytes().into()),
                address: launch_pad.get_cis2_contract(),
            },
            token_amount: TokenAmount::from(100),
        };

        let _: LPTokenInfo = host
            .invoke_contract(
                &host.state().dex_address(),
                &liquidity_params,
                EntrypointName::new_unchecked("addLiquidity"),
                Amount::zero(),
            )?
            .1
            .unwrap()
            .get()?;

        // Transfering the withdrawable amount to the owner
        host.invoke_transfer(&owner, withdrawable)?;
        // Set the withdrawn flag in launchpad state
        host.state_mut()
            .get_mut_launchpad(product_name)
            .unwrap()
            .withdrawn = true;

        return Ok(());
    }

    Err(LaunchPadError::WithDrawn)
}

#[receive(
    contract = "LaunchPad",
    name = "WithDrawLockedFunds",
    mutable,
    parameter = "String",
    error = "LaunchPadError"
)]
fn withdraw_locked_funds(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    todo!("NOT YET IMPLEMENTED")
}

#[receive(
    contract = "LaunchPad",
    name = "CancelLaunchPad",
    mutable,
    parameter = "String",
    error = "LaunchPadError"
)]
fn cancel(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    // Only Account is supposed to invoke this method
    let owner = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(LaunchPadError::OnlyAccount),
    };

    // Reading the product name to identify the launch pad
    let product_name: ProductName = ctx.parameter_cursor().get()?;

    // Getting the launch pad from state identified by the product name
    let launch_pad = host.state().get_launchpad(product_name.clone())?;

    // Make sure that the transaction is authorized
    ensure!(
        owner == launch_pad.get_product_owner(),
        LaunchPadError::UnAuthorized
    );

    // Make sure that the launch pad is not already canceled, did not
    //reach the soft cap, launch pad is not completed.
    ensure!(
        !launch_pad.is_canceled() && !launch_pad.is_completed() && !launch_pad.reached_soft_cap(),
        LaunchPadError::Canceled
    );

    host.state_mut()
        .get_mut_launchpad(product_name)
        .unwrap()
        .status = LaunchPadStatus::CANCELED;

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "viewState",
    return_value = "StateView",
    error = "LaunchPadError"
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
    error = "LaunchPadError"
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
    error = "LaunchPadError"
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
    error = "LaunchPadError"
)]
fn view_my_launch_pads(ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<LaunchPadsView> {
    let holder = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => bail!(LaunchPadError::OnlyAccount),
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
