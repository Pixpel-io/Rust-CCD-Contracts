use concordium_std::*;
use concordium_cis2::*;

use core::fmt::Debug;
use std::ops::Deref;

use crate::errors::*;
use crate::types::*;
use crate::responses::*;


#[derive(Serial, Deserial, SchemaType, Clone, Debug)]
pub struct ExchangeState {
    pub lp_token_id: ContractTokenId,
    pub ccd_balance: ContractTokenAmount,
}


#[derive(Serial, DeserialWithState, Deletable, StateClone, Debug)]
#[concordium(state_parameter = "S")]
pub struct AddressState<S> {
    pub balances: StateMap<ContractTokenId, ContractTokenAmount, S>,
    pub operators: StateSet<Address, S>,
}


impl<S: HasStateApi> AddressState<S> {
    fn empty(state_builder: &mut StateBuilder<S>) -> Self {
        AddressState {
            balances: state_builder.new_map(),
            operators: state_builder.new_set(),
        }
    }
}


#[derive(StateClone, Serial, DeserialWithState, Debug)]
#[concordium(state_parameter = "S")]
pub struct State<S> {
    pub exchanges: StateMap<TokenInfo, ExchangeState, S>,
    pub lp_tokens_state: StateMap<Address, AddressState<S>, S>,
    pub lp_tokens_supply: StateMap<ContractTokenId, ContractTokenAmount, S>,
    pub last_lp_token_id: ContractTokenId,
}


impl<S: HasStateApi> State<S> {
    pub fn empty(state_builder: &mut StateBuilder<S>) -> Self {
        State {
            exchanges: state_builder.new_map(),
            lp_tokens_state: state_builder.new_map(),
            lp_tokens_supply: state_builder.new_map(),
            last_lp_token_id: TokenIdU64(0),
        }
    }

    // Exchanges

    pub fn create_exchange(
        &mut self,
        token_info: &TokenInfo,
    ) -> Result<(), ContractError> {
        let lp_token_id = TokenIdU64(self.last_lp_token_id.0 + 1);
        self.exchanges.insert(
            token_info.clone(),
            ExchangeState {
                lp_token_id,
                ccd_balance: TokenAmountU64(0),
            }
        );
        self.lp_tokens_supply.insert(lp_token_id, 0.into());
        self.last_lp_token_id = lp_token_id;
        Ok(())
    }

    pub fn get_exchange_view(
        &self,
        token_info: &TokenInfo,
        holder: &Address,
    ) -> ContractResult<ExchangeView> {
        let exchange_state =
            self.exchanges.get(token_info).map(|v| v.deref().clone()).ok_or(ContractError::ExchangeNotFound)?;
        let lp_token_id = exchange_state.lp_token_id;
        let lp_tokens_supply =
            self.lp_tokens_supply.get(&lp_token_id).map(|v| *v.deref()).ok_or(ContractError::ExchangeNotFound)?;
        let lp_tokens_holder_balance =
            self.balance(&lp_token_id, holder)?;

        Ok(ExchangeView {
            token: token_info.clone(),
            token_balance: 0.into(),
            ccd_balance: exchange_state.ccd_balance,
            lp_token_id,
            lp_tokens_supply,
            lp_tokens_holder_balance
        })
    }

    // LP tokens

    pub fn mint(
        &mut self,
        token_id: &ContractTokenId,
        amount: ContractTokenAmount,
        owner: &Address,
        state_builder: &mut StateBuilder<S>,
    ) {
        let mut owner_state =
            self.lp_tokens_state.entry(*owner).or_insert_with(|| AddressState::empty(state_builder));
        let mut owner_balance = owner_state.balances.entry(*token_id).or_insert(0.into());
        *owner_balance += amount;
        let mut token_supply =
            self.lp_tokens_supply.entry(*token_id).or_insert(0.into());
        *token_supply += amount;
    }

    pub fn burn(
        &mut self,
        token_id: &ContractTokenId,
        amount: ContractTokenAmount,
        owner: &Address,
        state_builder: &mut StateBuilder<S>,
    ) -> ContractResult<()> {
        let mut owner_state =
            self.lp_tokens_state.entry(*owner).or_insert_with(|| AddressState::empty(state_builder));
        let mut owner_balance = owner_state.balances.entry(*token_id).or_insert(0.into());
        ensure!(*owner_balance >= amount, ContractError::InsufficientFunds);
        *owner_balance -= amount;
        let mut token_supply =
            self.lp_tokens_supply.entry(*token_id).or_insert(0.into());
        *token_supply -= amount;

        Ok(())
    }

