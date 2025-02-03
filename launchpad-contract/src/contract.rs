///! A Pixpel Launchpad
///
/// Terms
/// Term Rocket = rocket is similar to launchpad that invest create to get initial funding
/// Term Developer = user whol will create the rocket for getting funding against cis2 token
/// Term Player = user who will invest in the rocket
///
/// In this launchpad 2 kind of users can interact first is developer and second is player
/// Developer will create rocket with few validations, validations will be covered in following.
/// Player will invest in the rocket until unless rocket end time reach or rocket hardcap limit reached
///
/// Functions guideline
/// Init: Init function is for initializing the smart contract state where we have 3 state
///  - launchpad ==> all the information until lockup will start
///  - total_launchpad ==> length of launchpad
///  - lockup_detail ==> all the information after lockup will start
///  - admin ==> admin where all platform fee will transfer
///
/// create_launchpad
///  - in this function we are creating launchpad will following validations
///     - developer must pay 6% of hardcap that will transfer to smart contract 5% fee is security fee and 1% is platform fee
///     - hardcap must be > 40% from softcap
///     - sender must not be smart contract
///     - and at the end transfer 6% ccd to admin wallet and the desire ampount of cis2 token to smart contract
///
/// onReceivingCIS2
///  - thi is a hook which helps to transfer cis2 token from sender to smart contract
///
/// update_admin
///  - this help to update admin wallet addres but only exisiting admin can change admin address
///
/// live_pause
///  - this function use to turn live and pause but once developer turn live user must wait for 12 hours to turn back against
///  - developer can pause the launchpad maximum 3 time
///  - launchpad will be live automatically after 48 hours
///
/// vest
///  -  players can invest through this function in launchpad with following validations
///     - launchpad must not be pause
///     - launchpad must not be cancel
///     - min and max amount must satisfy
///     - if current amount + exisitng invested amout == hardcap then launchpad must be finalized
///     - cliff period will be start from that time if hardcap limit reached
///     - add dtails in lockup details state
///
/// retrieve
///  - players can get their ccd amount back through function before end_time reached or hardcap limit and cancel must be false
/// because when cancel will true players will get their ccd automatically
///
/// claim
///  - this functions help players to claim their tokens when cliff period end and claim date arrived, claim tokens date is multiple
/// each date must satisfy the current time
///
/// view
///  - everyone can view the state of launchpad total_launchpad and lockup_details
///
use concordium_cis2::*;
use concordium_std::{collections::BTreeMap, *};

use crate::errors::*;
use crate::params::*;
use crate::response::*;
use crate::state::State;
use crate::types::*;

fn get_token_reserve<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
    token_info: &TokenInfo,
) -> ContractResult<ContractTokenAmount> {
    crate::cis2_client::Cis2Client::get_balance(
        host,
        token_info.id.clone(),
        &token_info.address,
        Address::Contract(ctx.self_address()),
    )
}

/// Initialize the contract instance.
#[init(contract = "launchpad", parameter = "InitParameter")]
fn init<S: HasStateApi>(
    _ctx: &impl HasInitContext,
    _state_builder: &mut StateBuilder<S>,
) -> InitResult<State> {
    // Parse the parameter.
    let param: InitParameter = _ctx.parameter_cursor().get()?;

    // Set the state.
    Ok(State {
        launchpad: BTreeMap::new(),
        total_launchpad: 0,
        admin: param.admin,
        lockup_details: BTreeMap::new(),
    })
}

