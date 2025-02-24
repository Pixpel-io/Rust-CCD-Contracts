use concordium_std::*;
// use concordium_cis2::Cis2Error;


#[derive(Serialize, Debug, PartialEq, Eq, Reject, SchemaType)]
pub enum ContractError {
    #[from(ParseError)]
    ParseParamsError,

    ExchangeNotFound,
    ExchangeAlreadyExists,

    LogFull,
    LogMalformed,
    InvalidContractName,
    ContractOnly,

    InvokeContractError,
    InvokeContractNoResult,
    InvokeTransferError,
    ParseParams,
    ParseResult,

    InvalidTokenId,
    InsufficientFunds,
    Unauthorized,

    IncorrectTokenCcdRatio,
    TokenNotCis2,
    NotOperator,
    CalledByAContract,
    AmountTooLarge,
    MissingAccount,

    MissingContract,
    MissingEntrypoint,
    MessageFailed,
    LogicReject {
        reason:       i32,
        // return_value: Vec<u8>,
    },
    Trap,

    // Swaps
    InsufficientOutputAmount,
    InvalidReserves,
}


impl From<TransferError> for ContractError {
    fn from(le: TransferError) -> Self {
        match le {
            TransferError::AmountTooLarge => Self::AmountTooLarge,
            TransferError::MissingAccount => Self::MissingAccount,
        }
    }
}

impl From<LogError> for ContractError {
    fn from(le: LogError) -> Self {
        match le {
            LogError::Full => Self::LogFull,
            LogError::Malformed => Self::LogMalformed,
        }
    }
}

impl<T> From<CallContractError<T>> for ContractError {
    fn from(cce: CallContractError<T>) -> Self {
        match cce {
            CallContractError::AmountTooLarge => Self::AmountTooLarge,
            CallContractError::MissingAccount => Self::MissingAccount,
            CallContractError::MissingContract => Self::MissingContract,
            CallContractError::MissingEntrypoint => Self::MissingEntrypoint,
            CallContractError::MessageFailed => Self::MessageFailed,
            CallContractError::LogicReject {
                reason, return_value: _
            } => Self::LogicReject {
                reason,
                // return_value: return_value.into()
            },
            CallContractError::Trap => Self::Trap,
        }
    }
}

impl From<NewContractNameError> for ContractError {
    fn from(_: NewContractNameError) -> Self { Self::InvalidContractName }
}
