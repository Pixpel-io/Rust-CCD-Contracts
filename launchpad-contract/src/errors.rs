/// The different errors that the `vote` function can produce.
use concordium_std::*;

#[derive(Serialize, Debug, PartialEq, Eq, Reject, SchemaType)]
pub enum LaunchPadError {
    /// Raised when parsing the parameter failed.
    #[from(ParseError)]
    ParsingFailed, //1

    VestingFinished,               // 2
    VestingNotFinished,            // 3
    InvalidUser,                   // 4
    LaunchpadNotExist,             // 5
    ContractUser,                  // 6
    NotOwner,                      // 7
    NotLive,                       // 8
    UserNotExist,                  // 9
    InsuffiecienRegFee,            // 10
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
    LaunchPadNotFound,
    AmountTooLarge,
    MissingAccount,
    WrongContract,
    WrongTokenAmount,
    WrongTokenID,
    UnAuthorized,
    AlreadyPaused,
    AlreadyLive,
    TimeStillLeft,
    PauseLimit,
    PauseDuration,
}

impl From<TransferError> for LaunchPadError {
    fn from(value: TransferError) -> Self {
        match value {
            TransferError::AmountTooLarge => Self::AmountTooLarge,
            TransferError::MissingAccount => Self::MissingAccount
        }
    }
}