#[receive(
    contract = "launchpad",
    name = "create_launchpad",
    mutable,
    parameter = "LaunchpadParam",
    error = "VestingError",
    payable
)]
fn create_launchpad<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
    _amount: Amount,
) -> VestingResult<()> {
    // Ensure that the sender is an account.
    let acc = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(VestingError::ContractUser.into()),
    };

    // parse the parameter
    let new_launchpad_param: LaunchpadParam = ctx
        .parameter_cursor()
        .get()
        .map_err(|_e: ParseError| VestingError::ParseParams)?;

    // calculate the 5% fee that will applicable if developer will cancel the contract otherwise he will get back
    let calculate_dev_amount_if_cancel = new_launchpad_param.hard_cap * 5 / 100;

    // calculate the 1% fee that will transfer to platform
    let calculate_platform_fee = new_launchpad_param.hard_cap * 1 / 100;

    ensure!(
        _amount
            >= Amount {
                micro_ccd: calculate_dev_amount_if_cancel + calculate_platform_fee
            },
        VestingError::InsuffiecientFunds
    );

    ensure!(
        new_launchpad_param.hard_cap > new_launchpad_param.soft_cap,
        VestingError::HardCappSmaller
    );

    // hardcap must be greater than 40% of softcap
    let soft_cap_minimum = new_launchpad_param.soft_cap * 40 / 100;
    ensure!(
        new_launchpad_param.hard_cap > new_launchpad_param.soft_cap + soft_cap_minimum,
        VestingError::HardcapNot40ToSoftcap
    );

    // clone state
    let launchpad_counter = host.state().total_launchpad.clone();

    let launchpad_data = Launchpad {
        start_time: new_launchpad_param.start_time,
        end_time: new_launchpad_param.end_time,
        live: new_launchpad_param.live,
        owner: acc,
        soft_cap: Amount {
            micro_ccd: new_launchpad_param.soft_cap,
        },
        hard_cap: Amount {
            micro_ccd: new_launchpad_param.hard_cap,
        },
        minimum_invest: Amount {
            micro_ccd: new_launchpad_param.minimum_invest,
        },
        maximum_invest: Amount {
            micro_ccd: new_launchpad_param.maximum_invest,
        },
        invest_amount: Amount { micro_ccd: 0 },
        holders: BTreeMap::new(),
        cancel: false,
        total_tx: 0,
        dev_paid: Amount {
            micro_ccd: calculate_dev_amount_if_cancel,
        },
        pause_until: ctx.metadata().slot_time(),
        pause_start: ctx.metadata().slot_time(),
        live_pause_count: 0,
        cis2_amount: new_launchpad_param.token_param.token_amount,
        cis2_price: new_launchpad_param.cis2_price,
    };

    // develoepr will add days that will start after end time so following is the feature to add cliff period in end time
    let end_time_millis = Timestamp::timestamp_millis(&new_launchpad_param.end_time);
    let cliff_period_millis: Timestamp =
        Timestamp::from_timestamp_millis(end_time_millis + new_launchpad_param.cliff_duration);

    let lockup_detail = LockupDetails {
        cliff_period: cliff_period_millis,
        lockup_holders: BTreeMap::new(),
        token_release_data: new_launchpad_param.token_release_data,
        cliff_duration: new_launchpad_param.cliff_duration,
    };

    crate::cis2_client::Cis2Client::transfer(
        host,
        new_launchpad_param.token_param.id.clone(),
        new_launchpad_param.token_param.address,
        new_launchpad_param.token_param.token_amount,
        ctx.sender(),
        Receiver::Contract(
            ctx.self_address(),
            OwnedEntrypointName::new_unchecked("onReceivingCIS2".to_string()),
        ),
    )?;

    let platform_fee_to_amount = Amount {
        micro_ccd: calculate_platform_fee,
    };

    host.state_mut()
        .lockup_details
        .insert(launchpad_counter + 1, lockup_detail);

    //insert launchpad
    host.state_mut()
        .launchpad
        .insert(launchpad_counter + 1, launchpad_data);

    host.state_mut().total_launchpad += 1;

    let admin = host.state().admin.clone();

    host.invoke_transfer(&admin, platform_fee_to_amount)
        .map_err(|_| VestingError::InvalidUser)?;

    Ok(())
}

/// this is the hook function for executing the CIS2 tokn transaction from user to contract address
#[receive(
    contract = "launchpad",
    name = "onReceivingCIS2",
    error = "VestingError"
)]
fn token_on_cis2_received<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    _host: &impl HasHost<State, StateApiType = S>,
) -> VestingResult<()> {
    Ok(())
}

/// admin can change the admin address
#[receive(
    contract = "launchpad",
    name = "update_admin",
    error = "VestingError",
    parameter = "Admin",
    mutable
)]

fn update_admin<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
) -> VestingResult<()> {
    let sender = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(VestingError::ContractUser),
    };

    let param: Admin = ctx.parameter_cursor().get()?;

    let admin = host.state().admin.clone();

    ensure!(sender == admin, VestingError::InvalidUser);

    host.state_mut().admin = param;

    Ok(())
}

