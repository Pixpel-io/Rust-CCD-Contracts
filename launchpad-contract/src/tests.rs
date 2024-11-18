// use concordium_std::*;

// #[concordium_cfg_test]
// mod tests {
//     use super::*;
//     use concordium_cis2::*;
//     use concordium_std::test_infrastructure::*;
//     use core::fmt::Debug;
//     use std::collections::HashMap;
//     use std::process::id;

//     use crate::contract::*;
//     use crate::errors::*;
//     use crate::params::*;
//     use crate::state::*;
//     use crate::types::*;

//     const ACCOUNT_DEPLOYER: AccountAddress = AccountAddress([1u8; 32]);
//     // const ADDRESS_DEPLOYER: Address = Address::Account(ACCOUNT_DEPLOYER);

//     const ACCOUNT_USER: AccountAddress = AccountAddress([2u8; 32]);
//     const ADDRESS_USER: Address = Address::Account(ACCOUNT_USER);

//     const ACCOUNT_ANOTHER_USER: AccountAddress = AccountAddress([3u8; 32]);
//     const ADDRESS_ANOTHER_USER: Address = Address::Account(ACCOUNT_ANOTHER_USER);

//     const ACCOUNT_OPERATOR: AccountAddress = AccountAddress([4u8; 32]);
//     const ADDRESS_OPERATOR: Address = Address::Account(ACCOUNT_OPERATOR);

//     const ACCOUNT_VILLAIN: AccountAddress = AccountAddress([5u8; 32]);
//     const ADDRESS_VILLAIN: Address = Address::Account(ACCOUNT_VILLAIN);

//     const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
//     const ADDRESS_0: Address = Address::Account(ACCOUNT_0);
//     const TOKEN_CONTRACT_ADDRESS: ContractAddress = ContractAddress {
//         index: 1,
//         subindex: 0,
//     };
//     const MARKET_CONTRACT_ADDRESS: ContractAddress = ContractAddress {
//         index: 2,
//         subindex: 0,
//     };

//     const SWAP_INDEX: u64 = 500;

//     struct CallResult<T> {
//         result: T,
//         logger: TestLogger,
//         host: TestHost<State>,
//     }

//     fn get_token_index(number: u64) -> u64 {
//         1000 + number
//     }

//     fn get_token(number: u64) -> TokenInfo {
//         TokenInfo {
//             address: ContractAddress {
//                 index: get_token_index(number),
//                 subindex: 0,
//             },
//             id: TokenIdVec(vec![00]),
//         }
//     }

//     fn get_ctx<'a>(address_sender: Address) -> TestReceiveContext<'a> {
//         let mut ctx = TestReceiveContext::empty();
//         ctx.set_self_address(ContractAddress::new(SWAP_INDEX, 0));
//         ctx.set_owner(ACCOUNT_DEPLOYER);
//         ctx.set_sender(address_sender);
//         ctx
//     }

//     fn mock_cis2_transfer(mut host: TestHost<State>, index: u64) -> TestHost<State> {
//         host.setup_mock_entrypoint(
//             ContractAddress { index, subindex: 0 },
//             OwnedEntrypointName::new_unchecked("transfer".to_string()),
//             MockFn::new_v1(move |_parameter, _amount, _balance, _state: &mut State| {
//                 Ok((false, ()))
//             }),
//         );
//         host
//     }

//     fn mock_cis2_balance_of(
//         mut host: TestHost<State>,
//         index: u64,
//         balances: HashMap<Address, u64>,
//     ) -> TestHost<State> {
//         host.setup_mock_entrypoint(
//             ContractAddress { index, subindex: 0 },
//             OwnedEntrypointName::new_unchecked("balanceOf".to_string()),
//             MockFn::new_v1(move |parameter, _amount, _balance, _state: &mut State| {
//                 let mut cur = Cursor {
//                     offset: 0,
//                     data: parameter.as_ref(),
//                 };
//                 let params = BalanceOfQueryParams::<TokenIdVec>::deserial(&mut cur).unwrap();
//                 let mut response = Vec::with_capacity(params.queries.len());
//                 for query in params.queries {
//                     response.push(match balances.get(&query.address) {
//                         Some(val) => TokenAmountU64::from(*val),
//                         None => TokenAmountU64::from(0),
//                     });
//                 }
//                 Ok((false, BalanceOfQueryResponse(response)))
//             }),
//         );
//         host
//     }

//     #[concordium_test]
//     fn test_0050_lp_liquidity() {
//         let mut state_builder = TestStateBuilder::new();
//         // let state = State::new(&mut state_builder);
//         // let mut host = TestHost::new(state, state_builder);

//         let mut logger = TestLogger::init();

//         let token_1 = get_token(1);
//         let mint_first_ccd_amount = Amount::from_micro_ccd(1_000_000_000);
//         let mint_first_token_amount = ContractTokenAmount::from(3_000_000_000_000);
//         let mint_second_ccd_amount = Amount::from_micro_ccd(2_000_000_000);
//         let mint_second_token_amount = ContractTokenAmount::from(6_000_000_000_000);
//         let remove_lp_token_amount = ContractTokenAmount::from(3_000_000_000);
//         let token_id_1 = ContractTokenId::from(1);

//         // Checking if the list of liquidity pools is empty

//         // First filling of the liquidity pool
//         let add_params = TokenParam {
//             token_amount: TokenAmountU64(10),
//             id: token_id_1,
//             address: TOKEN_CONTRACT_ADDRESS,
//         };
//         let parameter_bytes = to_bytes(&add_params);
//         host = mock_cis2_transfer(host, get_token_index(1));
//         let mut ctx = get_ctx(ADDRESS_USER);
//         ctx.set_parameter(&parameter_bytes);

//         // claim!(
//         //     logger.logs.contains(&to_bytes(&Cis2Event::Mint(MintEvent {
//         //         owner: ADDRESS_USER,
//         //         token_id: ContractTokenId::from(1),
//         //         amount: ContractTokenAmount::from(mint_first_ccd_amount.micro_ccd),
//         //     }))),
//         //     "Expected an event for minting"
//         // );

//         host = mock_cis2_balance_of(
//             host,
//             get_token_index(1),
//             HashMap::from([(
//                 Address::Contract(ContractAddress {
//                     index: SWAP_INDEX,
//                     subindex: 0,
//                 }),
//                 mint_first_token_amount.0,
//             )]),
//         );

//         let mut ctx = get_ctx(ADDRESS_USER);
//         ctx.set_parameter(&parameter_bytes);

//         // Checking for the wrong ratio

//         // Second filling of the liquidity pool
//         host = mock_cis2_balance_of(
//             host,
//             get_token_index(1),
//             HashMap::from([(
//                 Address::Contract(ContractAddress {
//                     index: SWAP_INDEX,
//                     subindex: 0,
//                 }),
//                 mint_first_token_amount.0,
//             )]),
//         );
//     }
// }
