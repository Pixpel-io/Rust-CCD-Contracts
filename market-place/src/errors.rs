//! Provides error types which can be returned by Marketplace Contract.
//! Read more about errors which can be returned by a Concordium Contract [here](https://developer.concordium.software/en/mainnet/smart-contracts/guides/custom-errors.html)

use concordium_cis2::{Cis2ClientError, Cis2Error};
use concordium_std::{self, CallContractError, Deserial, ParseError, Reject, SchemaType, Serial, TransferError, UnwrapAbort};
pub mod num {
    pub use concordium_std::num::NonZeroI32;
}

#[derive(Debug, Reject, Deserial, Serial, SchemaType)]
pub enum Error {
    #[from(ParseError)]
    Parse,
    OnlyContract,
    JobFailed,
    NotFound,
    InvalidPayment,
    CalledByAContract,
    TokenNotListed,
    Cis2ClientError,
    CollectionNotCis2,
    InvalidAmountPaid,
    InvokeTransferError,
    NoBalance,
    NotOperator,
    InvalidCommission,
    InvalidTokenQuantity,
    InvalidRoyalty,
    AmountTooLarge,
    MissingAccount,
    InvalidResponse,
    MissingContract,
    MissingEntrypoint,
    MessageFailed,
    Trap,
    CIS2(i32)
}

// Mapping error received from cis2-client `(Cis2ClientError)` to
// contract error.
impl From<Cis2ClientError<Error>> for Error {
    fn from(e: Cis2ClientError<Error>) -> Self {
        match e {
            Cis2ClientError::InvokeContractError(err) => err.into(),
            Cis2ClientError::ParseResult => Self::Parse,
            Cis2ClientError::InvalidResponse => Self::InvalidResponse,
        }
    }
}

// Mapping error received from cis2-client `(Cis2ClientError)` to
// contract error.
impl From<CallContractError<Cis2Error<Error>>> for Error {
    fn from(e: CallContractError<Cis2Error<Error>>) -> Self {
        match e {
            CallContractError::AmountTooLarge => Self::AmountTooLarge,
            CallContractError::MissingAccount => Self::MissingAccount,
            CallContractError::MissingContract => Self::MissingContract,
            CallContractError::MissingEntrypoint => Self::MissingEntrypoint,
            CallContractError::MessageFailed => Self::MessageFailed,
            CallContractError::LogicReject {
                reason,
                return_value: _,
            } => Self::CIS2(reason),
            CallContractError::Trap => Self::Trap,
        }
    }
}

// Mapping error received while transfering amount `(TransferError)`
// to the contract error.
impl From<TransferError> for Error {
    fn from(value: TransferError) -> Self {
        match value {
            TransferError::AmountTooLarge => Self::AmountTooLarge,
            TransferError::MissingAccount => Self::MissingAccount,
        }
    }
}

#[cfg(test)]
use concordium_std::from_bytes;

#[cfg(test)]
use concordium_smart_contract_testing::{
    ContractInvokeError, ContractInvokeErrorKind, InvokeFailure,
};

#[cfg(test)]
impl From<ContractInvokeError> for Error {
    fn from(value: ContractInvokeError) -> Self {
        if let ContractInvokeErrorKind::ExecutionError { failure_kind } = value.kind {
            if let InvokeFailure::ContractReject { code: _, data } = failure_kind {
                from_bytes::<Error>(&data).expect("[Error] Parse Launch-pad error")
            } else {
                panic!("[Error] Unable to map received invocation error code")
            }
        } else {
            panic!(
                "[Error] Unable to map ContractInvokeError other than ExecutionError {:#?}",
                value
            )
        }
    }
}