#[receive(
    contract = "launchpad",
    name = "live_pause",
    mutable,
    parameter = "LivePauseParam",
    error = "VestingError"
)]
fn live_pause<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
) -> VestingResult<()> {
    // Ensure that the sender is an account.
    match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(VestingError::ContractUser),
    };

    // parse paramaeter
    let launchpad_param: LivePauseParam = ctx.parameter_cursor().get()?;

    // clone state
    let launchpad_list = host.state().launchpad.clone();

    // get id
    let id = launchpad_param.id;

    // find by id and update launchpad
    if let Some(entry) = launchpad_list.get(&id) {
        let matched = entry;

        ensure!(matched.cancel == false, VestingError::LaunchpadNotExist);

        let sender = ctx.sender();

        ensure!(
            sender.matches_account(&matched.owner),
            VestingError::LaunchpadNotExist
        );

        ensure!(
            ctx.metadata().slot_time() < matched.end_time,
            VestingError::VestingFinished
        );

        let launchpad_data;
        // after launchpad pause can't be turn on before 12 hours
        if &matched.pause_until > &ctx.metadata().slot_time() {
            let pause_start_time: u64 = Timestamp::timestamp_millis(&matched.pause_start);
            let twelve_hour_millis: u64 = 43200000; // 12 hour
            let pause_period: Timestamp =
                Timestamp::from_timestamp_millis(pause_start_time + twelve_hour_millis);

            ensure!(
                ctx.metadata().slot_time() > pause_period,
                VestingError::LivePauseTimeRestricted
            );

            launchpad_data = Launchpad {
                start_time: matched.start_time,
                end_time: matched.end_time,
                soft_cap: matched.soft_cap,
                hard_cap: matched.hard_cap,
                owner: matched.owner,
                holders: matched.holders.clone(),
                invest_amount: matched.invest_amount,
                maximum_invest: matched.maximum_invest,
                minimum_invest: matched.invest_amount,
                cancel: matched.cancel,
                total_tx: matched.total_tx,
                live: true,
                dev_paid: matched.dev_paid,
                pause_until: ctx.metadata().slot_time(),
                pause_start: ctx.metadata().slot_time(),
                live_pause_count: matched.live_pause_count,
                cis2_amount: matched.cis2_amount,
                cis2_price: matched.cis2_price,
            };
        } else {
            ensure!(
                matched.live_pause_count <= 3,
                VestingError::LivePauseCycleCompleted
            );

            let now: u64 = Timestamp::timestamp_millis(&ctx.metadata().slot_time());
            let add_two_days = Timestamp::from_timestamp_millis(now + 172800000); // add 2 days millis

            launchpad_data = Launchpad {
                start_time: matched.start_time,
                end_time: matched.end_time,
                soft_cap: matched.soft_cap,
                hard_cap: matched.hard_cap,
                owner: matched.owner,
                holders: matched.holders.clone(),
                invest_amount: matched.invest_amount,
                maximum_invest: matched.maximum_invest,
                minimum_invest: matched.invest_amount,
                cancel: matched.cancel,
                total_tx: matched.total_tx,
                live: false,
                dev_paid: matched.dev_paid,
                pause_until: add_two_days,
                pause_start: ctx.metadata().slot_time(),
                live_pause_count: matched.live_pause_count + 1,
                cis2_amount: matched.cis2_amount,
                cis2_price: matched.cis2_price,
            };
        }
        host.state_mut().launchpad.insert(id, launchpad_data);
    } else {
        return Err(VestingError::LaunchpadNotExist);
    }
    // Insert or replace the vote for the account.
    Ok(())
}

