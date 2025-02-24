use concordium_std::*;
use concordium_cis2::*;

use crate::state::*;
use crate::params::*;
use crate::errors::*;
use crate::events::*;
use crate::responses::*;
use crate::types::*;
use crate::cis2_client::Cis2Client;


const TOKEN_METADATA_BASE_URL: &str = "https://concordium-servernode.dev-site.space/api/v1/metadata/swap/lp-tokens?";

const SUPPORTS_STANDARDS: [StandardIdentifier<'static>; 2] =
    [CIS0_STANDARD_IDENTIFIER, CIS2_STANDARD_IDENTIFIER];

pub const FEE_MULTIPLIER: u128 = 10000;
pub const FEE: u128 = 100; // 1%


#[init(
    contract = "pixpel_swap"
)]
pub fn init<S: HasStateApi>(
    _ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    Ok(State::empty(state_builder))
}


fn get_token_reserve<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    token_info: &TokenInfo,
) -> ContractResult<ContractTokenAmount> {
    Cis2Client::get_balance(
        host,
        token_info.id.clone(),
        &token_info.address,
        Address::Contract(ctx.self_address()),
    )
}

fn get_token_reserve_safe<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    token_info: &TokenInfo,
) -> ContractTokenAmount {
    match get_token_reserve(ctx, host, token_info) {
        Ok(value) => value,
        Err(_e) => ContractTokenAmount::default(),
    }
}


#[receive(
    contract = "pixpel_swap",
    name = "view",
    return_value = "StateView",
    error = "ContractError",
    mutable
)]
fn contract_view<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<StateView> {
    let state = host.state();

    let mut exchanges = Vec::new();
    for (ex, ex_state) in state.exchanges.iter() {
        exchanges.push(ExchangeStateView {
            token_info: ex.clone(),
            exchange_state: ExchangeState {
                lp_token_id: ex_state.lp_token_id,
                ccd_balance: ex_state.ccd_balance,
            },
            token_balance: 0.into(),
        });
    }

    let mut lp_tokens_state = Vec::new();
    for (addr, addr_state) in state.lp_tokens_state.iter() {
        let mut balances = Vec::new();
        let mut operators = Vec::new();
        for (token_id, amount) in addr_state.balances.iter() {
            balances.push((*token_id, *amount));
        }
        for o in addr_state.operators.iter() {
            operators.push(*o);
        }
        lp_tokens_state.push((*addr, AddressStateView {
            balances,
            operators,
        }));
    }

    let mut lp_tokens_supply = Vec::new();
    for (addr, addr_supply) in state.lp_tokens_supply.iter() {
        lp_tokens_supply.push((*addr, *addr_supply));
    }

    let last_lp_token_id = state.last_lp_token_id;

    for ex in &mut exchanges {
        ex.token_balance = get_token_reserve_safe(ctx, host,&ex.token_info.clone());
    }

    Ok(StateView {
        exchanges,
        lp_tokens_state,
        lp_tokens_supply,
        last_lp_token_id,
        contract_ccd_balance: host.self_balance(),
    })
}


// Exchanges

#[receive(
    contract = "pixpel_swap",
    name = "getExchange",
    parameter = "GetExchangeParams",
    return_value = "ExchangeView",
    error = "ContractError",
    mutable
)]
pub fn get_exchange<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType=S>,
) -> ContractResult<ExchangeView> {
    let params: GetExchangeParams = ctx.parameter_cursor().get()?;
    let mut viewstate = host.state().get_exchange_view(&params.token, &params.holder)?;
    viewstate.token_balance = get_token_reserve_safe(ctx, host, &params.token);
    Ok(viewstate)
}


#[receive(
    contract = "pixpel_swap",
    name = "getExchanges",
    parameter = "GetExchangesParams",
    return_value = "ExchangesView",
    error = "ContractError",
    mutable
)]
pub fn get_exchanges<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType=S>,
) -> ContractResult<ExchangesView> {
    let params: GetExchangesParams = ctx.parameter_cursor().get()?;
    let state = host.state();

    let mut exchanges = Vec::new();
    for (token_info, _) in state.exchanges.iter() {
        exchanges.push(
            state.get_exchange_view(&token_info, &params.holder)?
        );
    }

    for ex in &mut exchanges {
        ex.token_balance = get_token_reserve_safe(ctx, host,&ex.token.clone());
    }

    Ok( ExchangesView {
        exchanges,
    })
}


