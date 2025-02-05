use crate::{
    error::Error,
    params::{AddItemParameter, BidParams},
    state::ItemState,
};
use concordium_cis2::{
    AdditionalData, BalanceOfQuery, BalanceOfQueryParams, BalanceOfQueryResponse, OperatorOfQuery,
    OperatorOfQueryParams, OperatorOfQueryResponse, OperatorUpdate, Receiver, TokenAmountU64,
    TokenIdU8, UpdateOperator, UpdateOperatorParams,
};
use concordium_smart_contract_testing::{
    module_load_v1, Account, AccountKeys, Chain, ContractInvokeError, ContractInvokeErrorKind,
    Energy, InitContractPayload, InvokeFailure, Signer, UpdateContractPayload,
};
use concordium_std::{
    AccountAddress, AccountBalance, Address, Amount, ContractAddress, MetadataUrl,
    OwnedContractName, OwnedParameter, OwnedReceiveName, SchemaType, Serial,
};
use concordium_std_derive::account_address;

mod bid;
mod cancel;
mod finalize;
mod item;
mod smoke;

/// Dummy addresses as raw bytes
pub const ALICE: AccountAddress =
    account_address!("2xBpaHottqhwFZURMZW4uZduQvpxNDSy46iXMYs9kceNGaPpZX");
pub const BOB: AccountAddress =
    account_address!("2xdTv8awN1BjgYEw8W1BVXVtiEwG2b29U8KoZQqJrDuEqddseE");
pub const CAROL: AccountAddress =
    account_address!("2y57FyMyqAfY7X1SuSWJ5VMt1Z3ZgxbKt9w5mGoTwqA7YcpbXr");

/// Address types for dummy addresses
pub const ALICE_ADDR: Address = Address::Account(ALICE);
pub const BOB_ADDR: Address = Address::Account(BOB);
pub const CAROL_ADDR: Address = Address::Account(CAROL);

/// Dummy signer which always signs with one key
pub const SIGNER: Signer = Signer::with_one_key();

/// Account balance to initilize the test accounts
pub const ACC_INITIAL_BALANCE: Amount = Amount::from_ccd(10000);

/// A helper function to setup and initialize the auction and cis2_multi contracts as mocks
/// for unit testing.
///
/// It is required to build the auction contract in the root as `build/auction.wasm.v1` and
/// cis2_multi build should be present in path `test-build-artifacts/cis2multi.wasm.v1`
pub fn initialize_chain_and_auction() -> (Chain, AccountKeys, ContractAddress, ContractAddress) {
    let mut chain = Chain::builder()
        .build()
        .expect("Should be able to build chain");

    // Create keys for ALICE.
    let rng = &mut rand::thread_rng();

    let keypairs_alice = AccountKeys::singleton(rng);

    let balance = AccountBalance {
        total: ACC_INITIAL_BALANCE,
        staked: Amount::zero(),
        locked: Amount::zero(),
    };

    // Create some accounts on the chain.
    chain.create_account(Account::new_with_keys(
        ALICE,
        balance,
        (&keypairs_alice).into(),
    ));
    chain.create_account(Account::new(BOB, ACC_INITIAL_BALANCE));
    chain.create_account(Account::new(CAROL, ACC_INITIAL_BALANCE));

    // Load and deploy the cis2 token module.
    let module = module_load_v1("./test-build-artifacts/cis2multi.wasm.v1").expect("Module exists");

    let deployment = chain
        .module_deploy_v1(SIGNER, CAROL, module)
        .expect("Deploy valid module");

    let payload = InitContractPayload {
        amount: Amount::zero(),
        mod_ref: deployment.module_reference,
        init_name: OwnedContractName::new_unchecked("init_cis2_multi".to_string()),
        param: OwnedParameter::from_serial(&TokenAmountU64(100u64)).expect("Serialize parameter"),
    };

    // Initialize the cis2 token contract.
    let token = chain
        .contract_init(SIGNER, CAROL, Energy::from(10000), payload)
        .expect("Initialize cis2 token contract");

    // Load and deploy the auction module.
    let module = module_load_v1("build/auction.wasm.v1").expect("Module exists");
    let deployment = chain
        .module_deploy_v1(SIGNER, CAROL, module)
        .expect("Deploy valid module");

    let payload = InitContractPayload {
        amount: Amount::zero(),
        mod_ref: deployment.module_reference,
        init_name: OwnedContractName::new_unchecked("init_cis2-auction".to_string()),
        param: OwnedParameter::empty(),
    };

    // Initialize the auction contract.
    let init_auction = chain
        .contract_init(SIGNER, CAROL, Energy::from(10000), payload)
        .expect("Initialize auction");

    (
        chain,
        keypairs_alice,
        init_auction.contract_address,
        token.contract_address,
    )
}

