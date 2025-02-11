use concordium_std::{
    bail, ensure, init, receive, Address, Amount, DeserialWithState, Entry, ExternContext,
    ExternReceiveContext, ExternReturnValue, ExternStateApi, Get, HasChainMetadata, HasCommonData,
    HasHost, HasInitContext, HasReceiveContext, HasStateApi, HasStateEntry, Host, InitContext,
    InitResult, ParseError, ReceiveContext, Reject, Serial, StateBuilder, UnwrapAbort, Write,
};
use errors::LaunchPadError;
use params::{CreateParams, InitParams};
use state::{LaunchPad, State};
use types::VestingResult;

pub mod contract;
pub mod errors;
pub mod params;
pub mod response;
pub mod state;
pub mod types;

#[cfg(test)]
mod tests;

/// Entry point which initializes the contract with new default state.
///
/// The state is empty except that the user must provide admin parameters
/// to be set while initialization.
#[init(contract = "launchpad", parameter = "InitParams")]
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
    contract = "launchpad",
    name = "CreateLaunchPad",
    mutable,
    parameter = "CreateParams",
    error = "VestingError",
    payable
)]
fn create_launchpad(
    ctx: &ReceiveContext,
    host: &mut Host<State>,
    amount: Amount,
) -> VestingResult<()> {
    // Ensure that the sender is an account.
    let acc = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(LaunchPadError::ContractUser.into()),
    };

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

    // Ensure that the provided cliff time period is valid
    ensure!(
        params.cliff_timestamp() > time_now
            && params.cliff_timestamp() > params.launchpad_end_time(),
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

    // crate::cis2_client::Cis2Client::transfer(
    //     host,
    //     params.token_param.id.clone(),
    //     params.token_param.address,
    //     params.token_param.token_amount,
    //     ctx.sender(),
    //     Receiver::Contract(
    //         ctx.self_address(),
    //         OwnedEntrypointName::new_unchecked("onReceivingCIS2".to_string()),
    //     ),
    // )?;

    // let platform_fee_to_amount = Amount {
    //     micro_ccd: calculate_platform_fee,
    // };

    // let admin = host.state().admin.clone();

    // host.invoke_transfer(&admin, platform_fee_to_amount)
    //     .map_err(|_| VestingError::InvalidUser)?;

    Ok(())
}