// LP tokens

#[receive(
    contract = "pixpel_swap",
    name = "transfer",
    parameter = "TransferParameter",
    error = "ContractError",
    enable_logger,
    mutable
)]
pub fn lpt_transfer<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let TransferParams(transfers): TransferParameter = ctx.parameter_cursor().get()?;
    let sender = ctx.sender();

    for Transfer {
        token_id,
        amount,
        from,
        to,
        data,
    } in transfers
    {
        let (state, builder) = host.state_and_builder();
        ensure!(from == sender || state.is_operator(&sender, &from), ContractError::Unauthorized);
        let to_address = to.address();
        state.transfer(&token_id, amount, &from, &to_address, builder)?;

        logger.log(&Cis2Event::Transfer(TransferEvent {
            token_id,
            amount,
            from,
            to: to_address,
        }))?;

        if let Receiver::Contract(address, entrypoint_name) = to {
            let parameter = OnReceivingCis2Params {
                token_id,
                amount,
                from,
                data,
            };
            host.invoke_contract(
                &address,
                &parameter,
                entrypoint_name.as_entrypoint_name(),
                Amount::zero(),
            )?;
        }
    }
    Ok(())
}


#[receive(
    contract = "pixpel_swap",
    name = "updateOperator",
    parameter = "UpdateOperatorParams",
    error = "ContractError",
    enable_logger,
    mutable
)]
pub fn lpt_update_operator<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let UpdateOperatorParams(params) = ctx.parameter_cursor().get()?;
    let sender = ctx.sender();

    let (state, builder) = host.state_and_builder();
    for param in params {
        match param.update {
            OperatorUpdate::Add => state.add_operator(&sender, &param.operator, builder),
            OperatorUpdate::Remove => state.remove_operator(&sender, &param.operator),
        }

        logger.log(&Cis2Event::<ContractTokenId, ContractTokenAmount>::UpdateOperator(
            UpdateOperatorEvent {
                owner:    sender,
                operator: param.operator,
                update:   param.update,
            },
        ))?;
    }
    Ok(())
}


#[receive(
    contract = "pixpel_swap",
    name = "balanceOf",
    parameter = "ContractBalanceOfQueryParams",
    return_value = "ContractBalanceOfQueryResponse",
    error = "ContractError"
)]
pub fn lpt_balance_of<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ContractBalanceOfQueryResponse> {
    let params: ContractBalanceOfQueryParams = ctx.parameter_cursor().get()?;
    let mut response = Vec::with_capacity(params.queries.len());
    for query in params.queries {
        let amount = host.state().balance(&query.token_id, &query.address)?;
        response.push(amount);
    }
    let result = ContractBalanceOfQueryResponse::from(response);
    Ok(result)
}


#[receive(
    contract = "pixpel_swap",
    name = "operatorOf",
    parameter = "OperatorOfQueryParams",
    return_value = "OperatorOfQueryResponse",
    error = "ContractError"
)]
pub fn lpt_operator_of<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<OperatorOfQueryResponse> {
    let params: OperatorOfQueryParams = ctx.parameter_cursor().get()?;
    let mut response = Vec::with_capacity(params.queries.len());
    for query in params.queries {
        let is_operator = host.state().is_operator(&query.address, &query.owner);
        response.push(is_operator);
    }
    let result = OperatorOfQueryResponse::from(response);
    Ok(result)
}


fn build_token_metadata_url(
    token_info: &TokenInfo,
) -> String {
    let mut token_metadata_url = String::from(TOKEN_METADATA_BASE_URL);
    token_metadata_url.push_str("contract_index=");
    token_metadata_url.push_str(&token_info.address.index.to_string());
    token_metadata_url.push_str("&token_id=");
    token_metadata_url.push_str(&token_info.id.to_string());
    token_metadata_url
}