/// The parameter for the contract function `mint` which mints/airdrops a number
/// of tokens to the owner's address.
#[derive(Serial, SchemaType, Clone)]
pub struct MintParams {
    /// Owner of the newly minted tokens.
    pub to: Receiver,
    /// The metadata_url of the token.
    pub metadata_url: MetadataUrl,
    /// The token_id to mint/create additional tokens.
    pub token_id: TokenIdU8,
    /// Additional data that can be sent to the receiving contract.
    pub data: AdditionalData,
}

/// A helper function which invokes `cis2_multi` contract to `mint` airdrop tokens for the given
/// account.
///
/// This is useful for minting some mock tokens to be tested in integration tests by auction
/// contract
pub fn mint_token(
    chain: &mut Chain,
    account: AccountAddress,
    cis2_contract: ContractAddress,
    token_id: TokenIdU8,
    url: String,
) {
    let params = MintParams {
        to: Receiver::from_account(account),
        metadata_url: MetadataUrl { url, hash: None },
        token_id,
        data: AdditionalData::empty(),
    };

    let payload = UpdateContractPayload {
        amount: Amount::from_ccd(0),
        address: cis2_contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2_multi.mint".to_string()),
        message: OwnedParameter::from_serial(&params).expect("[Error] Serialization Failed"),
    };

    let _ = chain
        .contract_update(
            SIGNER,
            account,
            Address::Account(account),
            Energy::from(10000),
            payload,
        )
        .expect("[Error] Mint Failed");
}

/// A helper function which invokes `cis2_multi` contract to get the balance of specific tokens minted
/// for a specifi account.
///
/// This is useful for integration testing
pub fn get_token_balance(
    chain: &Chain,
    account: AccountAddress,
    cis2_contract: ContractAddress,
    token_id: TokenIdU8,
) -> BalanceOfQueryResponse<TokenAmountU64> {
    let balance_of_params: BalanceOfQueryParams<_> = BalanceOfQueryParams {
        queries: vec![BalanceOfQuery {
            token_id,
            address: Address::Account(account),
        }],
    };

    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        receive_name: OwnedReceiveName::new_unchecked("cis2_multi.balanceOf".to_string()),
        address: cis2_contract,
        message: OwnedParameter::from_serial(&balance_of_params).expect("BalanceOf params"),
    };

    let invoke = chain
        .contract_invoke(
            account,
            Address::Account(account),
            Energy::from(10000),
            payload,
        )
        .expect("[Error] Balance_Of query Invocation failed");

    invoke
        .parse_return_value()
        .expect("[Error] Unable to deserialize response Balance_Of quary")
}

/// A helper function which invokes `cis2_multi` contract to update the operator of a certain
/// account or contract.
///
/// This is useful for integration testing
fn update_operator_of(
    chain: &mut Chain,
    invoker: AccountAddress,
    sender: Address,
    operator_to_be: Address,
    cis2_contract: ContractAddress,
) {
    let update_operator_params = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Add,
        operator: operator_to_be,
    }]);

    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        receive_name: OwnedReceiveName::new_unchecked("cis2_multi.updateOperator".to_string()),
        address: cis2_contract,
        message: OwnedParameter::from_serial(&update_operator_params)
            .expect("[Error] Unable to serialize UpdateOperator params"),
    };

    let _ = chain
        .contract_update(SIGNER, invoker, sender, Energy::from(10000), payload)
        .expect("[Error] Unable to Update Operator, invocation failed");
}

