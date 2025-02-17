use concordium_cis2::{Cis2ClientError, Cis2Error};
/// The different errors that the `vote` function can produce.
use concordium_std::*;

#[derive(Serialize, Debug, PartialEq, Eq, Reject, SchemaType)]
pub enum LaunchPadError {
    /// Raised when parsing the parameter failed.
    #[from(ParseError)]
    ParsingFailed, //1

    VestingFinished,               // 2
    StillVesting,            // 3
    InvalidUser,                   // 4
    LaunchpadNotExist,             // 5
    ContractUser,                  // 6
    NotOwner,                      // 7
    NotLive,                       // 8
    UserNotExist,                  // 9
    InSufficientAmount,            // 10
    MinimumInvestmentNotSatisfied, // 11
    LaunchReachedToMaximum,        // 12
    HardcapLimitReached,           // 13
    CliffPeriodNotEnd,             // 14
    ClaimDateNotStarted,           // 15
    InvokeVestingError,            // 16
    InvokeContractNoResult,        // 17
    InvokeContractNoResponse,      // 18
    ParseResult,                   // 19
    ParseParams,                   // 20
    Cis2ClientError,               // 21
    LaunchpadPaused,               // 22
    LaunchpadCancelled,            // 23
    AlreadyClaimed,                // 24
    CannotClaim,                   // 25
    LaunchpadNotEnd,               // 26
    NotOperator,                   // 27
    TokenNotCis2,                  // 28
    HardCappSmaller,               // 29
    LivePauseTimeRestricted,       // 30
    LivePauseCycleCompleted,       // 31
    HardcapNot40ToSoftcap,         // 32
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
    CliffNotElapsed,
    CycleNotElapsed,
    TimeStillLeft,
    PauseLimit,
    PauseDuration,
    LogFull,
    LogMalformed,
    VestLimit,
    SoftReached,
    InvalidResponse,
    MissingContract,
    MissingEntrypoint,
    MessageFailed,
    LogicReject,
    Trap,
    CyclesCompleted
}

impl From<TransferError> for LaunchPadError {
    fn from(value: TransferError) -> Self {
        match value {
            TransferError::AmountTooLarge => Self::AmountTooLarge,
            TransferError::MissingAccount => Self::MissingAccount
        }
    }
}

impl From<LogError> for LaunchPadError {
    fn from(value: LogError) -> Self {
        match value {
            LogError::Full => Self::LogFull,
            LogError::Malformed => Self::LogMalformed
        }   
    }
}

/// Mapping Cis2ClientError<Error> to Error.
impl From<Cis2ClientError<LaunchPadError>> for LaunchPadError {
    fn from(e: Cis2ClientError<LaunchPadError>) -> Self {
        match e {
            Cis2ClientError::InvokeContractError(err) => err.into(),
            Cis2ClientError::ParseResult => Self::ParseResult,
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