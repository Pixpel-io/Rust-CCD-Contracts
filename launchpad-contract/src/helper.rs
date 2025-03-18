use concordium_cis2::{OperatorUpdate, UpdateOperator, UpdateOperatorParams};
use concordium_std::{
    Address, Amount, CallContractError, ContractAddress, EntrypointName, HasHost, Host,
};

use crate::{errors::LaunchPadError, state::State};

const UPDATE_OPERATOR_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("updateOperator");

pub fn update_operator_of(
    host: &mut Host<State>,
    cis2_contract: ContractAddress,
    operator_to_be: Address,
) -> Result<bool, LaunchPadError> {
    let update_operator_params = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Add,
        operator: operator_to_be,
    }]);

    let res = host.invoke_contract(
        &cis2_contract,
        &update_operator_params,
        UPDATE_OPERATOR_ENTRYPOINT_NAME,
        Amount::zero(),
    );

    let res = match res {
        Ok(val) => val.0,
        Err(err) => {
            let lp_err = match err {
                CallContractError::AmountTooLarge => LaunchPadError::AmountTooLarge,
                CallContractError::MessageFailed => LaunchPadError::MessageFailed,
                CallContractError::Trap => LaunchPadError::Trap,
                CallContractError::MissingAccount => LaunchPadError::MissingAccount,
                CallContractError::MissingContract => LaunchPadError::MissingContract,
                CallContractError::MissingEntrypoint => LaunchPadError::MissingEntrypoint,
                CallContractError::LogicReject {
                    reason,
                    return_value: _,
                } => LaunchPadError::CIS2(reason),
            };

            return Err(lp_err);
        }
    };

    Ok(res)
}
