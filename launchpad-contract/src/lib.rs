use concordium_cis2::{
    Cis2Client, OnReceivingCis2DataParams, TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID,
};
use concordium_std::{
    bail, ensure, init, receive, Address, Amount, DeserialWithState, Entry, ExternContext,
    ExternReceiveContext, ExternReturnValue, ExternStateApi, Get, HasChainMetadata, HasCommonData,
    HasHost, HasInitContext, HasReceiveContext, HasStateApi, HasStateEntry, Host, InitContext,
    InitResult, ParseError, ReceiveContext, Reject, Serial, StateBuilder, Timestamp, UnwrapAbort,
    Write,
};
use errors::LaunchPadError;
use params::{ApprovalParams, CreateParams, InitParams, LivePauseParams};
use state::{LaunchPad, LaunchPadStatus, State, TimePeriod};
use types::ContractResult;

pub mod contract;
pub mod errors;
pub mod params;
pub mod response;
pub mod state;
pub mod types;

#[cfg(test)]
mod tests;

pub type ProductName = String;

/// Minimum Cliff duration allowed for a product before vesting
/// in milliseconds
const MIN_CLIFF_DURATION: u64 = 6.048e+8 as u64;

/// Minimum Pause duration allowed for a product to be pasued
/// before vesting in milliseconds
const MIN_PAUSE_DURATION: u64 = 1.728e+8 as u64;

/// Launch-Pad can only be pause at most three times
const MAX_PAUSE_COUNT: u8 = 3;

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
    payable
)]
fn create_launchpad(
    ctx: &ReceiveContext,
    host: &mut Host<State>,
    amount: Amount,
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
        amount >= host.state().registeration_fee(),
        LaunchPadError::InsuffiecienRegFee
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
        params.cliff().end > params.launchpad_end_time()
            && params.cliff().duration_as_millis() < MIN_CLIFF_DURATION,
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
        // Or else it will insert the launch-pad
        Entry::Vacant(entry) => {
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
    error = "LaunchPadError"
)]
fn approve_launchpad(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
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
    let mut launchpad = host.state_mut().get_launchpad(params.product_name)?;

    if params.approve {
        // Updating the launch-pad status to approved
        launchpad.status = LaunchPadStatus::APPROVED;

        drop(launchpad);

        // Transfering the registeration fee to platform
        host.invoke_transfer(
            &host.state().admin_address(),
            host.state().registeration_fee(),
        )?;

        return Ok(());
    }

    // Updating the launch-pad status to rejected if analyst
    // has rejected the launchpad
    launchpad.status = LaunchPadStatus::REJECTED;
    let owner = launchpad.get_product_owner();

    drop(launchpad);

    // Refunding the product owner in case if the launch-pad
    // is rejected
    host.invoke_transfer(&owner, host.state().registeration_fee())?;

    Ok(())
}

#[receive(
    contract = "LaunchPad",
    name = "Deposit",
    mutable,
    parameter = "OnReceiveCIS2Params",
    error = "LaunchPadError"
)]
fn deposit_tokens(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
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
    let mut launchpad = host.state_mut().get_launchpad(data)?;

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
    let mut launchpad = host.state_mut().get_launchpad(params.poduct_name)?;

    // Product owner (developer) is only allowed to pause
    // the launch pad
    ensure!(
        ctx.sender().matches_account(&launchpad.get_product_owner()),
        LaunchPadError::UnAuthorized
    );

    // Launch pad can only be pause during vesting
    ensure!(
        launchpad.is_vesting_finished(ctx.metadata().block_time()),
        LaunchPadError::VestingFinished
    );

    // Check if owner wants to pause the launch pad
    if params.to_pause {
        // Check if the launch pad is already paused
        ensure!(launchpad.is_live(), LaunchPadError::AlreadyPaused);
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
    ensure!(launchpad.is_paused(), LaunchPadError::AlreadyLive);
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