#[receive(
    contract = "pixpel_swap",
    name = "tokenMetadata",
    parameter = "ContractTokenMetadataQueryParams",
    return_value = "TokenMetadataQueryResponse",
    error = "ContractError"
)]
fn lpt_token_metadata<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<TokenMetadataQueryResponse> {
    let params: ContractTokenMetadataQueryParams = ctx.parameter_cursor().get()?;
    let mut response = Vec::with_capacity(params.queries.len());
    for token_id in params.queries {
        ensure!(host.state().contains_token(&token_id), ContractError::InvalidTokenId);
        let token_info = host.state().get_token_info_by_lp_token_id(&token_id)?;
        let metadata_url = MetadataUrl {
            url:  build_token_metadata_url(&token_info),
            hash: None,
        };
        response.push(metadata_url);
    }
    let result = TokenMetadataQueryResponse::from(response);
    Ok(result)
}


#[receive(
    contract = "pixpel_swap",
    name = "onReceivingCIS2",
    error = "ContractError"
)]
fn lpt_on_cis2_received<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    _host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    Ok(())
}


#[receive(
    contract = "pixpel_swap",
    name = "supports",
    parameter = "SupportsQueryParams",
    return_value = "SupportsQueryResponse",
    error = "ContractError"
)]
fn lpt_supports<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    _host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<SupportsQueryResponse> {
    let params: SupportsQueryParams = ctx.parameter_cursor().get()?;
    let mut response = Vec::with_capacity(params.queries.len());
    for std_id in params.queries {
        if SUPPORTS_STANDARDS.contains(&std_id.as_standard_identifier()) {
            response.push(SupportResult::Support);
        } else {
            response.push(SupportResult::NoSupport);
        }
    }
    let result = SupportsQueryResponse::from(response);
    Ok(result)
}


// Liquidity pools

#[receive(
    contract = "pixpel_swap",
    name = "addLiquidity",
    parameter = "AddLiquidityParams",
    error = "ContractError",
    enable_logger,
    mutable,
    payable
)]
pub fn lp_add_liquidity<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    amount: Amount,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let params: AddLiquidityParams = ctx.parameter_cursor().get()?;
    let state = host.state_mut();

    if state.exchanges.get(&params.token).is_none() {
        state.create_exchange(&params.token)?;
    }

    let exchange_state = match state.exchanges.get(&params.token) {
        None => bail!(ContractError::ExchangeNotFound),
        Some(exchange_state) => exchange_state
    };
    let lp_token_id = exchange_state.lp_token_id;
    let token_supply_micro= match state.lp_tokens_supply.get(&lp_token_id) {
        Some(token_supply) => token_supply.0,
        _ => 0,
    };

    let mut tokens_for_minting_micro: u64 = amount.micro_ccd;

    if token_supply_micro > 0 {
        let ccd_reserve = exchange_state.ccd_balance;
        let token_reserve = get_token_reserve(ctx, host,&params.token.clone())?;
        let correct_token_amount_micro = (amount.micro_ccd as u128 * token_reserve.0 as u128 / ccd_reserve.0 as u128) as u64;

        ensure!(
            params.token_amount.0 >= correct_token_amount_micro,
            ContractError::IncorrectTokenCcdRatio
        );

        tokens_for_minting_micro = (amount.micro_ccd as u128 * token_supply_micro as u128 / ccd_reserve.0 as u128) as u64;
    }

    let supports_cis2 = Cis2Client::supports_cis2(host, &params.token.address)?;
    ensure!(supports_cis2, ContractError::TokenNotCis2);

    let is_operator = Cis2Client::is_operator_of(
        host,
        ctx.sender(),
        ctx.self_address(),
        &params.token.address,
    )?;
    ensure!(is_operator, ContractError::NotOperator);

    Cis2Client::transfer(
        host,
        params.token.id.clone(),
        params.token.address,
        params.token_amount,
        ctx.sender(),
        Receiver::Contract(
            ctx.self_address(),
            OwnedEntrypointName::new_unchecked("onReceivingCIS2".to_string())
        ),
    )?;

    let (state, builder) = host.state_and_builder();
    state.mint(
        &lp_token_id,
        tokens_for_minting_micro.into(),
        &ctx.sender(),
        builder,
    );

    logger.log(&Cis2Event::Mint(MintEvent::<ContractTokenId, ContractTokenAmount> {
        token_id: lp_token_id,
        amount: tokens_for_minting_micro.into(),
        owner: ctx.sender(),
    }))?;

    logger.log(&Cis2Event::TokenMetadata::<_, ContractTokenAmount>(TokenMetadataEvent {
        token_id: lp_token_id,
        metadata_url: MetadataUrl {
            url:  build_token_metadata_url(&params.token),
            hash: None,
        },
    }))?;

    state.increase_exchange_ccd_balance(&params.token, amount.micro_ccd().into())?;

    Ok(())
}


