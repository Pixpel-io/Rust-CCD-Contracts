use concordium_std::*;
// use concordium_cis2::Cis2Error;

/*
 * OVERVIEW OF errors.rs
 *
 * This file defines the `ContractError` enum, which represents all possible errors that can occur in the `pixpel_swap`
 * smart contract on the Concordium blockchain. It also implements conversions from standard Concordium errors (e.g.,
 * `TransferError`, `LogError`, `CallContractError`, `NewContractNameError`) into `ContractError` variants. This allows
 * the contract to handle both custom errors and platform-specific errors uniformly.
 *
 * ENUM: `ContractError`
 * - Purpose: Enumerates all error conditions that can arise during contract execution, categorized into general errors,
 *   invocation errors, token-related errors, and swap-specific errors.
 * - Attributes: `Serialize`, `Debug`, `PartialEq`, `Eq`, `Reject`, `SchemaType` (for serialization, debugging, equality checks,
 *   rejection in Concordium, and schema generation).
 * - Variants:
 *   - `ParseParamsError`: Parsing input parameters failed (converted from `ParseError`).
 *   - `ExchangeNotFound`: No exchange exists for a token.
 *   - `ExchangeAlreadyExists`: An exchange already exists for a token.
 *   - `LogFull`: Event log is full (from `LogError::Full`).
 *   - `LogMalformed`: Event log format is invalid (from `LogError::Malformed`).
 *   - `InvalidContractName`: Contract name is invalid (from `NewContractNameError`).
 *   - `ContractOnly`: Operation restricted to contracts only.
 *   - `InvokeContractError`: General error invoking another contract.
 *   - `InvokeContractNoResult`: Contract invocation returned no result.
 *   - `InvokeTransferError`: Failed to transfer CCD.
 *   - `ParseParams`: Duplicate of `ParseParamsError` (possibly redundant, consider consolidating).
 *   - `ParseResult`: Failed to parse a contract invocation result.
 *   - `InvalidTokenId`: Token ID is invalid or not found.
 *   - `InsufficientFunds`: Not enough funds (CCD or tokens) for an operation.
 *   - `Unauthorized`: Sender lacks permission (e.g., not an operator).
 *   - `IncorrectTokenCcdRatio`: Token/CCD ratio for liquidity addition is incorrect.
 *   - `TokenNotCis2`: Token contract doesn’t support CIS-2 standard.
 *   - `NotOperator`: Sender is not an operator for a token operation.
 *   - `CalledByAContract`: Operation restricted to accounts, not contracts.
 *   - `AmountTooLarge`: Amount exceeds allowable limit (from `TransferError` or `CallContractError`).
 *   - `MissingAccount`: Account not found (from `TransferError` or `CallContractError`).
 *   - `MissingContract`: Target contract not found (from `CallContractError`).
 *   - `MissingEntrypoint`: Entrypoint not found in target contract (from `CallContractError`).
 *   - `MessageFailed`: Contract invocation message failed (from `CallContractError`).
 *   - `LogicReject { reason: i32 }`: Contract logic rejected the call with a reason (from `CallContractError`).
 *   - `Trap`: Execution trapped (unexpected error, from `CallContractError`).
 *   - `InsufficientOutputAmount`: Swap output below minimum specified amount.
 *   - `InvalidReserves`: Reserves (CCD or token) are zero or invalid for a swap.
 *
 * IMPLEMENTATIONS:
 * - `From<TransferError> for ContractError`:
 *   - Purpose: Converts `TransferError` (CCD transfer errors) into `ContractError`.
 *   - Mapping: `AmountTooLarge` -> `AmountTooLarge`, `MissingAccount` -> `MissingAccount`.
 *   - Usage: Handles CCD transfer failures in operations like `removeLiquidity`.
 * - `From<LogError> for ContractError`:
 *   - Purpose: Converts `LogError` (event logging errors) into `ContractError`.
 *   - Mapping: `Full` -> `LogFull`, `Malformed` -> `LogMalformed`.
 *   - Usage: Handles logging issues in functions like `lpt_transfer` or `lp_add_liquidity`.
 * - `From<CallContractError<T>> for ContractError`:
 *   - Purpose: Converts `CallContractError` (contract invocation errors) into `ContractError`.
 *   - Mapping: `AmountTooLarge` -> `AmountTooLarge`, `MissingAccount` -> `MissingAccount`, `MissingContract` -> `MissingContract`,
 *             `MissingEntrypoint` -> `MissingEntrypoint`, `MessageFailed` -> `MessageFailed`, `LogicReject` -> `LogicReject`,
 *             `Trap` -> `Trap`.
 *   - Usage: Handles errors from invoking other contracts (e.g., CIS-2 token transfers).
 * - `From<NewContractNameError> for ContractError`:
 *   - Purpose: Converts `NewContractNameError` (invalid contract name) into `ContractError`.
 *   - Mapping: Any error -> `InvalidContractName`.
 *   - Usage: Rare, typically for initialization or contract creation issues.
 *
 * NOTES FOR DEVELOPERS:
 * - Errors are thrown using `ensure!` or `bail!` macros in the contract code (see `contract.rs`).
 * - Some variants (e.g., `ParseParams` vs. `ParseParamsError`) may be redundant; consider streamlining.
 * - The `LogicReject` variant includes a `reason` field but comments out `return_value` (uncomment if needed for debugging).
 * - Extend this enum if new error conditions arise (e.g., new swap types or liquidity rules).
 */

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
