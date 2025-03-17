use concordium_cis2::Cis2ClientError;
use concordium_std::{
    CallContractError, LogError, ParseError, Reject, SchemaType, Serialize, TransferError,
    UnwrapAbort,
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
    LogicReject(i32),
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
                reason,
                return_value: _,
            } => Self::LogicReject(reason),
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
            panic!(
                "[Error] Unable to map ContractInvokeError other than ExecutionError {:#?}",
                value
            )
        }
    }
}

#[cfg(test)]
impl From<i32> for LaunchPadError {
    fn from(value: i32) -> Self {
        match value {
            -01 => Self::Parse,
            -02 => Self::Insufficient,
            -03 => Self::SmallerHardCap,
            -04 => Self::InCorrect,
            -05 => Self::InCorrectCliffPeriod,
            -06 => Self::ProductNameAlreadyTaken,
            -07 => Self::OnlyAccount,
            -08 => Self::OnlyContract,
            -09 => Self::OnlyAdmin,
            -10 => Self::NotFound,
            -11 => Self::WrongLaunchPad,
            -12 => Self::AmountTooLarge,
            -13 => Self::MissingAccount,
            -14 => Self::WrongContract,
            -15 => Self::WrongHolder,
            -16 => Self::WrongTokenAmount,
            -17 => Self::WrongTokenID,
            -18 => Self::UnAuthorized,
            -19 => Self::Paused,
            -20 => Self::Live,
            -21 => Self::Canceled,
            -22 => Self::Finished,
            -23 => Self::Vesting,
            -24 => Self::UnableToCancel,
            -25 => Self::Claimed,
            -26 => Self::CliffNotElapsed,
            -27 => Self::NotElapsed,
            -28 => Self::TimeStillLeft,
            -29 => Self::PauseLimit,
            -30 => Self::PauseDuration,
            -31 => Self::LogFull,
            -32 => Self::LogMalformed,
            -33 => Self::VestLimit,
            -34 => Self::SoftReached,
            -35 => Self::SoftNotReached,
            -36 => Self::InvalidResponse,
            -37 => Self::MissingContract,
            -38 => Self::MissingEntrypoint,
            -39 => Self::MessageFailed,
            -40 => Self::Trap,
            -41 => Self::CyclesCompleted,
            -42 => Self::Completed,
            -43 => Self::UpdateOperatorFailed,
            n => Self::LogicReject(n),
        }
    }
}