#[receive(
    contract = "pixpel_swap",
    name = "removeLiquidity",
    parameter = "RemoveLiquidityParams",
    error = "ContractError",
    enable_logger,
    mutable
)]
pub fn lp_remove_liquidity<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let params: RemoveLiquidityParams = ctx.parameter_cursor().get()?;
    let state = host.state();

    let sender_address = match ctx.sender() {
        Address::Contract(_) => bail!(ContractError::CalledByAContract),
        Address::Account(account_address) => account_address,
    };
    let exchange_state = match state.exchanges.get(&params.token) {
        None => bail!(ContractError::ExchangeNotFound),
        Some(exchange_state) => exchange_state
    };
    let lp_token_id = exchange_state.lp_token_id;
    let ccd_reserve = exchange_state.ccd_balance;
    let token_supply_micro= match state.lp_tokens_supply.get(&lp_token_id) {
        Some(token_supply) => token_supply.0,
        _ => 0,
    };

    let token_reserve = get_token_reserve(ctx, host,&params.token.clone())?;

    let ccd_amount_micro = ((ccd_reserve.0 as u128 * params.lp_token_amount.0 as u128) / token_supply_micro as u128) as u64;
    let token_amount_micro = ((token_reserve.0 as u128 * params.lp_token_amount.0 as u128) / token_supply_micro as u128) as u64;

    Cis2Client::transfer(
        host,
        params.token.id.clone(),
        params.token.address,
        ContractTokenAmount::from(token_amount_micro),
        Address::Contract(ctx.self_address()),
        Receiver::Account(sender_address),
    )?;

    host.invoke_transfer(
        &sender_address,
        Amount::from_micro_ccd(ccd_amount_micro)
    ).map_err(|_| ContractError::InvokeTransferError)?;

    let (state, builder) = host.state_and_builder();
    state.burn(
        &lp_token_id,
        params.lp_token_amount,
        &ctx.sender(),
        builder,
    )?;

    logger.log(&Cis2Event::Burn(BurnEvent::<ContractTokenId, ContractTokenAmount> {
        token_id: lp_token_id,
        amount: params.lp_token_amount,
        owner: ctx.sender(),
    }))?;

    state.decrease_exchange_ccd_balance(&params.token, ccd_amount_micro.into())?;

    Ok(())
}


// Swaps

fn get_output_amount(
    input_amount: u64,
    input_reserve: u64,
    output_reserve: u64,
) -> ContractResult<u64> {
    ensure!(
        (input_reserve > 0) && (output_reserve > 0),
        ContractError::InvalidReserves
    );

    let input_amount_with_fee = input_amount as u128 * (FEE_MULTIPLIER - FEE);
    let numerator = input_amount_with_fee * output_reserve as u128;
    let denominator = (input_reserve as u128 * FEE_MULTIPLIER) + input_amount_with_fee;
    Ok((numerator / denominator) as u64)
}


#[receive(
    contract = "pixpel_swap",
    name = "getCcdToTokenSwapAmount",
    parameter = "GetCcdToTokenSwapAmountParams",
    return_value = "SwapAmountResponse",
    error = "ContractError",
    mutable,
)]
fn get_ccd_to_token_swap_amount<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<SwapAmountResponse> {
    let params: GetCcdToTokenSwapAmountParams = ctx.parameter_cursor().get()?;

    let token_reserve: u64 = get_token_reserve(ctx, host, &params.token)?.into();
    let ccd_reserve: u64 = host.state().get_exchange_ccd_balance(&params.token)?.into();
    let tokens_bought = get_output_amount(
      params.ccd_sold.0,
      ccd_reserve,
      token_reserve
    )?;

    Ok( SwapAmountResponse {
        amount: tokens_bought.into(),
    })
}


