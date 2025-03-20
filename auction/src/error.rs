use concordium_cis2::{Cis2ClientError, Cis2Error};
use concordium_std::{CallContractError, LogError, ParseError, SchemaType, Serialize};
use concordium_std::{UnwrapAbort, *};

/// Errors of this contract.
#[derive(Debug, PartialEq, Eq, Clone, Reject, Serialize, SchemaType)]
pub enum Error {
    /// Failed parsing the parameter.
    ///
    /// Error code -1
    #[from(ParseError)]
    ParseParams,
    /// Raised when adding an item; The start time needs to be strictly smaller than the end time.
    ///
    /// Error code -2
    StartEndTimeError,
    /// Raised when adding an item; The end time needs to be in the future.
    ///
    /// Error code -3
    EndTimeError,
    /// Raised when a contract tries to bid; Only accounts
    /// are allowed to bderive" id.
    ///
    /// Error code -4
    OnlyAccount,
    /// Raised when the new bid amount is not greater than the current highest
    /// bid.
    ///
    /// Error code -5
    BidNotGreaterCurrentBid,
    /// Raised when the bid is placed after the auction end time passed.
    ///
    /// Error code -6
    BidTooLate,
    /// Raised when the bid is placed after the auction has been finalized.
    ///
    /// Error code -7
    AuctionAlreadyFinalized,
    /// Raised when the item index cannot be found in the contract.
    ///
    /// Error code -8
    NoItem,
    /// Raised when finalizing an auction before the auction end time passed.
    ///
    /// Error code -9
    AuctionStillActive,
    /// Raised when someone else than the cis2 token contract invokes the `bid`
    /// entry point.
    ///
    /// Error code -10
    NotTokenContract,
    /// Raised when payment is attempted with a different `token_id` than
    /// specified for an item.
    ///
    /// Error code -11
    WrongTokenID,
    /// Raised when the invocation of the cis2 token contract fails.
    ///
    /// Error code -12
    InvokeContractError,
    /// Raised when the parsing of the result from the cis2 token contract
    /// fails.
    ///
    /// Error code -13
    ParseResult,
    /// Raised when the response of the cis2 token contract is invalid.
    ///
    /// Error code -14
    InvalidResponse,
    /// Raised when the amount of cis2 tokens that was to be transferred is not
    /// available to the sender.
    ///
    /// Error code -15
    AmountTooLarge,
    /// Raised when the owner account of the cis 2 token contract that is being
    /// invoked does not exist. This variant should in principle not happen
    /// but is here for completeness.
    ///
    /// Error code -16
    MissingAccount,
    /// Raised when the cis2 token contract that is to be invoked does not
    /// exist.
    ///
    /// Error code -17
    MissingContract,
    /// Raised when the cis2 token contract to be invoked exists but the entry
    /// point that was named does not.
    ///
    /// Error code -18
    MissingEntrypoint,
    /// Raised when the sending of a message to the V0 contract failed.
    ///
    /// Error code -19
    MessageFailed,
    /// Raised when the cis2 token contract called rejected with the given reason.
    ///
    /// Error code -20
    LogicReject,
    /// Raised when the cis2 token contract execution triggered a runtime error.
    ///
    /// Error code -21
    Trap,
    /// Failed logging: Log is full.
    ///
    /// Error code -22
    LogFull,
    /// Failed logging: Log is malformed.
    ///
    /// Error code -23
    LogMalformed,
    /// Failed CCD transfer
    ///
    /// Error code -24
    TransferError,
    /// Caller is not the creator of the auction
    ///
    /// Error code -25
    UnAuthorized,
    /// Given contract is not CIS2 supported
    ///
    /// Error code -26
    CIS2NotSupported,
    /// Auction contract is not the operator of CIS2-contract
    ///
    /// Error code -27
    NotOperator,
    /// Creator of an auction item is not allowed to bid on its
    /// own item
    ///
    /// Error code -28
    CreatorCanNotBid,
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
impl From<Cis2ClientError<Error>> for Error {
    fn from(e: Cis2ClientError<Error>) -> Self {
        match e {
            Cis2ClientError::InvokeContractError(err) => err.into(),
            Cis2ClientError::ParseResult => Self::ParseResult,
            Cis2ClientError::InvalidResponse => Self::InvalidResponse,
        }
    }
}

impl From<u8> for Error {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::ParseParams,
            1 => Self::StartEndTimeError,
            2 => Self::EndTimeError,
            3 => Self::OnlyAccount,
            4 => Self::BidNotGreaterCurrentBid,
            5 => Self::BidTooLate,
            6 => Self::AuctionAlreadyFinalized,
            7 => Self::NoItem,
            8 => Self::AuctionStillActive,
            9 => Self::NotTokenContract,
            10 => Self::WrongTokenID,
            11 => Self::InvokeContractError,
            12 => Self::ParseResult,
            13 => Self::InvalidResponse,
            14 => Self::AmountTooLarge,
            15 => Self::MissingAccount,
            16 => Self::MissingContract,
            17 => Self::MissingEntrypoint,
            18 => Self::MessageFailed,
            19 => Self::LogicReject,
            20 => Self::Trap,
            21 => Self::LogFull,
            22 => Self::LogMalformed,
            23 => Self::TransferError,
            24 => Self::UnAuthorized,
            25 => Self::CIS2NotSupported,
            26 => Self::NotOperator,
            27 => Self::CreatorCanNotBid,
            _ => unimplemented!(),
        }
    }
}