#[receive(
    contract = "launchpad",
    name = "vest",
    mutable,
    payable,
    parameter = "VestParams",
    error = "VestingError"
)]
fn vest<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
    _amount: Amount,
) -> VestingResult<()> {
    // Ensure that the sender is an account.
    let acc = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(VestingError::ContractUser),
    };

    // parse param
    let params: VestParams = ctx.parameter_cursor().get()?;

    let id = params.launchpad_id;
    // clon state
    let launchpad_list = host.state().launchpad.clone();
    let lockup_details_list = host.state().lockup_details.clone();

    // find by id and update launchpad
    if let Some(entry) = launchpad_list.get(&id) {
        let matched = entry;

        let mut launchpad_holders_list = matched.holders.clone();

        if let Some(lockup) = lockup_details_list.get(&id) {
            let matched_lockup = lockup;

            let mut lockup_holder_list = matched_lockup.lockup_holders.clone();

            ensure!(matched.cancel == false, VestingError::LaunchpadCancelled);

            ensure!(
                ctx.metadata().slot_time() > matched.pause_until,
                VestingError::LaunchpadPaused
            );

            ensure!(
                ctx.metadata().slot_time() < matched.end_time,
                VestingError::VestingFinished
            );

            // find and update holder list or insert first entry
            if let Some(_hold) = launchpad_holders_list.get(&acc) {
                let matched_holder = _hold;

                // make sure invester amount must not be less than min and max than max
                ensure!(
                    _amount + *matched_holder >= matched.minimum_invest
                        && _amount + *matched_holder <= matched.maximum_invest,
                    VestingError::MinimumInvestmentNotSatisfied
                );

                // make sure launchpad doesnot reach to its hard_cap limit
                ensure!(
                    _amount + matched.invest_amount <= matched.hard_cap,
                    VestingError::HardcapLimitReached
                );

                launchpad_holders_list.insert(acc, _amount + *matched_holder);

                if let Some(_lockup_holder) = lockup_holder_list.get(&acc) {
                    let lockup_holder_ = _lockup_holder;
                    let insert_holder_data = LockupHolder {
                        cycle_completed: lockup_holder_.cycle_completed,
                        claimable_token: lockup_holder_.claimable_token + params.token_amount,
                        vested_date: ctx.metadata().slot_time(),
                    };
                    lockup_holder_list.insert(acc, insert_holder_data);
                }
            } else {
                // make sure invester amount must not be less than min and max than max
                ensure!(
                    _amount >= matched.minimum_invest && _amount <= matched.maximum_invest,
                    VestingError::MinimumInvestmentNotSatisfied
                );

                // make sure launchpad doesnot reach to its hard_cap limit
                ensure!(
                    _amount + matched.invest_amount <= matched.hard_cap,
                    VestingError::HardcapLimitReached
                );
                launchpad_holders_list.insert(acc, _amount);

                let insert_lockup_holder_data = LockupHolder {
                    cycle_completed: 0,
                    claimable_token: params.token_amount,
                    vested_date: ctx.metadata().slot_time(),
                };

                lockup_holder_list.insert(acc, insert_lockup_holder_data);
            }

            ensure!(
                matched.invest_amount != matched.hard_cap,
                VestingError::HardcapLimitReached
            );

            // this will execute when hard cap limit will reach and automaticaly cliff period will start
            if matched.hard_cap == _amount + matched.invest_amount {
                let now = Timestamp::timestamp_millis(&ctx.metadata().slot_time());
                let add_duration: Timestamp =
                    Timestamp::from_timestamp_millis(now + matched_lockup.cliff_duration);

                let cloned_release_details = matched_lockup.token_release_data.clone();
                let mut update_release_details = matched_lockup.token_release_data.clone();

                // check if the hard cap completed and end time didn't complete so token release dates will re arrange
                if ctx.metadata().slot_time() < matched.end_time {
                    let now = Timestamp::timestamp_millis(&ctx.metadata().slot_time());
                    let end_time_millis = Timestamp::timestamp_millis(&matched.end_time);
                    // for example current date15 june - 10 days is end date == 10 days millis
                    // so this 10 days will subtract from each releae date cycle
                    let sub_duration = end_time_millis - now;

                    for (k, v) in cloned_release_details.iter() {
                        let current_release_date_millis =
                            Timestamp::timestamp_millis(&v.release_time);

                        let update = ReleaseData {
                            release_time: Timestamp::from_timestamp_millis(
                                current_release_date_millis - sub_duration,
                            ),
                            per_cycle_release: v.per_cycle_release,
                        };
                        update_release_details.insert(*k, update);
                    }
                }

                let lockup_detail = LockupDetails {
                    cliff_period: add_duration,
                    token_release_data: update_release_details.clone(),
                    lockup_holders: lockup_holder_list.clone(),
                    cliff_duration: matched_lockup.cliff_duration,
                };
                host.state_mut().lockup_details.insert(id, lockup_detail);
            } else {
                let lockup_detail = LockupDetails {
                    cliff_period: matched_lockup.cliff_period,
                    token_release_data: matched_lockup.token_release_data.clone(),
                    lockup_holders: lockup_holder_list.clone(),
                    cliff_duration: matched_lockup.cliff_duration,
                };
                host.state_mut().lockup_details.insert(id, lockup_detail);
            }

            let launchpad_data = Launchpad {
                start_time: matched.start_time,
                end_time: matched.end_time,
                soft_cap: matched.soft_cap,
                hard_cap: matched.hard_cap,
                live: matched.live,
                invest_amount: _amount + matched.invest_amount,
                owner: matched.owner,
                holders: launchpad_holders_list.clone(),
                cancel: matched.cancel,
                minimum_invest: matched.minimum_invest,
                maximum_invest: matched.maximum_invest,
                total_tx: matched.total_tx + 1,
                dev_paid: matched.dev_paid,
                pause_until: matched.pause_until,
                live_pause_count: matched.live_pause_count,
                pause_start: matched.pause_start,
                cis2_amount: matched.cis2_amount,
                cis2_price: matched.cis2_price,
            };

            host.state_mut().launchpad.insert(id, launchpad_data);
        }
    } else {
        return Err(VestingError::LaunchpadNotExist);
    }
    // Insert or replace the vote for the account.

    Ok(())
}

