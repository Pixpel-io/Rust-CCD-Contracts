use concordium_cis2::{
    IsTokenAmount, IsTokenId, TokenAmountU64 as TokenAmount, TokenIdU64, TokenIdU8, TokenIdVec,
    TransferParams,
};
use concordium_std::{
    Address, Amount, CallContractError, ContractAddress, Deserial, EntrypointName, HasHost, Host,
    SchemaType, Serial, Serialize,
};

use crate::{errors::Error, state::State};

/// DEX `getExchange` entry-point name as `EntrypointName` type.
const GET_EXCHANGE_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("getExchange");
/// DEX `addLiquidity` entry-point name as `EntrypointName` type.
const ADD_LIQUIDITY_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("addLiquidity");
/// DEX `transfer` entry-point name as `EntrypointName` type.
const TRANSFER_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("transfer");

/// Defines the parameters to be passed required for adding liquidity
/// in DEX.
#[derive(Serial, Deserial, SchemaType)]
pub struct AddLiquidityParams {
    pub token: TokenInfo,
    pub token_amount: TokenAmount,
}

/// Contains the information regarding tokens, to be added and locked
/// in liquidity pool in DEX.
#[derive(Serial, Deserial, SchemaType, Clone, Debug)]
pub struct TokenInfo {
    /// Token ID for the token to be added.
    pub id: TokenIdVec,
    /// CIS2 address of the token.
    pub address: ContractAddress,
}

/// Defines the parameters to be passed to the DEX, to get the information
/// regarding the added liquidity pool.
#[derive(Serial, Deserial, SchemaType)]
pub struct GetExchangeParams {
    /// Address of the holder which added the tokens
    /// in pool.
    pub holder: Address,
    /// Token information regarding the token added
    /// in liquidity pool.
    pub token: TokenInfo,
}

/// Defines the response returned by `getExchange` method. It contains all
/// the details regarding locked tokens in liquidity pool.
#[derive(Serialize, SchemaType, Debug)]
pub struct ExchangeView {
    /// Token information, which are added in the pool.
    pub token: TokenInfo,
    /// Amount of tokens added in the pool.
    pub token_balance: TokenAmount,
    /// Amount of CCD added in the pool against the token.
    pub ccd_balance: TokenAmount,
    /// Tokend ID of LPToken assigned by the DEX.
    pub lp_token_id: TokenIdU64,
    /// Amount of LPTokens released by the DEX agains the
    /// pool.
    pub lp_tokens_supply: TokenAmount,
    /// Balance amount of LPTokens related to the holder.
    pub lp_tokens_holder_balance: TokenAmount,
}

/// # DEX Client
///
/// This is the client implementation for `DEX(Decentralized-Exchange)` contract, which
/// provides APIs to let the launch-pad interact with DEX compliant contract.
pub struct DexClient(pub ContractAddress);

impl DexClient {
    /// Constructor method, creates a new instace of DEX client
    /// from the provide dex contract address.
    pub fn new(contract_address: ContractAddress) -> Self {
        Self(contract_address)
    }

    /// Getter method to get the dex contract address of the DEX
    /// client.
    pub fn address(&self) -> ContractAddress {
        self.0
    }

    /// Calls the `getExchange` entry point of the DEX contract
    /// and returns the result.
    ///
    /// Result might contains the `ExchangeView` or the error
    /// return by DEX client.
    pub fn get_exchange(
        &self,
        host: &mut Host<State>,
        params: &GetExchangeParams,
    ) -> Result<ExchangeView, Error> {
        let result = self.invoke_contract::<_, ExchangeView>(
            host,
            params,
            GET_EXCHANGE_ENTRYPOINT_NAME,
            Amount::zero(),
        )?;

        Ok(result.1.unwrap())
    }

    /// Calls the `addLiquidity` entry point of the DEX contract
    /// to add a liquidity pool and returns the result.
    ///
    /// Returns never type `()` or the error returned by DEX client.
    pub fn add_liquidity(
        &self,
        host: &mut Host<State>,
        token_id: TokenIdU8,
        token_amount: TokenAmount,
        amount: Amount,
        cis2_contract: ContractAddress,
    ) -> Result<(), Error> {
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

    /// Calls the `transfer` entry point of the DEX contract
    /// to transfer the LPTokens and returns the result.
    ///
    /// Returns a `bool` if the transfer was successful or the
    /// error returned by DEX client.
    pub fn transfer<T, A>(
        &self,
        host: &mut Host<State>,
        params: TransferParams<T, A>,
    ) -> Result<bool, Error>
    where
        T: IsTokenId,
        A: IsTokenAmount,
    {
        let (state_modified, _) =
            self.invoke_contract::<_, ()>(host, &params, TRANSFER_ENTRYPOINT_NAME, Amount::zero())?;
        Ok(state_modified)
    }

    /// Raw implementation of invoking a method in DEX client. It
    /// is generic over input parameters and the return value type.
    ///
    /// - P : It takes any parameters as input which is serializable.
    /// - R : It returns any type, which is deserializable.
    ///
    /// This method returns a Result type, which contains either the
    /// `bool` and expected return type as `Option<R>` if expected or
    /// returns the error returned by DEX client as the contract error
    /// `Error::DEX(reason)` with the reject code.
    pub fn invoke_contract<P, R>(
        &self,
        host: &mut Host<State>,
        params: &P,
        method: EntrypointName,
        amount: Amount,
    ) -> Result<(bool, Option<R>), Error>
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
                    CallContractError::AmountTooLarge => Error::AmountTooLarge,
                    CallContractError::MessageFailed => Error::MessageFailed,
                    CallContractError::Trap => Error::Trap,
                    CallContractError::MissingAccount => Error::MissingAccount,
                    CallContractError::MissingContract => Error::MissingContract,
                    CallContractError::MissingEntrypoint => Error::MissingEntrypoint,
                    CallContractError::LogicReject {
                        reason,
                        return_value: _,
                    } => Error::DEX(reason),
                };

                return Err(lp_err);
            }
        };

        Ok(res)
    }
}