/// A helper function which invokes `cis2_multi` to check if an account or contract is
/// operator of an owner in ci2_contract
///
/// This is useful for integration testing
fn ensure_is_operator_of(
    chain: &mut Chain,
    invoker: AccountAddress,
    sender: Address,
    is_operator: Address,
    cis2_contract: ContractAddress,
) -> bool {
    let is_operator_params = OperatorOfQueryParams {
        queries: vec![OperatorOfQuery {
            owner: sender,
            address: is_operator,
        }],
    };

    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        receive_name: OwnedReceiveName::new_unchecked("cis2_multi.operatorOf".to_string()),
        address: cis2_contract,
        message: OwnedParameter::from_serial(&is_operator_params)
            .expect("[Error] Unable to serialize UpdateOperator params"),
    };

    let response: OperatorOfQueryResponse = chain
        .contract_invoke(invoker, sender, Energy::from(10000), payload)
        .expect("[Error] Unable to Update Operator, invocation failed")
        .parse_return_value()
        .expect("[Error] Unable parse OperatorOfQueryResponse");

    response.0[0]
}

/// A helper function to invoke `viewItemState` in auction to get a specefic
/// item's current state in the auction contract
///
/// Returns the `ItemState` type or panics with error message
fn get_item_state(
    chain: &Chain,
    contract: ContractAddress,
    account: AccountAddress,
    item_index: u16,
) -> ItemState {
    let view_item_params = item_index;

    let payload = UpdateContractPayload {
        amount: Amount::from_ccd(0),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.viewItemState".to_string()),
        message: OwnedParameter::from_serial(&view_item_params)
            .expect("[Error] Unable to serialize view item params"),
    };

    let item: ItemState = chain
        .contract_invoke(
            account,
            Address::Account(account),
            Energy::from(10000),
            payload,
        )
        .expect("[Error] Invocation failed while invoking 'addItem' ")
        .parse_return_value()
        .expect("[Error] Unable to deserialize ItemState");

    item
}

/// A helper function to invoke `bid` function in auction contract to bid on an
/// item listed for auction
///
/// Returns the `Ok()` if the invocation succeeds or else `auction::Error`
fn bid_on_item(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    sender: Address,
    amount: Amount,
    bid_params: BidParams,
) -> Result<(), Error> {
    let payload = UpdateContractPayload {
        amount,
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.bid".to_string()),
        message: OwnedParameter::from_serial(&bid_params)
            .expect("[Error] Unable to serialize bid_params"),
    };

    // BOB bids on the item added by ALICE
    let invoke_result =
        chain.contract_update(SIGNER, invoker, sender, Energy::from(10000), payload);

    match invoke_result {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// A helper function to invoke `addItem` function in auction contract to list an
/// item for auction
///
/// Returns the `Ok()` if the invocation succeeds or else `auction::Error`
fn add_item_for_auction(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    sender: Address,
    add_item_params: AddItemParameter,
) -> Result<(), Error> {
    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.addItem".to_string()),
        message: OwnedParameter::from_serial(&add_item_params)
            .expect("[Error] Unable to serialize bid_params"),
    };

    // BOB bids on the item added by ALICE
    let invoke_result =
        chain.contract_update(SIGNER, invoker, sender, Energy::from(10000), payload);

    match invoke_result {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// A helper function to invoke `finalize` function in auction contract for item finalization
/// listed in active auctions
///
/// Returns the `Ok()` if the invocation succeeds or else `auction::Error`
fn finalize_auction(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    sender: Address,
    item_index: u16,
) -> Result<(), Error> {
    let item_index_params = item_index;

    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("cis2-auction.finalize".to_string()),
        message: OwnedParameter::from_serial(&item_index_params)
            .expect("[Error] Unable to serialize bid_params"),
    };

    // BOB bids on the item added by ALICE
    let invoke_result =
        chain.contract_update(SIGNER, invoker, sender, Energy::from(10000), payload);

    match invoke_result {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// Mapping `ContractInvokeError` to `auction::error::Error`
///
/// It parse any invocation error captured while integration testing to contract error
impl From<ContractInvokeError> for Error {
    fn from(value: ContractInvokeError) -> Self {
        if let ContractInvokeErrorKind::ExecutionError { failure_kind } = value.kind {
            if let InvokeFailure::ContractReject { code: _, data } = failure_kind {
                data[0].into()
            } else {
                panic!("[Error] Unable to map received invocation error code")
            }
        } else {
            panic!("[Error] Unable to map ContractInvokeError other than ExecutionError")
        }
    }
}