#[receive(
    contract = "launchpad",
    name = "retrieve",
    mutable,
    parameter = "LaunchpadID",
    error = "VestingError"
)]
fn retrieve<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
) -> VestingResult<()> {
    // Ensure that the sender is an account.
    let acc = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(VestingError::ContractUser.into()),
    };

    // parse param
    let id: LaunchpadID = ctx.parameter_cursor().get()?;

    // clon state
    let launchpad_list = host.state().launchpad.clone();

    if let Some(entry) = launchpad_list.get(&id) {
        let matched = entry;

        let mut update_holders = matched.holders.clone();

        if let Some(hold) = matched.holders.get(&acc) {
            let amount_ = hold;

            ensure!(matched.cancel == false, VestingError::LaunchpadNotExist);

            ensure!(
                ctx.metadata().slot_time() < matched.end_time,
                VestingError::VestingFinished
            );

            update_holders.remove(&acc);

            let launchpad_data = Launchpad {
                start_time: matched.start_time,
                end_time: matched.end_time,
                soft_cap: matched.soft_cap,
                hard_cap: matched.hard_cap,
                live: matched.live,
                owner: matched.owner,
                cancel: matched.cancel,
                holders: update_holders.clone(),
                invest_amount: matched.invest_amount - *amount_,
                minimum_invest: matched.minimum_invest,
                maximum_invest: matched.maximum_invest,
                total_tx: matched.total_tx,
                dev_paid: matched.dev_paid,
                pause_until: matched.pause_until,
                pause_start: matched.pause_start,
                live_pause_count: matched.live_pause_count,
                cis2_amount: matched.cis2_amount,
                cis2_price: matched.cis2_price,
            };
            host.state_mut().launchpad.insert(id, launchpad_data);
            host.invoke_transfer(&acc, *amount_)
                .map_err(|_| VestingError::InvalidUser)?;
        } else {
            return Err(VestingError::UserNotExist.into());
        }
    } else {
        return Err(VestingError::InvalidUser.into());
    }

    Ok(())
}