#[receive(
    contract = "pixpel_swap",
    name = "getTokenToCcdSwapAmount",
    parameter = "GetTokenToCcdSwapAmountParams",
    return_value = "SwapAmountResponse",
    error = "ContractError",
    mutable,
)]
fn get_token_to_ccd_swap_amount<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<SwapAmountResponse> {
    let params: GetTokenToCcdSwapAmountParams = ctx.parameter_cursor().get()?;

    let token_reserve: u64 = get_token_reserve(ctx, host, &params.token)?.into();
    let ccd_reserve: u64 = host.state().get_exchange_ccd_balance(&params.token)?.into();
    let ccd_bought = get_output_amount(
      params.token_sold.0,
      token_reserve,
      ccd_reserve
    )?;

    Ok( SwapAmountResponse {
        amount: ccd_bought.into(),
    })
}


#[receive(
    contract = "pixpel_swap",
    name = "getTokenToTokenSwapAmount",
    parameter = "GetTokenToTokenSwapAmountParams",
    return_value = "SwapAmountResponse",
    error = "ContractError",
    mutable,
)]
fn get_token_to_token_swap_amount<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<SwapAmountResponse> {
    let params: GetTokenToTokenSwapAmountParams = ctx.parameter_cursor().get()?;

    let token_reserve: u64 = get_token_reserve(ctx, host, &params.token)?.into();
    let ccd_reserve: u64 = host.state().get_exchange_ccd_balance(&params.token)?.into();
    let ccd_bought = get_output_amount(
      params.token_sold.0,
      token_reserve,
      ccd_reserve
    )?;

    let purchased_token_reserve: u64 = get_token_reserve(ctx, host, &params.purchased_token)?.into();
    let purchased_ccd_reserve: u64 = host.state().get_exchange_ccd_balance(&params.purchased_token)?.into();
    let purchased_tokens_bought = get_output_amount(
      ccd_bought,
      purchased_ccd_reserve,
      purchased_token_reserve
    )?;

    Ok( SwapAmountResponse {
        amount: purchased_tokens_bought.into(),
    })
}


#[receive(
    contract = "pixpel_swap",
    name = "ccdToTokenSwap",
    parameter = "CcdToTokenSwapParams",
    error = "ContractError",
    enable_logger,
    mutable,
    payable
)]
pub fn ccd_to_token_swap<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    amount: Amount,
    logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let params: CcdToTokenSwapParams = ctx.parameter_cursor().get()?;

    let sender_address = match ctx.sender() {
        Address::Contract(_) => bail!(ContractError::CalledByAContract),
        Address::Account(account_address) => account_address,
    };

    let token_reserve: u64 = get_token_reserve(ctx, host,&params.token)?.into();
    let ccd_reserve: u64 = host.state().get_exchange_ccd_balance(&params.token)?.into();
    let tokens_bought = get_output_amount(
      amount.micro_ccd,
      ccd_reserve,
      token_reserve
    )?;

    ensure!(tokens_bought >= params.min_token_amount.0, ContractError::InsufficientOutputAmount);

    Cis2Client::transfer(
        host,
        params.token.id.clone(),
        params.token.address,
        ContractTokenAmount::from(tokens_bought),
        Address::Contract(ctx.self_address()),
        Receiver::Account(sender_address),
    )?;

    host.state_mut().increase_exchange_ccd_balance(&params.token, amount.micro_ccd.into())?;

    let timestamp = ctx.metadata().slot_time();
    logger.log(&Event::Swap(SwapEvent {
        client: ctx.sender(),
        action: SwapEventAction::BuyToken,
        double_swap: false,
        token: params.token,
        ccd_amount: amount.micro_ccd.into(),
        token_amount: tokens_bought.into(),
        ccd_reserve: ccd_reserve.into(),
        token_reserve: token_reserve.into(),
        timestamp,
    }))?;

    Ok(())
}


