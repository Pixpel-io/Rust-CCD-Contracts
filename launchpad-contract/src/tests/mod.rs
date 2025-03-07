use crate::{
    errors::LaunchPadError,
    params::{ApprovalParams, CreateParams},
    response::{LaunchPadView, StateView},
    state::Admin,
};
use concordium_cis2::{
    AdditionalData, BalanceOfQuery, BalanceOfQueryParams, BalanceOfQueryResponse, OperatorOfQuery,
    OperatorOfQueryParams, OperatorOfQueryResponse, OperatorUpdate, Receiver,
    TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID, Transfer, TransferParams, UpdateOperator,
    UpdateOperatorParams,
};
use concordium_smart_contract_testing::{
    module_load_v1, Account, AccountKeys, Chain, Energy, InitContractPayload, Signer,
    UpdateContractPayload,
};
use concordium_std::{
    AccountAddress, AccountBalance, Address, Amount, ContractAddress, Deserial, MetadataUrl,
    OwnedContractName, OwnedEntrypointName, OwnedParameter, OwnedReceiveName, SchemaType, Serial,
    ACCOUNT_ADDRESS_SIZE,
};

// mod bid;
// mod cancel;
// mod finalize;
// mod item;
mod smoke;

/// Dummy signer which always signs with one key
const SIGNER: Signer = Signer::with_one_key();

/// Account balance to initilize the test accounts
const ACC_INITIAL_BALANCE: Amount = Amount::from_ccd(10000);

const ADMIN: AccountAddress = AccountAddress([1; ACCOUNT_ADDRESS_SIZE]);
const OWNER: AccountAddress = AccountAddress([2; ACCOUNT_ADDRESS_SIZE]);

const HOLDERS: &'static [AccountAddress] = &[
    AccountAddress([3; ACCOUNT_ADDRESS_SIZE]),
    AccountAddress([4; ACCOUNT_ADDRESS_SIZE]),
    AccountAddress([5; ACCOUNT_ADDRESS_SIZE]),
];

const PLATFORM_REG_FEE: Amount = Amount::from_ccd(10);
const LIQUID_SHARE: u64 = 2;
const ALLOC_SHARE: u64 = 1;

const OWNER_TOKEN_ID: TokenID = TokenID(1);
const OWNER_TOKEN_URL: &str = "http://some.example/token/0";

/// A helper function to setup and initialize the auction and cis2_multi contracts as mocks
/// for unit testing.
///
/// It is required to build the auction contract in the root as `build/auction.wasm.v1` and
/// cis2_multi build should be present in path `test-build-artifacts/cis2multi.wasm.v1`
pub fn initialize_chain_and_launch_pad() -> (Chain, AccountKeys, ContractAddress, ContractAddress) {
    let mut chain = Chain::builder()
        .build()
        .expect("Should be able to build chain");

    // Create keys for ALICE.
    let rng = &mut rand::thread_rng();

    let keypairs_admin = AccountKeys::singleton(rng);

    let balance = AccountBalance {
        total: ACC_INITIAL_BALANCE,
        staked: Amount::zero(),
        locked: Amount::zero(),
    };

    // Create some accounts on the chain.
    chain.create_account(Account::new_with_keys(
        ADMIN,
        balance,
        (&keypairs_admin).into(),
    ));

    for acc_addr in [OWNER].iter().chain(HOLDERS.iter()) {
        chain.create_account(Account::new(*acc_addr, ACC_INITIAL_BALANCE));
    }

    // Load and deploy the cis2 token module.
    let module = module_load_v1("../nft-auction/test-build-artifacts/cis2multi.wasm.v1")
        .expect("Module exists");

    let deployment = chain
        .module_deploy_v1(SIGNER, ADMIN, module)
        .expect("Deploy valid module");

    let payload = InitContractPayload {
        amount: Amount::zero(),
        mod_ref: deployment.module_reference,
        init_name: OwnedContractName::new_unchecked("init_cis2_multi".to_string()),
        param: OwnedParameter::from_serial(&TokenAmount(10000u64)).expect("Serialize parameter"),
    };

    // Initialize the cis2 token contract.
    let token = chain
        .contract_init(SIGNER, ADMIN, Energy::from(10000), payload)
        .expect("Initialize cis2 token contract");

    // Load and deploy the auction module.
    let module = module_load_v1("build/launchpad.wasm.v1").expect("Module exists");
    let deployment = chain
        .module_deploy_v1(SIGNER, ADMIN, module)
        .expect("Deploy valid module");

    let admin_params = Admin {
        address: ADMIN,
        registeration_fee: PLATFORM_REG_FEE,
        liquidity_share: LIQUID_SHARE,
        allocation_share: ALLOC_SHARE,
        dex_address: ContractAddress::new(1001, 0),
    };

    let payload = InitContractPayload {
        amount: Amount::zero(),
        mod_ref: deployment.module_reference,
        init_name: OwnedContractName::new_unchecked("init_LaunchPad".to_string()),
        param: OwnedParameter::from_serial(&admin_params)
            .expect("[Error] Unable to Serialize Admin params"),
    };

    // Initialize the auction contract.
    let init_launch_pad = chain
        .contract_init(SIGNER, ADMIN, Energy::from(10000), payload)
        .expect("Initialize launch pad");

    (
        chain,
        keypairs_admin,
        init_launch_pad.contract_address,
        token.contract_address,
    )
}