#[receive(
    contract = "launchpad",
    name = "cancel",
    mutable,
    parameter = "CancelParam",
    error = "VestingError"
)]
fn cancel<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
) -> VestingResult<()> {
    let sender_address = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(VestingError::ContractUser),
    };

    // parse param
    let params: CancelParam = ctx.parameter_cursor().get()?;

    // clon state
    let launchpad_list = host.state().launchpad.clone();
    let admin = host.state().admin.clone();

    // find by id and update launchpad
    if let Some(entry) = launchpad_list.get(&params.launchpad_id) {
        let matched = entry;

        let sender = ctx.sender();

        ensure!(
            sender.matches_account(&matched.owner),
            VestingError::LaunchpadNotExist
        );

        ensure!(matched.cancel == false, VestingError::LaunchpadNotExist);

        ensure!(
            ctx.metadata().slot_time() < matched.end_time,
            VestingError::VestingFinished
        );

        ensure!(
            matched.invest_amount < matched.hard_cap,
            VestingError::VestingFinished
        );

        for (account, amt) in &matched.holders {
            host.invoke_transfer(&account, *amt)
                .map_err(|_| VestingError::InsuffiecientFunds)?;
        }

        host.invoke_transfer(&admin, matched.dev_paid)
            .map_err(|_| VestingError::InsuffiecientFunds)?;

        let token_reserve = get_token_reserve(ctx, host, &params.token.clone())?;

        crate::cis2_client::Cis2Client::transfer(
            host,
            params.token.id.clone(),
            params.token.address,
            ContractTokenAmount::from(token_reserve),
            Address::Contract(ctx.self_address()),
            Receiver::Account(sender_address),
        )?;

        let empty_list = BTreeMap::new();

        let launchpad_data = Launchpad {
            start_time: matched.start_time,
            end_time: matched.end_time,
            soft_cap: matched.soft_cap,
            hard_cap: matched.hard_cap,
            live: matched.live,
            invest_amount: matched.invest_amount,
            owner: matched.owner,
            holders: empty_list,
            cancel: true,
            minimum_invest: matched.minimum_invest,
            maximum_invest: matched.maximum_invest,
            total_tx: matched.total_tx,
            dev_paid: matched.dev_paid,
            pause_until: matched.pause_until,
            live_pause_count: matched.live_pause_count,
            pause_start: matched.pause_start,
            cis2_amount: matched.cis2_amount,
            cis2_price: matched.cis2_price,
        };

        host.state_mut()
            .launchpad
            .insert(params.launchpad_id, launchpad_data);
    } else {
        return Err(VestingError::LaunchpadNotExist);
    }

    Ok(())
}

/// after ending vesting time the CCD invested amount and the dev payment to platform that is 5 % will be send to dev wallet
#[receive(
    contract = "launchpad",
    name = "send_ccd_to_dev",
    mutable,
    parameter = "WithdrawParam",
    error = "VestingError"
)]
fn send_ccd_to_dev<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
) -> VestingResult<()> {
    match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(VestingError::ContractUser),
    };

    let params: WithdrawParam = ctx.parameter_cursor().get()?;

    let launchpad_list: BTreeMap<u64, Launchpad> = host.state().launchpad.clone();

    // find by id and update launchpad
    if let Some(entry) = launchpad_list.get(&params.launchpad_id) {
        let matched: &Launchpad = entry;

        ensure!(matched.cancel == false, VestingError::VestingFinished);

        ensure!(
            ctx.metadata().slot_time() > matched.end_time
                || matched.hard_cap == matched.invest_amount,
            VestingError::VestingFinished
        );

        // check if the hard cap didn't reach so remaining cis2 must return to developer wallet
        if matched.hard_cap < matched.invest_amount {
            crate::cis2_client::Cis2Client::transfer(
                host,
                params.token.id.clone(),
                params.token.address,
                ContractTokenAmount::from(params.remaining_cis2_amount),
                Address::Contract(ctx.self_address()),
                Receiver::Account(matched.owner),
            )?;
        }

        let launchpad_data = Launchpad {
            start_time: matched.start_time,
            end_time: matched.end_time,
            soft_cap: matched.soft_cap,
            hard_cap: matched.hard_cap,
            live: matched.live,
            invest_amount: Amount { micro_ccd: 0 },
            owner: matched.owner,
            holders: matched.holders.clone(),
            cancel: matched.cancel,
            minimum_invest: matched.minimum_invest,
            maximum_invest: matched.maximum_invest,
            total_tx: matched.total_tx,
            dev_paid: Amount { micro_ccd: 0 },
            pause_until: matched.pause_until,
            live_pause_count: matched.live_pause_count,
            pause_start: matched.pause_start,
            cis2_amount: matched.cis2_amount,
            cis2_price: matched.cis2_price,
        };
        host.invoke_transfer(&matched.owner, matched.invest_amount + matched.dev_paid)
            .map_err(|_| VestingError::InsuffiecientFunds)?;
        host.state_mut()
            .launchpad
            .insert(params.launchpad_id, launchpad_data);
    } else {
        return Err(VestingError::LaunchpadNotExist);
    }

    Ok(())
}

