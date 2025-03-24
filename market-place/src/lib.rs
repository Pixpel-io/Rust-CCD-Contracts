//! Marketplace Contract
//! This module provides implementation of the marketplace contract.
//! Marketplace Contract provides following functions
//! - `list` : returns a list of buyable tokens added to the contract instance.
//! - `add` : adds the token to the list of buyable tokens taking the price of
//!   the token as input.
//! - `transfer` : transfer the authority of the input listed token from one
//!   address to another.
//!
//! This code has not been checked for production readiness. Please use for
//! reference purposes
mod errors;
mod params;
mod response;
mod state;

use concordium_cis2::*;
use concordium_std::*;
use errors::Error;
use params::{AddParams, BuyerParams, InitParams, ListParams, TokenList};
use response::Token;
use state::{
    Commission, Price, State, TokenIdentifier, TokenInfo, TokenListItem, TokenRoyaltyState,
};

use crate::{params::TransferParams, state::TokenOwnerInfo};

#[cfg(test)]
mod tests;

type ContractResult<A> = Result<A, Error>;

const MAX_BASIS_POINTS: u16 = 10000;

/// Type of token Id used by the CIS2 contract.
type ContractTokenId = TokenIdU8;

/// Type of Token Amount used by the CIS2 contract.
type ContractTokenAmount = TokenAmountU64;

type Cis2ClientResult<T> = Result<T, concordium_cis2::Cis2ClientError<()>>;

/// Initializes a new Marketplace Contract
///
/// This function can be called by using InitParams.
/// The commission should be less than the maximum allowed value of 10000 basis
/// points
#[init(contract = "Market-Place", parameter = "InitParams")]
fn init(ctx: &InitContext, state_builder: &mut StateBuilder) -> InitResult<State> {
    let params: InitParams = ctx.parameter_cursor().get()?;

    ensure!(
        params.commission <= MAX_BASIS_POINTS,
        Error::InvalidCommission.into()
    );

    Ok(State::new(state_builder, params))
}

#[receive(
    contract = "Market-Place",
    name = "ListToken",
    parameter = "ListParams",
    mutable
)]
fn add(ctx: &ReceiveContext, host: &mut Host<State>) -> ContractResult<()> {
    let params: ListParams = ctx.parameter_cursor().get().map_err(|_e| Error::Parse)?;

    let sender = match ctx.sender() {
        Address::Account(address) => address,
        Address::Contract(_) => return Err(Error::OnlyContract),
    };

    ensure_supports_cis2(host, &params.cis2_address)?;
    ensure_is_operator(host, ctx, &params.cis2_address)?;
    ensure_balance(
        host,
        params.id,
        &params.cis2_address,
        sender,
        params.quantity.into(),
    )?;

    let (state, _) = host.state_and_builder();

    if state.add_token(&sender, params) {
        return Ok(());
    }

    Err(Error::JobFailed)
}

/// Allows for transferring the token specified by TransferParams.
///
/// This function is the typical buy function of a Marketplace where one
/// account can transfer an Asset by paying a price. The transfer will fail of
/// the Amount paid is < token_quantity * token_price
#[receive(
    contract = "Market-Place",
    name = "BuyToken",
    parameter = "BuyerParams",
    mutable,
    payable
)]
fn buy(ctx: &ReceiveContext, host: &mut Host<State>, amount: Amount) -> ContractResult<()> {
    let buyer = match ctx.sender() {
        Address::Account(address) => address,
        Address::Contract(_) => return Err(Error::OnlyContract),
    };

    let params: BuyerParams = ctx.parameter_cursor().get()?;

    let token_details = host.state().get_token(params.id, params.cis2_address)?;

    ensure!(params.quantity >= token_details.quantity, Error::JobFailed);
    ensure!(params.payment == token_details.price, Error::InvalidPayment);

    match token_details.price {
        Price::CCD(per_unit) => {
            let net_amount = per_unit * params.quantity;
            ensure!(
                amount >= Amount::from_ccd(net_amount),
                Error::InvalidTokenQuantity
            );

            let comission = (net_amount * host.state().commission.percentage_basis() as u64) / 100;

            host.invoke_transfer(&host.state().admin, Amount::from_ccd(comission))?;
            host.invoke_transfer(
                &token_details.owner,
                Amount::from_ccd(net_amount - comission),
            )?;
        }
        Price::PIXP(per_unit) => {
            let net_amount = per_unit * params.quantity;

            let pixp = host.state().pixp_client;
            let admin = host.state().admin;

            ensure_balance(host, pixp.0, &pixp.1, buyer, net_amount.into())?;

            let comission = (net_amount * host.state().commission.percentage_basis() as u64) / 100;

            Cis2Client::new(pixp.1).transfer(
                host,
                Transfer::<ContractTokenId, ContractTokenAmount> {
                    token_id: pixp.0,
                    amount: comission.into(),
                    from: buyer.into(),
                    to: admin.into(),
                    data: AdditionalData::empty(),
                },
            )?;

            Cis2Client::new(pixp.1).transfer(
                host,
                Transfer::<ContractTokenId, ContractTokenAmount> {
                    token_id: pixp.0,
                    amount: (net_amount - comission).into(),
                    from: buyer.into(),
                    to: token_details.owner.into(),
                    data: AdditionalData::empty(),
                },
            )?;
        }
    }

    Cis2Client::new(params.cis2_address).transfer(
        host,
        Transfer::<ContractTokenId, ContractTokenAmount> {
            token_id: params.id,
            amount: params.quantity.into(),
            from: token_details.owner.into(),
            to: buyer.into(),
            data: AdditionalData::empty(),
        },
    )?;

    host.state_mut()
        .token_list
        .get_mut(&TokenIdentifier {
            id: params.id,
            cis2_address: params.cis2_address,
        })
        .unwrap()
        .quantity -= params.quantity;

    Ok(())
}

