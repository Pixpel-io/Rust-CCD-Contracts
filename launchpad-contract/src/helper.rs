use concordium_cis2::{OperatorUpdate, UpdateOperator, UpdateOperatorParams};
use concordium_std::{
    Address, Amount, CallContractError, ContractAddress, EntrypointName, HasHost, Host,
};

use crate::{errors::Error, state::State};

const UPDATE_OPERATOR_ENTRYPOINT: EntrypointName = EntrypointName::new_unchecked("updateOperator");

/// This is the re-implementation of a method `update_operator_of` defined in Cis2Client.
///
/// The need for re-implementation of this function is due to wrong parameters are taken
/// by Cis2Client and results in parse error, So this implementation maps the correct
/// parameters to update an operator in CIS2 compliant contract.
///
/// It returns the result type, which contains either `bool` on successful invocation
/// or returns `Error::CIS2(reason)`, with the actual reject code.
pub fn update_operator_of(
    host: &mut Host<State>,
    cis2_contract: ContractAddress,
    operator_to_be: Address,
) -> Result<bool, Error> {
    let update_operator_params = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Add,
        operator: operator_to_be,
    }]);

    let res = host.invoke_contract(
        &cis2_contract,
        &update_operator_params,
        UPDATE_OPERATOR_ENTRYPOINT,
        Amount::zero(),
    );

    let res = match res {
        Ok(val) => val.0,
        Err(err) => {
            let lp_err = match err {
                CallContractError::AmountTooLarge => Error::AmountTooLarge,
                CallContractError::MessageFailed => Error::MessageFailed,
                CallContractError::Trap => Error::Trap,
                CallContractError::MissingAccount => Error::MissingAccount,
                CallContractError::MissingContract => Error::MissingContract,
                CallContractError::MissingEntrypoint => Error::MissingEntrypoint,
                CallContractError::LogicReject {
                    reason,
                    return_value: _,
                } => Error::CIS2(reason),
            };

            return Err(lp_err);
        }
    };

    Ok(res)
}