fn initialize_contract<P>(
    chain: &mut Chain,
    module_path: String,
    contract_name: String,
    init_params: Option<P>,
) -> ContractAddress
where
    P: Serial,
{
    let module = module_load_v1(module_path.as_str()).expect("[Error] Unable to load module");
    let deploy = chain
        .module_deploy_v1(SIGNER, ADMIN, module)
        .expect("[Error] Unable to deploy");

    let owned_params = if let Some(params) = init_params {
        OwnedParameter::from_serial(&params).expect("[Error] Deserialization init params")
    } else {
        OwnedParameter::empty()
    };

    let payload = InitContractPayload {
        amount: Amount::zero(),
        mod_ref: deploy.module_reference,
        init_name: OwnedContractName::new_unchecked(format!("init_{}", contract_name)),
        param: owned_params,
    };

    chain
        .contract_init(SIGNER, ADMIN, Energy::from(10000), payload)
        .expect("[Error] Unable to initialize contract")
        .contract_address
}

fn update_contract<P, R>(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    params: P,
    payable: Option<Amount>,
    receive_name: OwnedReceiveName,
) -> Result<R, LaunchPadError>
where
    P: Serial,
    R: Deserial,
{
    let amount = match payable {
        Some(amount) => amount,
        None => Amount::zero(),
    };

    let payload = UpdateContractPayload {
        amount,
        address: contract,
        receive_name,
        message: OwnedParameter::from_serial(&params).unwrap(),
    };

    let result = chain.contract_update(
        SIGNER,
        invoker,
        Address::Account(invoker),
        Energy::from(10000),
        payload,
    );

    match result {
        Ok(success) => match success.parse_return_value() {
            Ok(ret_type) => Ok(ret_type),
            Err(pe) => Err(pe.into()),
        },
        Err(ce) => Err(ce.into()),
    }
}