/// Returns a list of Added Tokens with Metadata which contains the token price
#[receive(contract = "Market-Place", name = "list", return_value = "TokenList")]
fn list(_ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<TokenList> {
    let tokens: Vec<TokenListItem<ContractTokenId, ContractTokenAmount>> = host
        .state()
        .list()
        .iter()
        .filter(|t| t.quantity.cmp(&ContractTokenAmount::from(0)).is_gt())
        .cloned()
        .collect::<Vec<TokenListItem<ContractTokenId, ContractTokenAmount>>>();

    Ok(TokenList(tokens))
}

#[receive(
    contract = "Market-Place",
    name = "ViewTokenList",
    return_value = "Vec<Token>"
)]
fn view_list(_ctx: &ReceiveContext, host: &Host<State>) -> ContractResult<Vec<Token>> {
    let mut list: Vec<Token> = Vec::new();

    for (identifier, details) in host.state().token_list.iter() {
        list.push(Token {
            id: identifier.id,
            price: details.price,
            quantity: details.quantity,
            owner: details.owner.clone(),
        });
    }

    Ok(list)
}

struct DistributableAmounts {
    to_primary_owner: Amount,
    to_seller: Amount,
    to_marketplace: Amount,
}

/// Calls the [supports](https://proposals.concordium.software/CIS/cis-0.html#supports) function of CIS2 contract.
/// Returns error If the contract does not support the standard.
fn ensure_supports_cis2(
    host: &mut Host<State>,
    cis_contract_address: &ContractAddress,
) -> ContractResult<()> {
    let cis2_client = Cis2Client::new(*cis_contract_address);
    let res: Cis2ClientResult<SupportResult> = cis2_client.supports_cis2(host);

    let res = match res {
        Ok(res) => res,
        Err(_) => bail!(Error::Cis2ClientError),
    };

    match res {
        SupportResult::NoSupport => bail!(Error::CollectionNotCis2),
        SupportResult::SupportBy(_) => Ok(()),
        SupportResult::Support => Ok(()),
    }
}

/// Calls the [operatorOf](https://proposals.concordium.software/CIS/cis-2.html#operatorof) function of CIS contract.
/// Returns error if Current Contract Address is not an Operator of Transaction
/// Sender.
fn ensure_is_operator(
    host: &mut Host<State>,
    ctx: &ReceiveContext,
    cis_contract_address: &ContractAddress,
) -> ContractResult<()> {
    let cis2_client = Cis2Client::new(*cis_contract_address);
    let res: Cis2ClientResult<bool> =
        cis2_client.operator_of(host, ctx.sender(), Address::Contract(ctx.self_address()));
    let res = match res {
        Ok(res) => res,
        Err(_) => bail!(Error::Cis2ClientError),
    };
    ensure!(res, Error::NotOperator);
    Ok(())
}

/// Calls the [balanceOf](https://proposals.concordium.software/CIS/cis-2.html#balanceof) function of the CIS2 contract.
/// Returns error if the returned balance < input balance (balance param).
fn ensure_balance(
    host: &mut Host<State>,
    token_id: ContractTokenId,
    cis_contract_address: &ContractAddress,
    owner: AccountAddress,
    minimum_balance: ContractTokenAmount,
) -> ContractResult<()> {
    let cis2_client = Cis2Client::new(*cis_contract_address);

    let res: Cis2ClientResult<ContractTokenAmount> =
        cis2_client.balance_of(host, token_id, Address::Account(owner));
    let res = match res {
        Ok(res) => res,
        Err(_) => bail!(Error::Cis2ClientError),
    };
    ensure!(res.cmp(&minimum_balance).is_ge(), Error::NoBalance);

    Ok(())
}

// Distributes Selling Price, Royalty & Commission amounts.
fn distribute_amounts(
    host: &mut Host<State>,
    amount: Amount,
    token_owner: &AccountAddress,
    token_royalty_state: &TokenRoyaltyState,
    marketplace_owner: &AccountAddress,
) -> Result<(), Error> {
    let amounts = calculate_amounts(
        &amount,
        &host.state().commission,
        token_royalty_state.royalty,
    );

    host.invoke_transfer(token_owner, amounts.to_seller)
        .map_err(|_| Error::InvokeTransferError)?;

    if amounts
        .to_marketplace
        .cmp(&Amount::from_micro_ccd(0))
        .is_gt()
    {
        host.invoke_transfer(marketplace_owner, amounts.to_marketplace)
            .map_err(|_| Error::InvokeTransferError)?;
    }

    if amounts
        .to_primary_owner
        .cmp(&Amount::from_micro_ccd(0))
        .is_gt()
    {
        host.invoke_transfer(&token_royalty_state.primary_owner, amounts.to_primary_owner)
            .map_err(|_| Error::InvokeTransferError)?;
    };

    Ok(())
}

/// Calculates the amounts (Commission, Royalty & Selling Price) to be
/// distributed
fn calculate_amounts(
    amount: &Amount,
    commission: &Commission,
    royalty_percentage_basis: u16,
) -> DistributableAmounts {
    let commission_amount = (*amount * commission.percentage_basis().into())
        .quotient_remainder(MAX_BASIS_POINTS.into());

    let royalty_amount =
        (*amount * royalty_percentage_basis.into()).quotient_remainder(MAX_BASIS_POINTS.into());

    DistributableAmounts {
        to_seller: amount
            .subtract_micro_ccd(commission_amount.0.micro_ccd())
            .subtract_micro_ccd(royalty_amount.0.micro_ccd()),
        to_marketplace: commission_amount.0,
        to_primary_owner: royalty_amount.0,
    }
}
