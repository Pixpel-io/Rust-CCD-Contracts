use concordium_cis2::{
    AdditionalData, BalanceOfQuery, BalanceOfQueryParams, BalanceOfQueryResponse, OperatorUpdate,
    Receiver, TokenAmountU64 as TokenAmount, TokenIdU64, TokenIdU8 as TokenID, Transfer,
    TransferParams, UpdateOperator, UpdateOperatorParams,
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

use crate::{
    errors::Error,
    params::{BuyerParams, InitParams, ListParams},
    response::Token,
    ContractTokenAmount, ContractTokenId,
};

mod smoke;

/// Dummy signer which always signs with one key
const SIGNER: Signer = Signer::with_one_key();

/// Account balance to initilize the test accounts
const ACC_INITIAL_BALANCE: Amount = Amount::from_ccd(20000);

const SELLER: AccountAddress = AccountAddress([1; ACCOUNT_ADDRESS_SIZE]);
const BUYER: AccountAddress = AccountAddress([2; ACCOUNT_ADDRESS_SIZE]);
const ADMIN: AccountAddress = AccountAddress([3; ACCOUNT_ADDRESS_SIZE]);

const SELLER_TOKEN_ID_1: TokenID = TokenID(1);
const SELLER_TOKEN_URL_1: &str = "http://some.example/token/0";

const SELLER_TOKEN_ID_2: TokenID = TokenID(3);
const SELLER_TOKEN_URL_2: &str = "http://some.example/token/2";

const PIXP_TOKEN_ID: TokenID = TokenID(2);
const PIXP_TOKEN_URL: &str = "http://some.example/token/1";

pub fn initialize_chain_and_contracts() -> (Chain, AccountKeys, ContractAddress, ContractAddress) {
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

    for acc_addr in [SELLER, BUYER].iter() {
        chain.create_account(Account::new(*acc_addr, ACC_INITIAL_BALANCE));
    }

    // Load and deploy the cis2 token module.
    let cis2_contract = initialize_contract(
        &mut chain,
        "../auction/test-build-artifacts/cis2multi.wasm.v1",
        "cis2_multi",
        TokenAmount(10000u64),
    );

    // Load and deploy the main market place module.
    let market_place = initialize_contract(
        &mut chain,
        "build/market-place.wasm.v1",
        "Market-Place",
        InitParams {
            commission: 1,
            admin: ADMIN,
            pixp_id: PIXP_TOKEN_ID,
            pixp_address: cis2_contract,
        },
    );

    (chain, keypairs_admin, market_place, cis2_contract)
}

fn initialize_contract<P>(
    chain: &mut Chain,
    module_path: &str,
    contract_name: &str,
    init_params: P,
) -> ContractAddress
where
    P: Serial,
{
    let module = module_load_v1(module_path).expect("[Error] Unable to load module");
    let deploy = chain
        .module_deploy_v1(SIGNER, ADMIN, module)
        .expect("[Error] Unable to deploy");

    let owned_params = OwnedParameter::from_serial(&init_params).unwrap();

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
    receive_name: &str,
) -> Result<R, Error>
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
        receive_name: OwnedReceiveName::new_unchecked(receive_name.to_string()),
        message: OwnedParameter::from_serial(&params).unwrap(),
    };

    let result = chain.contract_update(
        SIGNER,
        invoker,
        Address::Account(invoker),
        Energy::from(20000),
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
    receive_name: &str,
) -> R
where
    P: Serial,
    R: Deserial,
{
    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked(receive_name.to_string()),
        message: OwnedParameter::from_serial(&params).expect("[Error] Unable to parse params"),
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
        .expect("[Error] Unable to deserialize response")
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

impl From<(AccountAddress, TokenID, String)> for MintParams {
    fn from(value: (AccountAddress, TokenID, String)) -> Self {
        Self {
            to: Receiver::from_account(value.0),
            metadata_url: MetadataUrl {
                url: value.2,
                hash: None,
            },
            token_id: value.1,
            data: AdditionalData::empty(),
        }
    }
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
    let params = MintParams::from((account, token_id, url));

    update_contract::<_, ()>(
        chain,
        cis2_contract,
        account,
        params,
        None,
        "cis2_multi.mint",
    )
    .expect("[Error] Mint Failed");
}

fn transfer_tokens(
    chain: &mut Chain,
    account: AccountAddress,
    params: Transfer<ContractTokenId, ContractTokenAmount>,
    contract: ContractAddress,
) {
    let transfer_params = TransferParams(vec![params]);

    update_contract::<_, ()>(
        chain,
        contract,
        account,
        transfer_params,
        None,
        "cis2_multi.transfer",
    )
    .expect("[Error] Token Tranfer Failed");
}

/// A helper function which invokes `cis2_multi` contract to get the balance of specific tokens minted
/// for a specifi account.
///
/// This is useful for integration testing
pub fn get_token_balance(
    chain: &mut Chain,
    invoker: AccountAddress,
    balance_of: &[(Address, TokenID)],
    cis2_contract: ContractAddress,
) -> BalanceOfQueryResponse<TokenAmount> {
    let queries: Vec<_> = balance_of
        .iter()
        .map(|(address, token_id)| BalanceOfQuery {
            token_id: *token_id,
            address: *address,
        })
        .collect::<Vec<_>>();

    read_contract(
        chain,
        cis2_contract,
        invoker,
        BalanceOfQueryParams { queries },
        "cis2_multi.balanceOf",
    )
}

/// A helper function which invokes `cis2_multi` contract to update the operator of a certain
/// account or contract.
///
/// This is useful for integration testing
fn update_operator_of(
    chain: &mut Chain,
    invoker: AccountAddress,
    operator_to_be: Address,
    cis2_contract: ContractAddress,
) -> Result<(), Error> {
    let update_operator_params = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Add,
        operator: operator_to_be,
    }]);

    update_contract(
        chain,
        cis2_contract,
        invoker,
        update_operator_params,
        None,
        "cis2_multi.updateOperator",
    )
}

fn buy_token(
    chain: &mut Chain,
    params: BuyerParams,
    amount: Amount,
    invoker: AccountAddress,
    contract: ContractAddress,
) -> Result<(), Error> {
    update_contract(
        chain,
        contract,
        invoker,
        params,
        Some(amount),
        "Market-Place.BuyToken",
    )
}

fn list_token(
    chain: &mut Chain,
    invoker: AccountAddress,
    params: ListParams,
    contract: ContractAddress,
) -> Result<(), Error> {
    update_contract(
        chain,
        contract,
        invoker,
        params,
        None,
        "Market-Place.ListToken",
    )
}

fn view_token_list(
    chain: &mut Chain,
    invoker: AccountAddress,
    contract: ContractAddress,
) -> Vec<Token> {
    read_contract::<_, _>(chain, contract, invoker, (), "Market-Place.ViewTokenList")
}
