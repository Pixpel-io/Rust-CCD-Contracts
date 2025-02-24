use concordium_std::*;

/*
 * OVERVIEW OF tests.rs
 *
 * This file contains the unit tests for the `pixpel_swap` smart contract on the Concordium blockchain, using the `concordium_cfg_test`
 * framework. It tests the contract's initialization, LP token operations (transfer, operator updates, balance queries), and liquidity
 * pool functionality (add/remove liquidity). The tests use a mock environment provided by `concordium_std::test_infrastructure`.
 *
 * CONSTANTS:
 * - `ACCOUNT_DEPLOYER`, `ACCOUNT_USER`, `ACCOUNT_ANOTHER_USER`, `ACCOUNT_OPERATOR`, `ACCOUNT_VILLAIN`: Predefined account addresses for testing.
 * - `ADDRESS_*`: Corresponding `Address` variants for the above accounts.
 * - `SWAP_INDEX: u64 = 500`: Contract index for the `pixpel_swap` contract in tests.
 *
 * HELPER STRUCTS AND FUNCTIONS:
 * - `CallResult<T>`: Wraps test results with a result value, logger, and host state for inspection.
 *   - Fields: `result: T` (test outcome), `logger: TestLogger` (logged events), `host: TestHost<State<TestStateApi>>` (state after execution).
 * - `get_token_index(number: u64) -> u64`: Generates a token contract index (1000 + number).
 * - `get_token(number: u64) -> TokenInfo`: Creates a `TokenInfo` with a fixed token ID and contract address based on `number`.
 * - `get_ctx(address_sender: Address) -> TestReceiveContext`: Sets up a test context with sender and contract address.
 * - `get_host() -> TestHost<State<TestStateApi>>`: Initializes a fresh test host with an empty state.
 * - `mock_cis2_supports`, `mock_cis2_operator_of`, `mock_cis2_balance_of`, `mock_cis2_transfer`: Mock CIS-2 entrypoints (`supports`, `operatorOf`, `balanceOf`, `transfer`) for token contract interactions.
 * - `expect_error`: Asserts that a `Result` is an error and matches the expected value, with a custom message.
 *
 * TESTS:
 * - `test_0010_init_success`:
 *   - Purpose: Verifies successful contract initialization.
 *   - Setup: Empty context and state builder.
 *   - Assertion: `init` returns `Ok` with an empty state.
 *   - Errors: Fails if initialization returns an unexpected error.
 *
 * - `mint_and_transfer(sender: Address, amount: ContractTokenAmount) -> CallResult<ContractResult<()>>`:
 *   - Purpose: Helper to mint LP tokens and test transfers, used by multiple tests.
 *   - Setup: Mints 1B tokens to `ADDRESS_USER`, adds `ADDRESS_OPERATOR`, attempts a transfer.
 *   - Returns: `CallResult` with transfer result, logger, and host state.
 *
 * - `test_0020_lpt_transfer_success`:
 *   - Purpose: Tests successful LP token transfer from owner.
 *   - Setup: Uses `mint_and_transfer` with `ADDRESS_USER` as sender.
 *   - Assertions: Transfer succeeds, balances update (0 for sender, 1B for receiver), one `Transfer` event logged.
 *   - Errors: Fails if transfer fails or balances/events are incorrect.
 *
 * - `test_0021_lpt_transfer_operator_success`:
 *   - Purpose: Tests successful LP token transfer by an operator.
 *   - Setup: Uses `mint_and_transfer` with `ADDRESS_OPERATOR`.
 *   - Assertions: Transfer succeeds.
 *   - Errors: Fails if transfer fails.
 *
 * - `test_0022_lpt_transfer_wrong_sender`:
 *   - Purpose: Tests transfer failure with an unauthorized sender.
 *   - Setup: Uses `mint_and_transfer` with `ADDRESS_VILLAIN`.
 *   - Assertions: Fails with `Unauthorized` error.
 *   - Errors: Fails if error isn’t `Unauthorized`.
 *
 * - `test_0023_lpt_transfer_wrong_amount`:
 *   - Purpose: Tests transfer failure with insufficient funds.
 *   - Setup: Uses `mint_and_transfer` with amount exceeding minted tokens.
 *   - Assertions: Fails with `InsufficientFunds` error.
 *   - Errors: Fails if error isn’t `InsufficientFunds`.
 *
 * - `test_update_operator(operator_update_str: &str, is_operator: bool)`:
 *   - Purpose: Helper to test adding/removing operators.
 *   - Setup: Updates operator status, checks state and `operatorOf`.
 *   - Assertions: Operator status matches expectation, one `UpdateOperator` event logged.
 *
 * - `test_0030_lpt_update_operator_add_success`:
 *   - Purpose: Tests adding an operator.
 *   - Setup: Calls `test_update_operator` with "add".
 *   - Assertions: Operator is added, verified via state and `operatorOf`.
 *
 * - `test_0031_lpt_update_operator_remove_success`:
 *   - Purpose: Tests removing an operator.
 *   - Setup: Calls `test_update_operator` with "remove".
 *   - Assertions: Operator is removed, verified via state and `operatorOf`.
 *
 * - `test_0040_lpt_balance_of`:
 *   - Purpose: Tests querying LP token balances.
 *   - Setup: Mints 1B tokens to `ADDRESS_USER`, checks balances before/after burning.
 *   - Assertions: Balances are 1B for `ADDRESS_USER`, 0 for others, 0 after burn.
 *   - Errors: Fails if balances are incorrect.
 *
 * - `test_0050_lp_liquidity`:
 *   - Purpose: Tests adding and removing liquidity with specific amounts.
 *   - Setup: Adds liquidity twice (1B CCD/3T tokens, 2B CCD/6T tokens), removes 3B LP tokens.
 *   - Assertions: Verifies exchange states (CCD/token balances, LP supply), events (`Mint`, `Burn`).
 *   - Errors: Fails if operations fail or balances/events are incorrect.
 *
 * - `test_0051_lp_liquidity`:
 *   - Purpose: Tests adding liquidity with different amounts, includes debug prints.
 *   - Setup: Adds liquidity twice (144M CCD/174M tokens, 100M CCD/120M tokens), checks balances.
 *   - Assertions: Prints results for debugging, no strict assertions (incomplete test).
 *   - Notes: Appears incomplete; consider adding assertions or removing debug prints for production.
 *
 * NOTES FOR DEVELOPERS:
 * - Tests use mock CIS-2 behaviors to simulate token contract interactions; ensure mocks match real token contracts in production.
 * - `test_0051_lp_liquidity` is incomplete; add assertions to verify behavior.
 * - Extend tests for swap functions (`ccdToTokenSwap`, etc.) and error cases (e.g., non-CIS-2 tokens).
 * - Use `claim_eq!` and `expect_error` for precise assertions; avoid panics with `.unwrap()` where possible.
 */