    pub fn transfer(
        &mut self,
        token_id: &ContractTokenId,
        amount: ContractTokenAmount,
        from: &Address,
        to: &Address,
        state_builder: &mut StateBuilder<S>,
    ) -> ContractResult<()> {
        ensure!(self.contains_token(token_id), ContractError::InvalidTokenId);
        if amount == 0.into() {
            return Ok(());
        }

        {
            let mut from_address_state =
                self.lp_tokens_state.entry(*from).occupied_or(ContractError::InsufficientFunds)?;
            let mut from_balance = from_address_state
                .balances
                .entry(*token_id)
                .occupied_or(ContractError::InsufficientFunds)?;
            ensure!(*from_balance >= amount, ContractError::InsufficientFunds);
            *from_balance -= amount;
        }

        let mut to_address_state =
            self.lp_tokens_state.entry(*to).or_insert_with(|| AddressState::empty(state_builder));
        let mut to_address_balance = to_address_state.balances.entry(*token_id).or_insert(0.into());
        *to_address_balance += amount;

        Ok(())
    }

    #[inline(always)]
    pub fn contains_token(&self, token_id: &ContractTokenId) -> bool {
        self.lp_tokens_supply.get(token_id).is_some()
    }

    pub fn is_operator(&self, address: &Address, owner: &Address) -> bool {
        self.lp_tokens_state
            .get(owner)
            .map(|address_state| address_state.operators.contains(address))
            .unwrap_or(false)
    }

    pub fn add_operator(
        &mut self,
        owner: &Address,
        operator: &Address,
        state_builder: &mut StateBuilder<S>,
    ) {
        let mut owner_state =
            self.lp_tokens_state.entry(*owner).or_insert_with(|| AddressState::empty(state_builder));
        owner_state.operators.insert(*operator);
    }

    pub fn remove_operator(&mut self, owner: &Address, operator: &Address) {
        self.lp_tokens_state.entry(*owner).and_modify(|address_state| {
            address_state.operators.remove(operator);
        });
    }

    pub fn balance(
        &self,
        token_id: &ContractTokenId,
        address: &Address,
    ) -> ContractResult<ContractTokenAmount> {
        ensure!(self.contains_token(token_id), ContractError::InvalidTokenId);
        let balance = self.lp_tokens_state.get(address).map_or(0.into(), |address_state| {
            address_state.balances.get(token_id).map_or(0.into(), |x| *x)
        });
        Ok(balance)
    }

    // Liquidity pools

    pub fn get_exchange_ccd_balance(
        &self,
        token_info: &TokenInfo,
    ) -> ContractResult<ContractTokenAmount> {
        let exchange_state =
            self.exchanges.get(token_info).map(|v| v.deref().clone()).ok_or(ContractError::ExchangeNotFound)?;
        Ok(exchange_state.ccd_balance)
    }

    pub fn increase_exchange_ccd_balance(
        &mut self,
        token_info: &TokenInfo,
        ccd_balance: ContractTokenAmount,
    ) -> Result<(), ContractError> {
        let mut exchange_state =
            self.exchanges.entry(token_info.clone()).or_insert_with(|| ExchangeState {
                lp_token_id: TokenIdU64::from(0),
                ccd_balance: TokenAmountU64(0),
            });
        exchange_state.ccd_balance += ccd_balance;
        Ok(())
    }

    pub fn decrease_exchange_ccd_balance(
        &mut self,
        token_info: &TokenInfo,
        ccd_balance: ContractTokenAmount,
    ) -> Result<(), ContractError> {
        let mut exchange_state =
            self.exchanges.entry(token_info.clone()).or_insert_with(|| ExchangeState {
                lp_token_id: TokenIdU64::from(0),
                ccd_balance: TokenAmountU64(0),
            });
        exchange_state.ccd_balance -= ccd_balance;
        Ok(())
    }

    // Metadata helpers

    pub fn get_token_info_by_lp_token_id(
        &self,
        lp_token_id: &ContractTokenId
    ) -> ContractResult<TokenInfo> {
        for (token_info, ex_state) in self.exchanges.iter() {
            if &ex_state.lp_token_id == lp_token_id {
                return Ok(token_info.clone());
            }
        }
        Err(ContractError::ExchangeNotFound)
    }
}