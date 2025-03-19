use concordium_cis2::{Cis2ClientError, Cis2Error};
use concordium_std::{
    CallContractError, LogError, ParseError, Reject, SchemaType, Serialize, TransferError,
    UnwrapAbort,
};

pub mod num {
    pub use concordium_std::num::NonZeroI32;
}

#[cfg(test)]
use concordium_std::from_bytes;

#[derive(Serialize, Debug, PartialEq, Reject, Eq, SchemaType)]
pub enum LaunchPadError {
    /// Raised when parsing the parameter failed.
    #[from(ParseError)]
    Parse,
    Insufficient,
    SmallerHardCap,
    InCorrect,
    InCorrectCliffPeriod,
    ProductNameAlreadyTaken,
    OnlyAccount,
    OnlyContract,
    OnlyAdmin,
    NotFound,
    WrongLaunchPad,
    AmountTooLarge,
    MissingAccount,
    WrongContract,
    WrongHolder,
    WrongTokenAmount,
    WrongTokenID,
    UnAuthorized,
    Paused,
    Live,
    Canceled,
    Finished,
    Vesting,
    UnableToCancel,
    Claimed,
    CliffNotElapsed,
    NotElapsed,
    TimeStillLeft,
    PauseLimit,
    PauseDuration,
    LogFull,
    LogMalformed,
    VestLimit,
    SoftReached,
    SoftNotReached,
    InvalidResponse,
    MissingContract,
    MissingEntrypoint,
    MessageFailed,
    Trap,
    CyclesCompleted,
    Completed,
    UpdateOperatorFailed,
    CIS2(i32),
    DEX(i32),
}

impl From<CallContractError<LaunchPadError>> for LaunchPadError {
    fn from(e: CallContractError<LaunchPadError>) -> Self {
        match e {
            CallContractError::AmountTooLarge => Self::AmountTooLarge,
            CallContractError::MissingAccount => Self::MissingAccount,
            CallContractError::MissingContract => Self::MissingContract,
            CallContractError::MissingEntrypoint => Self::MissingEntrypoint,
            CallContractError::MessageFailed => Self::MessageFailed,
            CallContractError::LogicReject {
                reason,
                return_value: _,
            } => Self::DEX(reason),
            CallContractError::Trap => Self::Trap,
        }
    }
}

impl From<TransferError> for LaunchPadError {
    fn from(value: TransferError) -> Self {
        match value {
            TransferError::AmountTooLarge => Self::AmountTooLarge,
            TransferError::MissingAccount => Self::MissingAccount,
        }
    }
}

impl From<LogError> for LaunchPadError {
    fn from(value: LogError) -> Self {
        match value {
            LogError::Full => Self::LogFull,
            LogError::Malformed => Self::LogMalformed,
        }
    }
}

/// Mapping Cis2ClientError<Error> to Error.
impl From<Cis2ClientError<LaunchPadError>> for LaunchPadError {
    fn from(e: Cis2ClientError<LaunchPadError>) -> Self {
        match e {
            Cis2ClientError::InvokeContractError(err) => err.into(),
            Cis2ClientError::ParseResult => Self::Parse,
            Cis2ClientError::InvalidResponse => Self::InvalidResponse,
        }
    }
}

/// Mapping CallContractError<ExternCallResponse> to Error.
impl From<CallContractError<Cis2Error<LaunchPadError>>> for LaunchPadError {
    fn from(e: CallContractError<Cis2Error<LaunchPadError>>) -> Self {
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

#[cfg(test)]
use concordium_smart_contract_testing::{
    ContractInvokeError, ContractInvokeErrorKind, InvokeFailure,
};

/// Mapping `ContractInvokeError` to `ContractError`
///
/// It parse any invocation error captured while integration testing to contract error
#[cfg(test)]
impl From<ContractInvokeError> for LaunchPadError {
    fn from(value: ContractInvokeError) -> Self {
        if let ContractInvokeErrorKind::ExecutionError { failure_kind } = value.kind {
            if let InvokeFailure::ContractReject { code: _, data } = failure_kind {
                from_bytes::<LaunchPadError>(&data).expect("[Error] Parse Launch-pad error")
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