#[concordium_cfg_test]
mod tests {
    use super::*;
    use concordium_std::test_infrastructure::*;
    use concordium_cis2::*;
    use core::fmt::Debug;
    use std::collections::HashMap;

    use crate::contract::*;
    use crate::types::*;
    use crate::state::*;
    use crate::params::*;
    use crate::errors::*;

    const ACCOUNT_DEPLOYER: AccountAddress = AccountAddress([1u8; 32]);
    // const ADDRESS_DEPLOYER: Address = Address::Account(ACCOUNT_DEPLOYER);

    const ACCOUNT_USER: AccountAddress = AccountAddress([2u8; 32]);
    const ADDRESS_USER: Address = Address::Account(ACCOUNT_USER);

    const ACCOUNT_ANOTHER_USER: AccountAddress = AccountAddress([3u8; 32]);
    const ADDRESS_ANOTHER_USER: Address = Address::Account(ACCOUNT_ANOTHER_USER);

    const ACCOUNT_OPERATOR: AccountAddress = AccountAddress([4u8; 32]);
    const ADDRESS_OPERATOR: Address = Address::Account(ACCOUNT_OPERATOR);

    const ACCOUNT_VILLAIN: AccountAddress = AccountAddress([5u8; 32]);
    const ADDRESS_VILLAIN: Address = Address::Account(ACCOUNT_VILLAIN);

    const SWAP_INDEX: u64 = 500;

    struct CallResult<T> {
        result: T,
        logger: TestLogger,
        host: TestHost<State<TestStateApi>>,
    }

    fn get_token_index(number: u64) -> u64 {1000 + number}