fn read_contract<P, R>(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    params: P,
    receive_name: OwnedReceiveName,
) -> R
where
    P: Serial,
    R: Deserial,
{
    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        receive_name,
        address: contract,
        message: OwnedParameter::from_serial(&params).expect("BalanceOf params"),
    };

    let result = chain.contract_invoke(
        invoker,
        Address::Account(invoker),
        Energy::from(10000),
        payload,
    );

    result
        .unwrap()
        .parse_return_value()
        .expect("[Error] Unable to deserialize")
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
    pub token_id: TokenID,
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
    token_id: TokenID,
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
    invoker: AccountAddress,
    balance_of: Address,
    cis2_contract: ContractAddress,
    token_id: TokenID,
) -> BalanceOfQueryResponse<TokenAmount> {
    let balance_of_params: BalanceOfQueryParams<_> = BalanceOfQueryParams {
        queries: vec![BalanceOfQuery {
            token_id,
            address: balance_of,
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
            invoker,
            Address::Account(invoker),
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

/// A helper function which invokes `cis2_multi` to check if an account or contract is
/// operator of an owner in ci2_contract
///
/// This is useful for integration testing
fn deposit_tokens_to_launch_pad(
    chain: &mut Chain,
    invoker: AccountAddress,
    product_name: String,
    cis2_contract: ContractAddress,
    launch_pad_contract: ContractAddress,
) {
    let transfer_params = TransferParams::<TokenID, TokenAmount>(vec![Transfer {
        token_id: OWNER_TOKEN_ID,
        amount: TokenAmount(10000),
        from: Address::Account(OWNER),
        to: Receiver::Contract(
            launch_pad_contract,
            OwnedEntrypointName::new_unchecked("Deposit".to_string()),
        ),
        data: AdditionalData::from(product_name.as_bytes().to_owned()),
    }]);

    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        receive_name: OwnedReceiveName::new_unchecked("cis2_multi.transfer".to_string()),
        address: cis2_contract,
        message: OwnedParameter::from_serial(&transfer_params)
            .expect("[Error] Unable to serialize UpdateOperator params"),
    };

    chain
        .contract_update(
            SIGNER,
            invoker,
            Address::Account(invoker),
            Energy::from(10000),
            payload,
        )
        .expect("[Error] Deposit Failed");
}

// /// A helper function to invoke `viewItemState` in auction to get a specefic
// /// item's current state in the auction contract
// ///
// /// Returns the `ItemState` type or panics with error message
// fn get_item_state(
//     chain: &Chain,
//     contract: ContractAddress,
//     account: AccountAddress,
//     item_index: u16,
// ) -> ItemState {
//     let view_item_params = item_index;

//     let payload = UpdateContractPayload {
//         amount: Amount::from_ccd(0),
//         address: contract,
//         receive_name: OwnedReceiveName::new_unchecked("cis2-auction.viewItemState".to_string()),
//         message: OwnedParameter::from_serial(&view_item_params)
//             .expect("[Error] Unable to serialize view item params"),
//     };

//     let item: ItemState = chain
//         .contract_invoke(
//             account,
//             Address::Account(account),
//             Energy::from(10000),
//             payload,
//         )
//         .expect("[Error] Invocation failed while invoking 'addItem' ")
//         .parse_return_value()
//         .expect("[Error] Unable to deserialize ItemState");

//     item
// }

/// A helper function to invoke `viewItemState` in auction to get a specefic
/// item's current state in the auction contract
///
/// Returns the `ItemState` type or panics with error message
fn view_state(chain: &Chain, contract: ContractAddress, account: AccountAddress) -> StateView {
    let payload = UpdateContractPayload {
        amount: Amount::from_ccd(0),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("LaunchPad.viewState".to_string()),
        message: OwnedParameter::empty(),
    };

    let item: StateView = chain
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

// /// A helper function to invoke `bid` function in auction contract to bid on an
// /// item listed for auction
// ///
// /// Returns the `Ok()` if the invocation succeeds or else `auction::Error`
// fn bid_on_item(
//     chain: &mut Chain,
//     contract: ContractAddress,
//     invoker: AccountAddress,
//     sender: Address,
//     amount: Amount,
//     bid_params: BidParams,
// ) -> Result<(), Error> {
//     let payload = UpdateContractPayload {
//         amount,
//         address: contract,
//         receive_name: OwnedReceiveName::new_unchecked("cis2-auction.bid".to_string()),
//         message: OwnedParameter::from_serial(&bid_params)
//             .expect("[Error] Unable to serialize bid_params"),
//     };

//     // BOB bids on the item added by ALICE
//     let invoke_result =
//         chain.contract_update(SIGNER, invoker, sender, Energy::from(10000), payload);

//     match invoke_result {
//         Ok(_) => Ok(()),
//         Err(err) => Err(err.into()),
//     }
// }

/// A helper function to invoke `CreatLaunchPad` function in launch pad contract to list an
/// product for ICO presale
///
/// Returns the `Ok()` if the invocation succeeds or else `LaunchPadError::Error`
fn add_launch_pad(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    add_params: CreateParams,
) -> Result<(), LaunchPadError> {
    let payload = UpdateContractPayload {
        amount: PLATFORM_REG_FEE,
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("LaunchPad.CreateLaunchPad".to_string()),
        message: OwnedParameter::from_serial(&add_params)
            .expect("[Error] Unable to serialize bid_params"),
    };

    // Adds the product for presale as launch pad
    let invoke_result = chain.contract_update(
        SIGNER,
        invoker,
        Address::Account(invoker),
        Energy::from(10000),
        payload,
    );

    match invoke_result {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// A helper function to invoke `CreatLaunchPad` function in launch pad contract to list an
/// product for ICO presale
///
/// Returns the `Ok()` if the invocation succeeds or else `LaunchPadError::Error`
fn approve_launch_pad(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    approval_params: ApprovalParams,
) -> Result<(), LaunchPadError> {
    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("LaunchPad.ApproveLaunchPad".to_string()),
        message: OwnedParameter::from_serial(&approval_params)
            .expect("[Error] Unable to serialize bid_params"),
    };

    // Adds the product for presale as launch pad
    let invoke_result = chain.contract_update(
        SIGNER,
        invoker,
        Address::Account(invoker),
        Energy::from(10000),
        payload,
    );

    match invoke_result {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// A helper function to invoke `CreatLaunchPad` function in launch pad contract to list an
/// product for ICO presale
///
/// Returns the `Ok()` if the invocation succeeds or else `LaunchPadError::Error`
fn get_launch_pad(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    product_name: String,
) -> LaunchPadView {
    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked("LaunchPad.viewLaunchPad".to_string()),
        message: OwnedParameter::from_serial::<String>(&product_name)
            .expect("[Error] Serialization"),
    };

    // Adds the product for presale as launch pad
    let invoke_result = chain.contract_invoke(
        invoker,
        Address::Account(invoker),
        Energy::from(10000),
        payload,
    );

    match invoke_result {
        Ok(success) => success.parse_return_value().unwrap(),
        Err(err) => panic!("[Error::get_launch_pad] {:?}", LaunchPadError::from(err)),
    }
}
// /// A helper function to invoke `finalize` function in auction contract for item finalization
// /// listed in active auctions
// ///
// /// Returns the `Ok()` if the invocation succeeds or else `auction::Error`
// fn finalize_auction(
//     chain: &mut Chain,
//     contract: ContractAddress,
//     invoker: AccountAddress,
//     sender: Address,
//     item_index: u16,
// ) -> Result<(), Error> {
//     let item_index_params = item_index;

//     let payload = UpdateContractPayload {
//         amount: Amount::zero(),
//         address: contract,
//         receive_name: OwnedReceiveName::new_unchecked("cis2-auction.finalize".to_string()),
//         message: OwnedParameter::from_serial(&item_index_params)
//             .expect("[Error] Unable to serialize bid_params"),
//     };

//     // BOB bids on the item added by ALICE
//     let invoke_result =
//         chain.contract_update(SIGNER, invoker, sender, Energy::from(10000), payload);

//     match invoke_result {
//         Ok(_) => Ok(()),
//         Err(err) => Err(err.into()),
//     }
// }

// /// Mapping `ContractInvokeError` to `auction::error::Error`
// ///
// /// It parse any invocation error captured while integration testing to contract error
// impl From<ContractInvokeError> for Error {
//     fn from(value: ContractInvokeError) -> Self {
//         if let ContractInvokeErrorKind::ExecutionError { failure_kind } = value.kind {
//             if let InvokeFailure::ContractReject { code: _, data } = failure_kind {
//                 data[0].into()
//             } else {
//                 panic!("[Error] Unable to map received invocation error code")
//             }
//         } else {
//             panic!("[Error] Unable to map ContractInvokeError other than ExecutionError")
//         }
//     }
// }
