use std::vec;

use concordium_cis2::*;
use concordium_std::*;

use crate::{errors::ContractError, state::State};

pub const SUPPORTS_ENTRYPOINT_NAME: &str = "supports";
pub const OPERATOR_OF_ENTRYPOINT_NAME: &str = "operatorOf";
pub const BALANCE_OF_ENTRYPOINT_NAME: &str = "balanceOf";
pub const TRANSFER_ENTRYPOINT_NAME: &str = "transfer";

pub struct Cis2Client;

impl Cis2Client {
    pub(crate) fn supports_cis2<
        S: HasStateApi,
    >(
        host: &mut impl HasHost<State<S>, StateApiType = S>,
        token_contract_address: &ContractAddress,
    ) -> Result<bool, ContractError> {
        let params = SupportsQueryParams {
            queries: vec![StandardIdentifierOwned::new_unchecked("CIS-2".to_string())],
        };
        let parsed_res: SupportsQueryResponse = Cis2Client::invoke_contract_read_only(
            host,
            token_contract_address,
            SUPPORTS_ENTRYPOINT_NAME,
            &params,
        )?;
        let supports_cis2: bool = {
            let f = parsed_res
                .results
                .first()
                .ok_or(ContractError::InvokeContractError)?;
            match f {
                SupportResult::NoSupport => false,
                SupportResult::Support => true,
                SupportResult::SupportBy(_) => false,
            }
        };

        Ok(supports_cis2)
    }

    pub(crate) fn is_operator_of<
        S: HasStateApi,
    >(
        host: &mut impl HasHost<State<S>, StateApiType = S>,
        owner: Address,
        current_contract_address: ContractAddress,
        token_contract_address: &ContractAddress,
    ) -> Result<bool, ContractError> {
        let params = &OperatorOfQueryParams {
            queries: vec![OperatorOfQuery {
                owner,
                address: Address::Contract(current_contract_address),
            }],
        };

        let parsed_res: OperatorOfQueryResponse = Cis2Client::invoke_contract_read_only(
            host,
            token_contract_address,
            OPERATOR_OF_ENTRYPOINT_NAME,
            params,
        )?;

        let is_operator = parsed_res
            .0
            .first()
            .ok_or(ContractError::InvokeContractError)?
            .to_owned();

        Ok(is_operator)
    }

    pub(crate) fn get_balance<
        S,
        T: IsTokenId + Clone,
        A: Default + IsTokenAmount + Clone + Copy + ops::Sub<Output = A>,
    >(
        host: &mut impl HasHost<State<S>, StateApiType = S>,
        token_id: T,
        token_contract_address: &ContractAddress,
        owner: Address,
    ) -> Result<A, ContractError>
    where
        S: HasStateApi,
    {
        let params = BalanceOfQueryParams {
            queries: vec![BalanceOfQuery {
                token_id,
                address: owner,
            }],
        };

        let parsed_res: BalanceOfQueryResponse<A> = Cis2Client::invoke_contract_read_only(
            host,
            token_contract_address,
            BALANCE_OF_ENTRYPOINT_NAME,
            &params,
        )?;

        let ret = parsed_res.0.first().map_or(A::default(), |f| *f);

        Ok(ret)
    }

    pub(crate) fn transfer<
        S,
        T: IsTokenId + Clone,
        A: IsTokenAmount + Clone + Copy + ops::Sub<Output = A>,
    >(
        host: &mut impl HasHost<State<S>, StateApiType = S>,
        token_id: T,
        token_contract_address: ContractAddress,
        amount: A,
        from: Address,
        to: Receiver,
    ) -> Result<bool, ContractError>
    where
        S: HasStateApi,
        A: IsTokenAmount,
    {
        let params = TransferParams(vec![Transfer {
            token_id,
            amount,
            from,
            data: AdditionalData::empty(),
            to,
        }]);

        Cis2Client::invoke_contract_read_only(
            host,
            &token_contract_address,
            TRANSFER_ENTRYPOINT_NAME,
            &params,
        )?;

        Ok(true)
    }

    fn invoke_contract_read_only<
        S: HasStateApi,
        R: Deserial,
        P: Serial,
    >(
        host: &mut impl HasHost<State<S>, StateApiType = S>,
        contract_address: &ContractAddress,
        entrypoint_name: &str,
        params: &P,
    ) -> Result<R, ContractError> {
        let invoke_contract_result = host
            .invoke_contract_read_only(
                contract_address,
                params,
                EntrypointName::new(entrypoint_name).unwrap_abort(),
                Amount::from_ccd(0),
            )?;
        let mut invoke_contract_res = match invoke_contract_result {
            Some(s) => s,
            None => return Err(ContractError::InvokeContractNoResult),
        };
        let parsed_res =
            R::deserial(&mut invoke_contract_res).map_err(|_e| ContractError::ParseResult)?;

        Ok(parsed_res)
    }

}