/// this function will be applicable after vesting and cliff time end. After each cycle completion holders can claim token
#[receive(
    contract = "launchpad",
    name = "claim",
    mutable,
    parameter = "ClaimParams",
    error = "VestingError"
)]
fn claim<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
) -> VestingResult<()> {
    let sender_address = match ctx.sender() {
        Address::Account(acc) => acc,
        Address::Contract(_) => return Err(VestingError::ContractUser),
    };

    let params: ClaimParams = ctx
        .parameter_cursor()
        .get()
        .map_err(|_e: ParseError| VestingError::ParseParams)?;

    let lockup_list: BTreeMap<u64, LockupDetails> = host.state().lockup_details.clone();
    let launchpad_list: BTreeMap<u64, Launchpad> = host.state().launchpad.clone();

    if let Some(_launchpad) = launchpad_list.get(&params.launchpad_id) {
        let matched_launchpad = _launchpad;

        // check launchpad cancel or not
        ensure!(
            matched_launchpad.cancel == false,
            VestingError::VestingFinished
        );

        // check launchpad finalized or not
        ensure!(
            ctx.metadata().slot_time() > matched_launchpad.end_time
                || matched_launchpad.hard_cap == matched_launchpad.invest_amount,
            VestingError::LaunchpadNotEnd
        );

        if let Some(_lockup) = lockup_list.get(&params.launchpad_id) {
            let lockup_detials_matched = _lockup;

            // check cliff period complete or not
            ensure!(
                ctx.metadata().slot_time() > lockup_detials_matched.cliff_period,
                VestingError::CliffPeriodNotEnd
            );

            let release_data_ = lockup_detials_matched.token_release_data.clone();

            if let Some(_release) = release_data_.get(&params.epoch_cycle) {
                let release_detials_matched = _release;

                // claim time must be complete for each cycle
                ensure!(
                    ctx.metadata().slot_time() > release_detials_matched.release_time,
                    VestingError::CannotClaim
                );

                if let Some(lockup_holder_) =
                    lockup_detials_matched.lockup_holders.get(&sender_address)
                {
                    let _lockup_holder = lockup_holder_;

                    // holders must not to claim more than 1 time
                    ensure!(
                        _lockup_holder.cycle_completed < params.epoch_cycle,
                        VestingError::CannotClaim
                    );

                    let mut _holder = lockup_detials_matched.lockup_holders.clone();

                    let update_holder = LockupHolder {
                        claimable_token: _lockup_holder.claimable_token,
                        cycle_completed: _lockup_holder.cycle_completed + 1,
                        vested_date: _lockup_holder.vested_date,
                    };

                    _holder.insert(sender_address, update_holder);

                    let launchpad_data = LockupDetails {
                        cliff_period: lockup_detials_matched.cliff_period,
                        token_release_data: lockup_detials_matched.token_release_data.clone(),
                        lockup_holders: _holder.clone(),
                        cliff_duration: lockup_detials_matched.cliff_duration,
                    };

                    host.state_mut()
                        .lockup_details
                        .insert(params.launchpad_id, launchpad_data);

                    // get the holders ccd token and calculate the token and pay to holders on claim time

                    let calculate_claim_token_amount = _lockup_holder.claimable_token
                        * release_detials_matched.per_cycle_release
                        / 100;

                    crate::cis2_client::Cis2Client::transfer(
                        host,
                        params.id.clone(),
                        params.address,
                        ContractTokenAmount::from(calculate_claim_token_amount),
                        Address::Contract(ctx.self_address()),
                        Receiver::Account(sender_address),
                    )?;
                }
            }
        }
    }

    Ok(())
}

#[receive(contract = "launchpad", name = "view", return_value = "VestingView")]
fn view<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State, StateApiType = S>,
) -> ReceiveResult<VestingView> {
    let total_launchpad = host.state().total_launchpad.clone();
    let launchpad = host.state().launchpad.clone();
    let lockup_details = host.state().lockup_details.clone();

    Ok(VestingView {
        total_launchpad,
        launchpad,
        lockup_details,
    })
}