    fn get_token(number: u64) -> TokenInfo {
        TokenInfo {
            address: ContractAddress {
                index: get_token_index(number),
                subindex: 0
            },
            id: TokenIdVec(vec![00,]),
        }
    }

    fn get_ctx<'a>(
        address_sender: Address,
    ) -> TestReceiveContext<'a> {
        let mut ctx = TestReceiveContext::empty();
        ctx.set_self_address(ContractAddress::new(SWAP_INDEX, 0));
        ctx.set_owner(ACCOUNT_DEPLOYER);
        ctx.set_sender(address_sender);
        ctx
    }

    fn get_host() -> TestHost<State<TestStateApi>> {
        let mut state_builder = TestStateBuilder::new();
        let state = State::empty(&mut state_builder);
        TestHost::new(state, state_builder)
    }

    fn mock_cis2_supports(
        mut host: TestHost<State<TestStateApi>>,
        index: u64,
        result: bool,
    ) -> TestHost<State<TestStateApi>> {
        host.setup_mock_entrypoint(
            ContractAddress {index, subindex: 0,},
            OwnedEntrypointName::new_unchecked("supports".to_string()),
            MockFn::new_v1(move |_parameter, _amount, _balance, _state: &mut State<TestStateApi>| {
                let result = match result {
                    true => SupportResult::Support,
                    false => SupportResult::NoSupport,
                };
                Ok((false, SupportsQueryResponse{
                    results: vec![result,]
                }))
            }),
        );
        host
    }

    fn mock_cis2_operator_of(
        mut host: TestHost<State<TestStateApi>>,
        index: u64,
        result: bool,
    ) -> TestHost<State<TestStateApi>> {
        host.setup_mock_entrypoint(
            ContractAddress {index, subindex: 0,},
            OwnedEntrypointName::new_unchecked("operatorOf".to_string()),
            MockFn::new_v1(move |_parameter, _amount, _balance, _state: &mut State<TestStateApi>| {
                Ok((false, OperatorOfQueryResponse(
                    vec![result,]
                )))
            }),
        );
        host
    }

    fn mock_cis2_balance_of(
        mut host: TestHost<State<TestStateApi>>,
        index: u64,
        balances: HashMap<Address, u64>,
    ) -> TestHost<State<TestStateApi>> {
        host.setup_mock_entrypoint(
            ContractAddress {index, subindex: 0,},
            OwnedEntrypointName::new_unchecked("balanceOf".to_string()),
            MockFn::new_v1(move |parameter, _amount, _balance, _state: &mut State<TestStateApi>| {
                let mut cur = Cursor {offset: 0, data: parameter.as_ref()};
                let params= BalanceOfQueryParams::<TokenIdVec>::deserial(&mut cur).unwrap();
                let mut response = Vec::with_capacity(params.queries.len());
                for query in params.queries {
                    response.push(match balances.get(&query.address) {
                        Some(val) => TokenAmountU64::from(*val),
                        None => TokenAmountU64::from(0),
                    });
                }
                Ok((false, BalanceOfQueryResponse(response)))
            }),
        );
        host
    }

    fn mock_cis2_transfer(
        mut host: TestHost<State<TestStateApi>>,
        index: u64,
    ) -> TestHost<State<TestStateApi>> {
        host.setup_mock_entrypoint(
            ContractAddress {index, subindex: 0,},
            OwnedEntrypointName::new_unchecked("transfer".to_string()),
            MockFn::new_v1(move |_parameter, _amount, _balance, _state: &mut State<TestStateApi>| {
                Ok((false, ()))
            }),
        );
        host
    }

    fn expect_error<E, T>(expr: Result<T, E>, err: E, msg: &str)
    where
        E: Eq + Debug,
        T: Debug, {
        let actual = expr.expect_err_report(msg);
        claim_eq!(actual, err);
    }

    #[concordium_test]
    fn test_0010_init_success() {
        let ctx = TestInitContext::empty();

        let mut state_builder = TestStateBuilder::new();

        let state_result = init(&ctx, &mut state_builder);
        state_result.expect_report("Unexpected error in contract initialization results");
    }

    fn mint_and_transfer(
        sender: Address,
        amount: ContractTokenAmount,
    ) -> CallResult<ContractResult<()>> {
        let mut logger = TestLogger::init();
        let mut host = get_host();

        let mint_amount: ContractTokenAmount = ContractTokenAmount::from(1000_000_000);
        let (state, builder) = host.state_and_builder();
        state.mint(
            &ContractTokenId::from(0),
            mint_amount,
            &ADDRESS_USER,
            builder,
        );
        state.add_operator(
            &ADDRESS_USER,
            &ADDRESS_OPERATOR,
            builder,
        );


        let transfer = Transfer {
            token_id: ContractTokenId::from(0),
            amount,
            from:     ADDRESS_USER,
            to:       Receiver::from_account(ACCOUNT_ANOTHER_USER),
            data:     AdditionalData::empty(),
        };
        let parameter = to_bytes(&TransferParameter::from(vec![transfer]));
        let mut ctx = get_ctx(sender);
        ctx.set_parameter(&parameter);

        let result: ContractResult<()> = lpt_transfer(&ctx, &mut host, &mut logger);

        CallResult {result, logger, host}
    }

    #[concordium_test]
    fn test_0020_lpt_transfer_success() {
        let sender = ADDRESS_USER;
        let token_id = ContractTokenId::from(0);
        let amount = ContractTokenAmount::from(1000_000_000);
        let CallResult{result, logger, host} = mint_and_transfer(sender, amount);
        claim!(result.is_ok(), "Unexpected error in result");

        let balance0 =
            host.state().balance(&token_id, &ADDRESS_USER).expect_report("Token is expected to exist");
        let balance1 =
            host.state().balance(&token_id, &ADDRESS_ANOTHER_USER).expect_report("Token is expected to exist");
        claim_eq!(
            balance0,
            0.into(),
            "Token owner balance should be decreased by the transferred amount."
        );
        claim_eq!(
            balance1,
            1000_000_000.into(),
            "Token receiver balance should be increased by the transferred amount"
        );

        claim_eq!(logger.logs.len(), 1, "Only one event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(&Cis2Event::Transfer(TransferEvent {
                from:     ADDRESS_USER,
                to:       ADDRESS_ANOTHER_USER,
                token_id: token_id,
                amount:   ContractTokenAmount::from(1000_000_000),
            })),
            "Incorrect event emitted"
        )

    }

    #[concordium_test]
    fn test_0021_lpt_transfer_operator_success() {
        let sender = ADDRESS_OPERATOR;
        let amount = ContractTokenAmount::from(1000_000_000);
        let CallResult{result, logger: _, host: _} = mint_and_transfer(sender, amount);
        claim!(result.is_ok(), "Unexpected error in result");
    }

    #[concordium_test]
    fn test_0022_lpt_transfer_wrong_sender() {
        let sender = ADDRESS_VILLAIN;
        let amount = ContractTokenAmount::from(1000_000_000);
        let CallResult{result, logger: _, host: _} = mint_and_transfer(sender, amount);
        expect_error(
            result,
            ContractError::Unauthorized,
            "The call was expected to return an error",
        );
    }

    #[concordium_test]
    fn test_0023_lpt_transfer_wrong_amount() {
        let sender = ADDRESS_USER;
        let amount = ContractTokenAmount::from(1000_000_001);
        let CallResult{result, logger: _, host: _} = mint_and_transfer(sender, amount);
        expect_error(
            result,
            ContractError::InsufficientFunds,
            "The call was expected to return an error",
        );
    }

    fn get_operator_update(s: &str) -> OperatorUpdate {
        match s {
            "add" => OperatorUpdate::Add,
            _ => OperatorUpdate::Remove,
        }
    }

    fn test_update_operator(
        operator_update_str: &str,
        is_operator: bool
    ) {
        let mut logger = TestLogger::init();
        let mut host = get_host();

        let update = UpdateOperator {
            operator: ADDRESS_OPERATOR,
            update:   get_operator_update(operator_update_str),
        };
        let parameter = to_bytes(&UpdateOperatorParams(vec![update]));
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result: ContractResult<()> = lpt_update_operator(&ctx, &mut host, &mut logger);
        claim!(result.is_ok(), "Results in rejection");

        let state_is_operator = host.state().is_operator(&ADDRESS_OPERATOR, &ADDRESS_USER);
        claim!(state_is_operator == is_operator, "Wrong is_operator result");

        let operator_of_query = OperatorOfQuery {
            address: ADDRESS_OPERATOR,
            owner:   ADDRESS_USER,
        };
        let parameter = to_bytes(&OperatorOfQueryParams {queries: vec![operator_of_query]});
        ctx.set_parameter(&parameter);

        let result: ContractResult<OperatorOfQueryResponse> = lpt_operator_of(&ctx, &host);

        claim_eq!(
            result.expect_report("Failed getting result value").0,
            [is_operator],
            "Wrong lpt_operator_of result"
        );

        claim_eq!(logger.logs.len(), 1, "One event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(&Cis2Event::<ContractTokenId, ContractTokenAmount>::UpdateOperator(
                UpdateOperatorEvent {
                    owner:    ADDRESS_USER,
                    operator: ADDRESS_OPERATOR,
                    update:   get_operator_update(operator_update_str),
                }
            )),
            "Incorrect event emitted"
        )
    }

    #[concordium_test]
    fn test_0030_lpt_update_operator_add_success() {
        test_update_operator("add", true)
    }

    #[concordium_test]
    fn test_0031_lpt_update_operator_remove_success() {
        test_update_operator("remove", false)
    }

    #[concordium_test]
    fn test_0040_lpt_balance_of() {
        let mut host = get_host();

        let mint_amount: ContractTokenAmount = ContractTokenAmount::from(1000_000_000);
        let token_id = ContractTokenId::from(0);

        let (state, builder) = host.state_and_builder();
        state.mint(
            &token_id,
            mint_amount,
            &ADDRESS_USER,
            builder,
        );

        let parameter = to_bytes(&BalanceOfQueryParams {
            queries: vec![BalanceOfQuery {
                token_id,
                address: ADDRESS_USER,
            }],
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = lpt_balance_of(&ctx, &host);
        claim_eq!(
            result.expect_report("Failed getting result value").0,
            [mint_amount],
            "Wrong lpt_balance_of result"
        );

        let parameter = to_bytes(&BalanceOfQueryParams {
            queries: vec![BalanceOfQuery {
                token_id,
                address: ADDRESS_ANOTHER_USER,
            }],
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = lpt_balance_of(&ctx, &host);
        claim_eq!(
            result.expect_report("Failed getting result value").0,
            [0.into()],
            "Wrong lpt_balance_of result"
        );

        let (state, builder) = host.state_and_builder();
        state.burn(
            &token_id,
            mint_amount,
            &ADDRESS_USER,
            builder,
        ).unwrap();

        let parameter = to_bytes(&BalanceOfQueryParams {
            queries: vec![BalanceOfQuery {
                token_id,
                address: ADDRESS_USER,
            }],
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = lpt_balance_of(&ctx, &host);
        claim_eq!(
            result.expect_report("Failed getting result value").0,
            [0.into()],
            "Wrong lpt_balance_of result"
        );
    }

    #[concordium_test]
    fn test_0050_lp_liquidity() {
        let mut logger = TestLogger::init();
        let mut host = get_host();

        let token_1 = get_token(1);
        let mint_first_ccd_amount = Amount::from_micro_ccd(1_000_000_000);
        let mint_first_token_amount = ContractTokenAmount::from(3_000_000_000_000);
        let mint_second_ccd_amount = Amount::from_micro_ccd(2_000_000_000);
        let mint_second_token_amount = ContractTokenAmount::from(6_000_000_000_000);
        let remove_lp_token_amount = ContractTokenAmount::from(3_000_000_000);


        // Checking if the list of liquidity pools is empty
        let parameter = to_bytes(&GetExchangesParams {
            holder: ADDRESS_USER,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = get_exchanges(&ctx, &mut host);
        claim_eq!(
            result.expect_report("Failed getting result value").exchanges.len(),
            0,
            "Wrong get_exchanges result"
        );


        // First filling of the liquidity pool
        host = mock_cis2_supports(host, get_token_index(1), true);
        host = mock_cis2_operator_of(host, get_token_index(1), true);
        host = mock_cis2_transfer(host, get_token_index(1));
        let parameter = to_bytes(&AddLiquidityParams {
            token: token_1.clone(),
            token_amount: mint_first_token_amount,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);
        let result = lp_add_liquidity(&ctx, &mut host, mint_first_ccd_amount, &mut logger);
        claim!(result.is_ok(), "Results in rejection");

        claim!(
            logger.logs.contains(&to_bytes(&Cis2Event::Mint(MintEvent {
                owner:    ADDRESS_USER,
                token_id: ContractTokenId::from(1),
                amount:   ContractTokenAmount::from(mint_first_ccd_amount.micro_ccd),
            }))),
            "Expected an event for minting"
        );

        claim!(
            logger.logs.contains(&to_bytes(&Cis2Event::TokenMetadata::<_, ContractTokenAmount>(
                TokenMetadataEvent {
                    token_id:     ContractTokenId::from(1),
                    metadata_url: MetadataUrl {
                        url:  "https://concordium-servernode.dev-site.space/api/v1/metadata/swap/lp-tokens?contract_index=1001&token_id=00".to_string(),
                        hash: None,
                    },
                }
            ))),
            "Expected an event for token metadata"
        );

        host = mock_cis2_balance_of(
            host,
            get_token_index(1),
            HashMap::from([
                (Address::Contract(ContractAddress {index: SWAP_INDEX, subindex: 0}), mint_first_token_amount.0),
            ])
        );
        let parameter = to_bytes(&GetExchangeParams {
            token: token_1.clone(),
            holder: ADDRESS_USER,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = get_exchange(&ctx, &mut host);
        claim!(result.is_ok(), "Results in rejection");

        let exchange_info = result.expect_report("Failed getting result value");
        claim_eq!(exchange_info.ccd_balance.0, mint_first_ccd_amount.micro_ccd, "Wrong ccd_balance in get_exchange result");
        claim_eq!(exchange_info.token_balance, mint_first_token_amount, "Wrong token_balance in get_exchange result");
        claim_eq!(exchange_info.lp_tokens_holder_balance.0, mint_first_ccd_amount.micro_ccd, "Wrong lp_tokens_holder_balance in get_exchange result");


        // Checking for the wrong ratio
        let parameter = to_bytes(&AddLiquidityParams {
            token: token_1.clone(),
            token_amount: mint_second_token_amount - 1.into(),
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);
        let result = lp_add_liquidity(&ctx, &mut host, mint_second_ccd_amount, &mut logger);
        expect_error(
            result,
            ContractError::IncorrectTokenCcdRatio,
            "The call was expected to return an error",
        );


        // Second filling of the liquidity pool
        host = mock_cis2_balance_of(
            host,
            get_token_index(1),
            HashMap::from([
                (Address::Contract(ContractAddress {index: SWAP_INDEX, subindex: 0}), mint_first_token_amount.0),
            ])
        );
        let parameter = to_bytes(&AddLiquidityParams {
            token: token_1.clone(),
            token_amount: mint_second_token_amount,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);
        let result = lp_add_liquidity(&ctx, &mut host, mint_second_ccd_amount, &mut logger);
        claim!(result.is_ok(), "Results in rejection");

        host = mock_cis2_balance_of(
            host,
            get_token_index(1),
            HashMap::from([
                (
                    Address::Contract(ContractAddress {index: SWAP_INDEX, subindex: 0}),
                    mint_first_token_amount.0 + mint_second_token_amount.0
                ),
            ])
        );
        let parameter = to_bytes(&GetExchangeParams {
            token: token_1.clone(),
            holder: ADDRESS_USER,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = get_exchange(&ctx, &mut host);
        claim!(result.is_ok(), "Results in rejection");

        let exchange_info = result.expect_report("Failed getting result value");
        claim_eq!(
            exchange_info.ccd_balance.0,
            mint_first_ccd_amount.micro_ccd + mint_second_ccd_amount.micro_ccd,
            "Wrong ccd_balance in get_exchange result"
        );
        claim_eq!(
            exchange_info.token_balance,
            mint_first_token_amount + mint_second_token_amount,
            "Wrong token_balance in get_exchange result"
        );
        claim_eq!(
            exchange_info.lp_tokens_holder_balance.0,
            mint_first_ccd_amount.micro_ccd + mint_second_ccd_amount.micro_ccd,
            "Wrong lp_tokens_holder_balance in get_exchange result"
        );

        // Liquidity withdrawal
        host = mock_cis2_balance_of(
            host,
            get_token_index(1),
            HashMap::from([
                (
                    Address::Contract(ContractAddress {index: SWAP_INDEX, subindex: 0}),
                    mint_first_token_amount.0 + mint_second_token_amount.0
                ),
            ])
        );
        host.set_self_balance(Amount::from_micro_ccd(remove_lp_token_amount.0));
        let parameter = to_bytes(&RemoveLiquidityParams {
            token: token_1.clone(),
            lp_token_amount: remove_lp_token_amount,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);
        let result = lp_remove_liquidity(&ctx, &mut host, &mut logger);
        claim!(result.is_ok(), "Results in rejection");

        claim!(
            logger.logs.contains(&to_bytes(&Cis2Event::Burn(BurnEvent {
                owner:    ADDRESS_USER,
                token_id: ContractTokenId::from(1),
                amount:   ContractTokenAmount::from(remove_lp_token_amount.0),
            }))),
            "Expected an event for burning"
        );

        host = mock_cis2_balance_of(
            host,
            get_token_index(1),
            HashMap::from([
                (Address::Contract(ContractAddress {index: SWAP_INDEX, subindex: 0}), 0),
            ])
        );
        let parameter = to_bytes(&GetExchangeParams {
            token: token_1.clone(),
            holder: ADDRESS_USER,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = get_exchange(&ctx, &mut host);
        claim!(result.is_ok(), "Results in rejection");

        let exchange_info = result.expect_report("Failed getting result value");
        claim_eq!(exchange_info.ccd_balance.0, 0u64, "Wrong ccd_balance in get_exchange result");
        claim_eq!(exchange_info.token_balance, 0u64.into(), "Wrong token_balance in get_exchange result");
        claim_eq!(exchange_info.lp_tokens_holder_balance.0, 0u64, "Wrong lp_tokens_holder_balance in get_exchange result");
    }

    #[concordium_test]
    fn test_0051_lp_liquidity() {
        let mut logger = TestLogger::init();
        let mut host = get_host();

        let token_1 = get_token(1);
        let mint_first_ccd_amount = Amount::from_micro_ccd(144688756);
        let mint_first_token_amount = ContractTokenAmount::from(174576853);
        let mint_second_ccd_amount = Amount::from_micro_ccd(100_000_000);
        let mint_second_token_amount = ContractTokenAmount::from(120_657_000);


        // First filling of the liquidity pool
        host = mock_cis2_supports(host, get_token_index(1), true);
        host = mock_cis2_operator_of(host, get_token_index(1), true);
        host = mock_cis2_transfer(host, get_token_index(1));
        let parameter = to_bytes(&AddLiquidityParams {
            token: token_1.clone(),
            token_amount: mint_first_token_amount,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);
        let result = lp_add_liquidity(&ctx, &mut host, mint_first_ccd_amount, &mut logger);

        println!("-------------------");
        println!("{:?}", result);

        // Check balance

        host = mock_cis2_balance_of(
            host,
            get_token_index(1),
            HashMap::from([
                (Address::Contract(ContractAddress {index: SWAP_INDEX, subindex: 0}), mint_first_token_amount.0),
            ])
        );
        let parameter = to_bytes(&GetExchangeParams {
            token: token_1.clone(),
            holder: ADDRESS_USER,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = get_exchange(&ctx, &mut host);

        println!("-------------------");
        println!("{:?}", result);

        // Second filling of the liquidity pool
        host = mock_cis2_balance_of(
            host,
            get_token_index(1),
            HashMap::from([
                (Address::Contract(ContractAddress {index: SWAP_INDEX, subindex: 0}), mint_first_token_amount.0),
            ])
        );
        let parameter = to_bytes(&AddLiquidityParams {
            token: token_1.clone(),
            token_amount: mint_second_token_amount,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);
        let result = lp_add_liquidity(&ctx, &mut host, mint_second_ccd_amount, &mut logger);

        println!("-------------------");
        println!("{:?}", result);

        // Check balance

        let parameter = to_bytes(&GetExchangeParams {
            token: token_1.clone(),
            holder: ADDRESS_USER,
        });
        let mut ctx = get_ctx(ADDRESS_USER);
        ctx.set_parameter(&parameter);

        let result = get_exchange(&ctx, &mut host);

        println!("-------------------");
        println!("{:?}", result);

    }


}