#[receive(
    contract = "pixpel_swap",
    name = "tokenToCcdSwap",
    parameter = "TokenToCcdSwapParams",
    error = "ContractError",
    enable_logger,
    mutable,
)]
pub fn token_to_ccd_swap<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    _logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let params: TokenToCcdSwapParams = ctx.parameter_cursor().get()?;
    let sender_address = match ctx.sender() {
        Address::Contract(_) => bail!(ContractError::CalledByAContract),
        Address::Account(account_address) => account_address,
    };

    let is_operator = Cis2Client::is_operator_of(
        host,
        ctx.sender(),
        ctx.self_address(),
        &params.token.address,
    )?;
    ensure!(is_operator, ContractError::NotOperator);

    let token_reserve: u64 = get_token_reserve(ctx, host, &params.token)?.into();
    let ccd_reserve: u64 = host.state().get_exchange_ccd_balance(&params.token)?.into();
    let ccd_bought = get_output_amount(
      params.token_sold.0,
      token_reserve,
      ccd_reserve
    )?;

    ensure!(ccd_bought >= params.min_ccd_amount.0, ContractError::InsufficientOutputAmount);

    Cis2Client::transfer(
        host,
        params.token.id.clone(),
        params.token.address,
        params.token_sold,
        ctx.sender(),
        Receiver::Contract(
            ctx.self_address(),
            OwnedEntrypointName::new_unchecked("onReceivingCIS2".to_string())
        ),
    )?;

    host.invoke_transfer(&sender_address, Amount::from_micro_ccd(ccd_bought))?;

    host.state_mut().decrease_exchange_ccd_balance(&params.token, ccd_bought.into())?;

    Ok(())
}


#[receive(
    contract = "pixpel_swap",
    name = "tokenToTokenSwap",
    parameter = "TokenToTokenSwapParams",
    error = "ContractError",
    enable_logger,
    mutable,
)]
pub fn token_to_token_swap<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    _logger: &mut impl HasLogger,
) -> ContractResult<()> {
    let params: TokenToTokenSwapParams = ctx.parameter_cursor().get()?;
    let sender_address = match ctx.sender() {
        Address::Contract(_) => bail!(ContractError::CalledByAContract),
        Address::Account(account_address) => account_address,
    };

    let is_operator = Cis2Client::is_operator_of(
        host,
        ctx.sender(),
        ctx.self_address(),
        &params.token.address,
    )?;
    ensure!(is_operator, ContractError::NotOperator);

    let token_reserve: u64 = get_token_reserve(ctx, host, &params.token)?.into();
    let ccd_reserve: u64 = host.state().get_exchange_ccd_balance(&params.token)?.into();
    let ccd_bought = get_output_amount(
      params.token_sold.0,
      token_reserve,
      ccd_reserve
    )?;

    let purchased_token_reserve: u64 = get_token_reserve(ctx, host, &params.purchased_token)?.into();
    let purchased_ccd_reserve: u64 = host.state().get_exchange_ccd_balance(&params.purchased_token)?.into();
    let purchased_tokens_bought = get_output_amount(
      ccd_bought,
      purchased_ccd_reserve,
      purchased_token_reserve
    )?;

    ensure!(purchased_tokens_bought >= params.min_purchased_token_amount.0, ContractError::InsufficientOutputAmount);

    Cis2Client::transfer(
        host,
        params.token.id.clone(),
        params.token.address,
        params.token_sold,
        ctx.sender(),
        Receiver::Contract(
            ctx.self_address(),
            OwnedEntrypointName::new_unchecked("onReceivingCIS2".to_string())
        ),
    )?;

    Cis2Client::transfer(
        host,
        params.purchased_token.id.clone(),
        params.purchased_token.address,
        ContractTokenAmount::from(purchased_tokens_bought),
        Address::Contract(ctx.self_address()),
        Receiver::Account(sender_address),
    )?;

    host.state_mut().decrease_exchange_ccd_balance(&params.token, ccd_bought.into())?;
    host.state_mut().increase_exchange_ccd_balance(&params.purchased_token, ccd_bought.into())?;

    Ok(())
}