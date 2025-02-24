use concordium_cis2::{Cis2ClientError, Cis2Error};
/// The different errors that the `vote` function can produce.
use concordium_std::*;

#[derive(Serialize, Debug, PartialEq, Eq, Reject, SchemaType)]
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
impl From<CallContractError<Cis2Error<LaunchPadError>>> for LaunchPadError {
    fn from(e: CallContractError<Cis2Error<LaunchPadError>>) -> Self {
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
impl From<i32> for LaunchPadError {
    fn from(code: i32) -> Self {
        match code {
            -1 => Self::Parse,
            -2 => Self::Insufficient,
            -3 => Self::SmallerHardCap,
            -4 => Self::InCorrectTimePeriod,
            -5 => Self::InCorrectCliffPeriod,
            -6 => Self::ProductNameAlreadyTaken,
            -7 => Self::OnlyAccount,
            -8 => Self::OnlyContract,
            -9 => Self::OnlyAdmin,
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
            -25 => Self::WithDrawn,
            -26 => Self::CliffNotElapsed,
            -27 => Self::CycleNotElapsed,
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
            -40 => Self::LogicReject,
            -41 => Self::Trap,
            -42 => Self::CyclesCompleted,
            -43 => Self::Completed,
            _ => unimplemented!(),
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
            panic!("[Error] Unable to map ContractInvokeError other than ExecutionError")
        }
    }
}
