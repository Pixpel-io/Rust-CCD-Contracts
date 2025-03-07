use concordium_cis2::Cis2ClientError;
use concordium_std::{
    CallContractError, LogError, ParseError, Reject, SchemaType, Serialize, TransferError, UnwrapAbort
};

pub mod num {
    pub use concordium_std::num::NonZeroI32;
}

#[repr(i32)]
#[derive(Serialize, Debug, PartialEq, Reject, Eq, SchemaType)]
pub enum LaunchPadError {
    /// Raised when parsing the parameter failed.
    #[from(ParseError)]
    Parse,
    Insufficient,
    SmallerHardCap,
    InCorrectTimePeriod,
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
    WithDrawn,
    CliffNotElapsed,
    CycleNotElapsed,
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
    LogicReject,
    Trap,
    CyclesCompleted,
    Completed,
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
impl<T> From<CallContractError<T>> for LaunchPadError {
    fn from(e: CallContractError<T>) -> Self {
        match e {
            CallContractError::AmountTooLarge => Self::AmountTooLarge,
            CallContractError::MissingAccount => Self::MissingAccount,
            CallContractError::MissingContract => Self::MissingContract,
            CallContractError::MissingEntrypoint => Self::MissingEntrypoint,
            CallContractError::MessageFailed => Self::MessageFailed,
            CallContractError::LogicReject {
                reason: _,
                return_value: _,
            } => Self::LogicReject,
            CallContractError::Trap => Self::Trap,
        }
    }
}

#[cfg(test)]
use concordium_smart_contract_testing::{
    ContractInvokeError, ContractInvokeErrorKind, InvokeFailure,
};

/// Mapping `ContractInvokeError` to `auction::error::Error`
///
/// It parse any invocation error captured while integration testing to contract error
#[cfg(test)]
impl From<ContractInvokeError> for LaunchPadError {
    fn from(value: ContractInvokeError) -> Self {
        if let ContractInvokeErrorKind::ExecutionError { failure_kind } = value.kind {
            if let InvokeFailure::ContractReject { code, data: _ } = failure_kind {
                code.into()
            } else {
                panic!("[Error] Unable to map received invocation error code")
            }
        } else {
            panic!("[Error] Unable to map ContractInvokeError other than ExecutionError {:#?}", value)
        }
    }
}

#[cfg(test)]
macro_rules! impl_from_i32_contiguous {
    ($enum:ident, $first:expr, $last:expr) => {
        impl From<i32> for $enum {
            fn from(code: i32) -> Self {
                debug_assert!(code <= $first && code >= $last, "Error code out-of-bounds");
                unsafe { std::mem::transmute((-code - 1)) }
            }
        }
    };
}

#[cfg(test)]
impl_from_i32_contiguous!(LaunchPadError, -1, -43);
