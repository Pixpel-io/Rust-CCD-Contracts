use concordium_cis2::{
    IsTokenAmount, IsTokenId, TokenAmountU64, TokenIdU8, TokenIdVec, Transfer, TransferParams,
};
use concordium_std::{
    to_bytes, Amount, CallContractError, ContractAddress, Deserial, EntrypointName, HasHost, Host,
    ParseError, Read, Reject, SchemaType, Serial, Write,
};

use crate::{
    errors::LaunchPadError,
    params::{AddLiquidityParams, GetExchangeParams, TokenInfo},
    response::ExchangeView,
    state::State,
    ContractResult,
};

const SUPPORTS_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("supports");
const GET_EXCHANGE_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("getExchange");
const ADD_LIQUIDITY_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("addLiquidity");
const TRANSFER_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("transfer");

pub struct DexClient {
    dex_address: ContractAddress,
}

impl DexClient {
    pub fn new(contract_address: ContractAddress) -> Self {
        Self {
            dex_address: contract_address,
        }
    }

    pub fn address(&self) -> ContractAddress {
        self.dex_address
    }

    pub fn get_exchange(
        &self,
        host: &mut Host<State>,
        params: &GetExchangeParams,
    ) -> Result<ExchangeView, LaunchPadError> {
        let result = self.invoke_contract::<_, ExchangeView>(
            host,
            params,
            GET_EXCHANGE_ENTRYPOINT_NAME,
            Amount::zero(),
        )?;

        Ok(result.1.unwrap())
    }
    pub fn add_liquidity(
        &self,
        host: &mut Host<State>,
        token_id: TokenIdU8,
        token_amount: TokenAmountU64,
        amount: Amount,
        cis2_contract: ContractAddress,
    ) -> Result<(), LaunchPadError> {
        self.invoke_contract::<_, ()>(
            host,
            &AddLiquidityParams {
                token: TokenInfo {
                    id: TokenIdVec(token_id.0.to_ne_bytes().into()),
                    address: cis2_contract,
                },
                token_amount,
            },
            ADD_LIQUIDITY_ENTRYPOINT_NAME,
            amount,
        )?;

        Ok(())
    }

    pub fn transfer<T, A>(
        &self,
        host: &mut Host<State>,
        params: TransferParams<T, A>,
    ) -> Result<bool, LaunchPadError>
    where
        T: IsTokenId,
        A: IsTokenAmount,
    {
        let (state_modified, _) =
            self.invoke_contract::<_, ()>(host, &params, TRANSFER_ENTRYPOINT_NAME, Amount::zero())?;
        Ok(state_modified)
    }

    pub fn invoke_contract<P, R>(
        &self,
        host: &mut Host<State>,
        params: &P,
        method: EntrypointName,
        amount: Amount,
    ) -> Result<(bool, Option<R>), LaunchPadError>
    where
        P: Serial,
        R: Deserial,
    {
        let res = host.invoke_contract(&self.address(), params, method, amount);

        let res = match res {
            Ok(val) => {
                let return_value = match val.1 {
                    Some(mut res) => Some(R::deserial(&mut res)?),
                    None => None,
                };
                (val.0, return_value)
            }
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
                    } => LaunchPadError::DEX(reason),
                };

                return Err(lp_err);
            }
        };

        Ok(res)
    }
}
