use concordium_cis2::{Cis2ClientError, Cis2Error};
use concordium_std::{
    CallContractError, LogError, ParseError, SchemaType, Serialize,
};
use concordium_std::{UnwrapAbort, *};

/// Errors of this contract.
#[derive(Debug, PartialEq, Eq, Clone, Reject, Serialize, SchemaType)]
pub enum Error {
    /// Failed parsing the parameter.
    #[from(ParseError)]
    ParseParams, //-1
    // Raised when adding an item; The start time needs to be strictly smaller than the end time.
    StartEndTimeError, //-2
    // Raised when adding an item; The end time needs to be in the future.
    EndTimeError, //-3
    /// Raised when a contract tries to bid; Only accounts
    /// are allowed to bderive" id.
    OnlyAccount, //-4
    /// Raised when the new bid amount is not greater than the current highest
    /// bid.
    BidNotGreaterCurrentBid, //-5
    /// Raised when the bid is placed after the auction end time passed.
    BidTooLate, //-6
    /// Raised when the bid is placed after the auction has been finalized.
    AuctionAlreadyFinalized, //-7
    /// Raised when the item index cannot be found in the contract.
    NoItem, //-8
    /// Raised when finalizing an auction before the auction end time passed.
    AuctionStillActive, //-9
    /// Raised when someone else than the cis2 token contract invokes the `bid`
    /// entry point.
    NotTokenContract, //-10
    /// Raised when payment is attempted with a different `token_id` than
    /// specified for an item.
    WrongTokenID, //-11
    /// Raised when the invocation of the cis2 token contract fails.
    InvokeContractError, //-12
    /// Raised when the parsing of the result from the cis2 token contract
    /// fails.
    ParseResult, //-13
    /// Raised when the response of the cis2 token contract is invalid.
    InvalidResponse, //-14
    /// Raised when the amount of cis2 tokens that was to be transferred is not
    /// available to the sender.
    AmountTooLarge, //-15
    /// Raised when the owner account of the cis 2 token contract that is being
    /// invoked does not exist. This variant should in principle not happen,
    /// but is here for completeness.
    MissingAccount, //-16
    /// Raised when the cis2 token contract that is to be invoked does not
    /// exist.
    MissingContract, //-17
    /// Raised when the cis2 token contract to be invoked exists, but the entry
    /// point that was named does not.
    MissingEntrypoint, //-18
    // Raised when the sending of a message to the V0 contract failed.
    MessageFailed, //-19
    // Raised when the cis2 token contract called rejected with the given reason.
    LogicReject, //-20
    // Raised when the cis2 token contract execution triggered a runtime error.
    Trap, //-21
    /// Failed logging: Log is full.
    LogFull, // -22
    /// Failed logging: Log is malformed.
    LogMalformed, // -23
    /// Failed CCD transfer
    TransferError, // -24
    /// Caller is not the creator of the auction
    UnAuthorized, // -25
    /// Given contract is not CIS2 supported
    CIS2NotSupported, // -26
    /// Auction contract is not the operator of CIS2-contract
    NotOperator, // -27
}

pub type ContractResult<A> = Result<A, Error>;

/// Mapping the logging errors to Error.
impl From<LogError> for Error {
    fn from(le: LogError) -> Self {
        match le {
            LogError::Full => Self::LogFull,
            LogError::Malformed => Self::LogMalformed,
        }
    }
}

/// Mapping CallContractError<ExternCallResponse> to Error.
impl From<CallContractError<Cis2Error<Error>>> for Error {
    fn from(e: CallContractError<Cis2Error<Error>>) -> Self {
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

/// Mapping Cis2ClientError<Error> to Error.
impl From<Cis2ClientError<crate::error::Error>> for crate::error::Error {
    fn from(e: Cis2ClientError<crate::error::Error>) -> Self {
        match e {
            Cis2ClientError::InvokeContractError(err) => err.into(),
            Cis2ClientError::ParseResult => Self::ParseResult,
            Cis2ClientError::InvalidResponse => Self::InvalidResponse,
        }
    }
}
