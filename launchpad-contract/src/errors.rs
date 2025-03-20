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

/// This defines all the errors as reject reason, occured while any underlying
/// methods invocation.
#[derive(Serialize, Debug, PartialEq, Reject, Eq, SchemaType)]
pub enum Error {
    /// Raised when parsing the parameter failed.
    ///
    /// Code -1
    #[from(ParseError)]
    Parse,
    /// Raised when the CCDs or tokens are not receided
    /// according to the expected amounts.
    ///
    /// Code -2
    Insufficient,
    /// Raised when in-correct information is received in
    /// parameters.
    ///
    /// Code -3
    InCorrect,
    /// Raised when the product name for a launch pad is
    /// already present in the contract.
    ///
    /// Code -4
    Taken,
    /// Raised when only account is allowed to invoke the
    /// method.
    ///
    /// Code -5
    OnlyAccount,
    /// Raised when only contract is allowed to invoke the
    /// method.
    ///
    /// Code -6
    OnlyContract,
    /// Raised when thee provided information is not found
    /// either the contract or launch pad.
    ///
    /// Code -6
    NotFound,
    /// Raised when an un-authorized account/contract tries
    /// to invoke a method.
    ///
    /// Code -7
    UnAuthorized,
    /// Raised when the release cycles (locked/unlocked) are
    /// already claimed.
    ///
    /// Code -8
    Claimed,
    /// Raised when duration is not elapsed before certain
    /// operation can be done, for example, holder can not
    /// claim tokens until the vesting duration is elapsed.
    ///
    /// Code -9
    NotElapsed,
    /// Raised when some parameter is out-of-bounds of a restricted
    /// limit, such Holder can vest within a limit posed by the launch
    /// pad using `VestLimits` bounds.
    ///
    /// Code -10
    Limit,
    /// Raised when the soft cap requirement is not yet met
    /// for an operation.
    ///
    /// Code -11
    SoftCap,
    /// Raised when a certain operation is invoked after
    /// the launch-pad is completed.
    ///
    /// Code -12
    Completed,
    /// Raised when an operation or method can not be
    /// compeleted due to some underlying condition violation.
    ///
    /// Code -13
    JobFailed,
    /// Propagated from the logger log errors, log full.
    ///
    /// Code -14
    LogFull,
    /// Propagated from the logger log errors, log is
    /// malformed.
    ///
    /// Code -15
    LogMalformed,
    /// Propagated from the contract invocation, account
    /// has insufficient amount while invocation.
    ///
    /// Code -16
    AmountTooLarge,
    /// Propagated from the contract invocation, account
    /// is missing while invocation.
    ///
    /// Code -17
    MissingAccount,
    /// Propagated from the contract invocation, invalid
    /// response received.
    ///
    /// Code -18
    InvalidResponse,
    /// Propagated from the contract invocation, contract
    /// is missing while invocation.
    ///
    /// Code -19
    MissingContract,
    /// Propagated from the contract invocation, unable to
    /// find the entry-point in contract invocation.
    ///
    /// Code -20
    MissingEntrypoint,
    /// Propagated from the contract invocation, message failed.
    ///
    /// Code -21
    MessageFailed,
    /// Propagated from the contract invocation, trapped.
    ///
    /// Code -22
    Trap,
    /// Raised when a logic is rejected from CIS2 contract
    /// with the given reject reason.
    ///
    /// Code -23
    CIS2(i32),
    /// Raised when a logic is rejected from DEX contract
    /// with the given reject reason.
    ///
    /// Code -24
    DEX(i32),
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

// Mapping error received from logger `(LogError)` to the contract error.
impl From<LogError> for Error {
    fn from(value: LogError) -> Self {
        match value {
            LogError::Full => Self::LogFull,
            LogError::Malformed => Self::LogMalformed,
        }
    }
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

#[cfg(test)]
use concordium_smart_contract_testing::{
    ContractInvokeError, ContractInvokeErrorKind, InvokeFailure,
};

/// Mapping `ContractInvokeError` to `ContractError`
///
/// It parse any invocation error captured while integration testing to contract error
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
