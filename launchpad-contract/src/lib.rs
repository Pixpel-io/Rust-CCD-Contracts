use concordium_cis2::{
    AdditionalData, Cis2Client, OnReceivingCis2DataParams, TokenAmountU64 as TokenAmount,
    TokenIdU8 as TokenID, Transfer,
};
use concordium_std::{
    bail, ensure, init, receive, Address, Amount, DeserialWithState, Entry, ExternContext,
    ExternReceiveContext, ExternReturnValue, ExternStateApi, Get, HasChainMetadata, HasCommonData,
    HasHost, HasInitContext, HasLogger, HasReceiveContext, HasStateApi, HasStateEntry, Host,
    InitContext, InitResult, Logger, ParseError, ReceiveContext, Reject, Serial, StateBuilder,
    UnwrapAbort, Write,
};
use errors::LaunchPadError;
use events::{ApproveEvent, CreateLaunchPadEvent, Event, RejectEvent, VestEvent};
use params::{ApprovalParams, CreateParams, InitParams, LivePauseParams, VestParams};
use state::{HolderInfo, LaunchPad, LaunchPadStatus, State, TimePeriod};
use types::ContractResult;

mod contract;
mod errors;
mod events;
mod params;
mod response;
mod state;
mod types;

#[cfg(test)]
mod tests;

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
type OnReceiveCIS2Params = OnReceivingCis2DataParams<TokenID, TokenAmount, ProductName>;

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
    let params: CreateParams = ctx
        .parameter_cursor()
        .get()
        .map_err(|_e: ParseError| LaunchPadError::ParseParams)?;

    // Esnure that user pays the complete registeration Fee
    ensure!(
        amount >= host.state().admin_registeration_fee(),
        LaunchPadError::InSufficientAmount
    );

    // Ensure hard-cap is greater than the soft-cap
    if let Some(hard_cap) = params.hard_cap {
        ensure!(hard_cap > params.soft_cap, LaunchPadError::HardCappSmaller)
    }

    let time_now = ctx.metadata().block_time();

    // Ensure that the launch-pad active time period is valid
    params.timeperiod.ensure_is_period_valid(time_now)?;

    // Ensure that the provided cliff time period is valid.
    // Cliff is consdiered only if it starts after the vesting
    // and has minimum duration of 7 days
    ensure!(
        params.cliff().millis() < MIN_CLIFF_DURATION,
        LaunchPadError::InCorrectCliffPeriod
    );

    // Creating the Launch-pad from user defined params and
    // getting the launch-pad ID
    let (launchpad_id, launchpad) = LaunchPad::from_create_params(params, &mut host.state_builder);

    // Updating the contract State with new launchpad entry
    match host.state_mut().launchpads.entry(launchpad_id) {
        // If the launch-pad with the same product name exists
        // it will not allow the launch-pad to be inserted
        Entry::Occupied(_) => {
            bail!(LaunchPadError::ProductNameAlreadyTaken)
        }
        // Or else it will insert the launch-pad in State and
        // dispatch the launch pad creation event
        Entry::Vacant(entry) => {
            logger.log(&Event::CREATED(CreateLaunchPadEvent {
                launchpad_id,
                launchpad_name: launchpad.product_name(),
                owner: launchpad.get_product_owner(),
                allocated_tokens: launchpad.get_product_token_amount(),
                base_price: launchpad.product_base_price(),
            }))?;

            entry.insert(launchpad);
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
    let (launch_pad_id, mut launchpad) = host.state_mut().get_mut_launchpad(params.product_name)?;

    let transfer_to = if params.approve {
        // Updating the launch-pad status to approved
        launchpad.status = LaunchPadStatus::APPROVED;

        logger.log(&Event::APPROVED(ApproveEvent {
            launchpad_id: launch_pad_id,
            launchpad_name: launchpad.product_name(),
        }))?;

        drop(launchpad);

        host.state().admin_address()
    } else {
        // Updating the launch-pad status to rejected if analyst
        // has rejected the launchpad
        launchpad.status = LaunchPadStatus::REJECTED;

        logger.log(&Event::REJECTED(RejectEvent {
            launchpad_id: launch_pad_id,
            launchpad_name: launchpad.product_name(),
        }))?;

        let owner = launchpad.get_product_owner();
        drop(launchpad);

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
    parameter = "OnReceiveCIS2Params",
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

    // Fetching the launch-pad from the state if the correct
    // product name is supplied
    let (launch_pad_id, mut launchpad) = host.state_mut().get_mut_launchpad(data)?;

    // Making sure that the deposit is made by the product
    // owner
    ensure!(
        from == Address::Account(launchpad.get_product_owner()),
        LaunchPadError::UnAuthorized
    );

    // Ensure that the correct CIS2 contract invoked the
    // deposit entry point using OnReceive hook
    ensure!(
        contract == launchpad.get_cis2_contract(),
        LaunchPadError::WrongContract
    );

    // Ensure other details, such as the correct token amount
    // is received or we have received the correct tokens by
    // matching the token ID given in launch-pad params
    ensure!(
        amount == launchpad.get_product_token_amount(),
        LaunchPadError::WrongTokenAmount
    );
    ensure!(
        token_id == launchpad.get_product_token_id(),
        LaunchPadError::WrongTokenID
    );

    // If every claim is valid, Launch-pad is made LIVE for presale
    // for the current product
    launchpad.status = LaunchPadStatus::LIVE;

    // Dispatching the event as notification when the vesting start
    // as soon as the allocated tokens are deposited
    logger.log(&Event::VESTINGSTARTED(VestEvent {
        launchpad_id: launch_pad_id,
        launchpad_name: launchpad.product_name(),
        vesting_time: launchpad.timeperiod,
        vesting_limits: launchpad.vest_limits.clone(),
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
    let (_, mut launchpad) = host.state_mut().get_mut_launchpad(params.poduct_name)?;

    // Product owner (developer) is only allowed to pause
    // the launch pad
    ensure!(
        ctx.sender().matches_account(&launchpad.get_product_owner()),
        LaunchPadError::UnAuthorized
    );

    // Launch pad can only be pause during vesting or before
    // reaching the soft cap
    ensure!(launchpad.reached_soft_cap(), LaunchPadError::SoftReached);
    ensure!(launchpad.is_finished(ctx), LaunchPadError::VestingFinished);

    // Check if owner wants to pause the launch pad
    if params.to_pause {
        // Check if the launch pad is already paused
        ensure!(launchpad.is_live(), LaunchPadError::Paused);
        // Check if the pause limit reached, launch pad is allowed
        // to be paused 3 times
        ensure!(
            launchpad.current_pause_count() < MAX_PAUSE_COUNT,
            LaunchPadError::PauseLimit
        );
        // Check if the pause duration given is not less than the
        // minimum allowed pause duration 48 hrs
        ensure!(
            params.pause_duration.duration_as_millis() >= MIN_PAUSE_DURATION,
            LaunchPadError::PauseDuration
        );

        // Pausing the launch pad
        launchpad.status = LaunchPadStatus::PAUSED;
        // Setting new pause details in launch pad
        launchpad.pause.timeperiod = params.pause_duration;
        launchpad.pause.count += 1;

        return Ok(());
    }

    // Whether the launch-pad is already live
    ensure!(launchpad.is_paused(), LaunchPadError::Live);
    // Check if the time is still left for pause duration
    // to complete
    ensure!(
        launchpad.is_pause_elapsed(ctx.metadata().block_time()),
        LaunchPadError::TimeStillLeft
    );

    // Resuming the launch pad
    launchpad.status = LaunchPadStatus::LIVE;
    // Resetting the pause durations
    launchpad.pause.timeperiod = TimePeriod::default();

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
    let (launch_pad_id, mut launchpad) = state.get_mut_launchpad(params.product_name)?;

    // Make sure that the launch pad is not paused, is not canceled
    // or is not finished, either due to vesting duration elapsed or
    // due to hard cap limit reached
    ensure!(launchpad.is_paused(), LaunchPadError::Paused);
    ensure!(launchpad.is_canceled(), LaunchPadError::Canceled);
    ensure!(!launchpad.is_finished(ctx), LaunchPadError::VestingFinished);

    // Verify whether the payable vesting amount received is within the
    // min and max vesting allowed
    ensure!(
        params.token_amount >= launchpad.vest_min() && params.token_amount <= launchpad.vest_max(),
        LaunchPadError::InSufficientAmount
    );

    let vest_max = launchpad.vest_max();

    // Updating or inserting the holder(investor) depending whether the
    // holder is new or existing in the launch pad state
    match launchpad.holders.entry(holder) {
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
    launchpad.collected += amount;
    launchpad.product.allocated_tokens -= params.token_amount;

    // Get the amount of tokens allocated for presale by the owner
    let allocated_tokens = launchpad.product.allocated_tokens;
    // Check if the product has acheived soft cap
    let reached_soft_cap = launchpad.reached_soft_cap();
    // Check if the product has paid the soft cap share to the platform
    let allocation_paid = launchpad.allocation_paid;

    drop(launchpad);

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
        let mut launchpad = host.state_mut().get_launchpad_by_id(launch_pad_id)?;
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
            entry.insert(vec![launch_pad_id]);
        }
        // Update the existing holder in the state with launch pad ID
        Entry::Occupied(mut entry) => {
            entry.modify(|launchpads| {
                if !launchpads.contains(&launch_pad_id) {
                    launchpads.push(launch_pad_id);
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
    let (_, mut launchpad) = host.state_mut().get_mut_launchpad(product_name)?;

    // Make sure that the launch pad is not paused, is not canceled
    // or is finished. As well as the cliff duration has elapsed
    ensure!(launchpad.is_canceled(), LaunchPadError::Canceled);
    ensure!(launchpad.is_finished(ctx), LaunchPadError::StillVesting);
    ensure!(
        launchpad.is_cliff_elapsed(ctx),
        LaunchPadError::CliffNotElapsed
    );

    let holder_info = launchpad.get_holder_info(holder)?;
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
                .duration_since(launchpad.lock_up.cliff)
                .unwrap()
                .millis()
                >= CYCLE_DURATION,
            LaunchPadError::CycleNotElapsed
        );

        // Return the current release cycle count and amount of
        // claimable tokens
        (1, holder_info.tokens.0 / launchpad.lock_up.release_cycles)
    } else {
        let last_cycle = holder_info.cycles_rolled;
        let last_released = holder_info.release_data.get(&last_cycle).unwrap();

        // Ensuring that holder is not exceeding the claims more
        // than the number of release cycles.
        // Also ensuring that the cycle duration has elapsed since
        // the last release for the holder
        ensure!(
            last_cycle >= launchpad.lock_up.release_cycles as u8,
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
            holder_info.tokens.0 / launchpad.lock_up.release_cycles,
        )
    };

    // Setting the release information regarding the current release
    // being made for the current holder
    launchpad.set_holder_release_info(holder, cycle, (claimable.into(), time_now));

    let cis2_client = Cis2Client::new(launchpad.get_cis2_contract());
    let token_id = launchpad.get_product_token_id();

    drop(launchpad);

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
    let (launch_pad_id, launchpad) = host.state().get_launchpad(product_name)?;

    // Make sure that the transaction is authorized
    ensure!(
        owner == launchpad.get_product_owner(),
        LaunchPadError::UnAuthorized
    );

    // Make sure that the launch pad is not paused, is not canceled
    // or is finished.
    ensure!(launchpad.is_canceled(), LaunchPadError::Canceled);
    ensure!(!launchpad.is_completed(), LaunchPadError::Completed);
    ensure!(launchpad.is_finished(ctx), LaunchPadError::StillVesting);

    // Owner can only withdraw collected funds if and only if
    // the product has acheived soft cap and the funds are not
    // already raised
    if !launchpad.withdrawn && launchpad.reached_soft_cap() {
        ensure!(launchpad.reached_soft_cap(), LaunchPadError::SoftNotReached);

        // Calculating the amount of funds in CCD to be locked
        // in liquidity according to the percentage provided by
        // the owner
        let liquidity_allocation = Amount::from_micro_ccd(
            (launchpad.collected.micro_ccd * launchpad.liquidity_details.liquidity_allocation)
                / 100,
        );

        // Remaining amount in CCD that can be withdrawn after the
        // allocation
        let withdrawable = launchpad.collected - liquidity_allocation;

        // TODO
        //
        // Need to implement the liquidity logic with DEX integration
        // to lock the funds before trasfering the funds to the owner
        //
        // And pay the the LPTokens bought from the DEX to admin

        // Transfering the withdrawable amount to the owner
        host.invoke_transfer(&owner, withdrawable)?;
        // Set the withdrawn flag in launchpad state
        host.state_mut()
            .get_launchpad_by_id(launch_pad_id)
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
fn withdraw_locked_funds(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()>  {
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
    let (launch_pad_id, launchpad) = host.state().get_launchpad(product_name)?;

    // Make sure that the transaction is authorized
    ensure!(
        owner == launchpad.get_product_owner(),
        LaunchPadError::UnAuthorized
    );

    // Make sure that the launch pad is not already canceled, did not
    //reach the soft cap, launch pad is not completed.
    ensure!(
        !launchpad.is_canceled() && !launchpad.is_completed() && !launchpad.reached_soft_cap(),
        LaunchPadError::Canceled
    );

    host.state_mut()
        .get_launchpad_by_id(launch_pad_id)
        .unwrap()
        .status = LaunchPadStatus::CANCELED;

    Ok(())
}
