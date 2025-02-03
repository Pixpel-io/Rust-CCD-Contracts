use concordium_std::{receive, Host, ReceiveContext, *};

use crate::{
    error::{ContractResult, Error},
    params::ReturnParamView,
    state::{AuctionState, ItemState, State},
};

/// View function that returns the content of the state.
#[receive(
    contract = "cis2-auction",
    name = "view",
    return_value = "ReturnParamView"
)]
fn view(_ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<ReturnParamView> {
    let state = host.state();

    let inner_state = state.items.iter().map(|x| (*x.0, x.1.clone())).collect();

    Ok(ReturnParamView {
        item_states: inner_state,
        counter: host.state().counter,
    })
}

/// ViewItemState function that returns the state of a specific item.
#[receive(
    contract = "cis2-auction",
    name = "viewItemState",
    return_value = "ItemState",
    parameter = "u16",
    error = "Error"
)]
fn view_item_state(ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<ItemState> {
    // Getting input parameter.
    let item_index: u16 = ctx.parameter_cursor().get()?;
    let item = host
        .state()
        .items
        .get(&item_index)
        .map(|x| x.to_owned())
        .ok_or(Error::NoItem)?;
    Ok(item)
}

/// ViewItemState function that returns the state of a specific item.
#[receive(
    contract = "cis2-auction",
    name = "viewActive",
    return_value = "ReturnParamView"
)]
fn view_active(_ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<ReturnParamView> {
    // Getting input parameter.
    let inner = get_items(host, AuctionState::NotSoldYet);

    let count = inner.iter().count() as u16;

    Ok(ReturnParamView {
        item_states: inner,
        counter: count,
    })
}

/// ViewItemState function that returns the state of a specific item.
#[receive(
    contract = "cis2-auction",
    name = "viewCanceled",
    return_value = "ReturnParamView"
)]
fn view_canceled(_ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<ReturnParamView> {
    // Getting input parameter.
    let inner = get_items(host, AuctionState::Canceled);

    let count = inner.iter().count() as u16;

    Ok(ReturnParamView {
        item_states: inner,
        counter: count,
    })
}

/// ViewItemState function that returns the state of a specific item.
#[receive(
    contract = "cis2-auction",
    name = "viewFinalized",
    return_value = "ReturnParamView"
)]
fn view_finalized(_ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<ReturnParamView> {
    // Getting input parameter.
    let inner = host
        .state()
        .items
        .iter()
        .filter(|(_, item)| {
            item.auction_state != AuctionState::NotSoldYet
                && item.auction_state != AuctionState::Canceled
        })
        .map(|(index, item)| (*index, item.clone()))
        .collect::<Vec<_>>();

    let count = inner.iter().count() as u16;

    Ok(ReturnParamView {
        item_states: inner,
        counter: count,
    })
}

fn get_items(host: &Host<State>, state: AuctionState) -> Vec<(u16, ItemState)> {
    let inner = host
        .state()
        .items
        .iter()
        .filter(|(_, item)| item.auction_state == state)
        .map(|(index, item)| (*index, item.clone()))
        .collect::<Vec<_>>();

    inner
}
