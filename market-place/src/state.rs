//! Defines the State (persisted data) for the contract.

#![cfg_attr(not(feature = "std"), no_std)]

use concordium_cis2::{Cis2Client, IsTokenId};
use concordium_std::{
    self, AccountAddress, Amount, ContractAddress, Deserial, DeserialWithState, Entry, SchemaType,
    Serial, Serialize, StateApi, StateBuilder, StateMap, StateSet,
};

use crate::{
    errors::Error, params::{self, InitParams, ListParams}, ContractTokenAmount, ContractTokenId
};

pub type TokenList<K = TokenIdentifier, V = TokenDetails, S = StateApi> = StateMap<K, V, S>;

#[derive(Clone, Serialize, PartialEq, Eq, Debug)]
pub struct TokenInfo<T = ContractTokenId> {
    pub id: T,
    pub address: ContractAddress,
}

#[derive(Clone, Serialize, PartialEq, Eq, Debug)]
pub struct TokenOwnerInfo<T = ContractTokenId> {
    pub id: T,
    pub address: ContractAddress,
    pub owner: AccountAddress,
}

impl<T: IsTokenId> TokenOwnerInfo<T> {
    pub fn from(token_info: TokenInfo<T>, owner: &AccountAddress) -> Self {
        TokenOwnerInfo {
            owner: *owner,
            id: token_info.id,
            address: token_info.address,
        }
    }
}

#[derive(Clone, Serialize, Copy, PartialEq, Eq, Debug)]
pub struct TokenPriceState<A = ContractTokenAmount> {
    pub quantity: A,
    pub price: Amount,
}

#[derive(Clone, Serialize, Copy, PartialEq, Eq, Debug)]
pub struct TokenRoyaltyState {
    /// Primary Owner (Account Address which added the token first time on a
    /// Marketplace Instance)
    pub primary_owner: AccountAddress,

    /// Royalty basis points. Royalty percentage * 100.
    /// This can me atmost equal to 100*100 = 10000(MAX_BASIS_POINTS)
    pub royalty: u16,
}

/// Marketplace Commission
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Commission(pub u16);

impl Commission {
    /// Commission basis points. equals to percent * 100
    #[inline(always)]
    pub fn percentage_basis(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Serialize, SchemaType, PartialEq, Eq, Clone)]
pub struct TokenListItem<T = ContractTokenId, A = ContractTokenAmount> {
    pub token_id: T,
    pub contract: ContractAddress,
    pub price: Amount,
    pub owner: AccountAddress,
    pub royalty: u16,
    pub primary_owner: AccountAddress,
    pub quantity: A,
}

#[derive(Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<T = ContractTokenId, A = ContractTokenAmount, S = StateApi> {
    pub commission: Commission,
    pub admin: AccountAddress,
    pub pixp_client: PixPToken,
    pub token_list: StateMap<TokenIdentifier, TokenDetails, S>,
    pub token_royalties: StateMap<TokenInfo<T>, TokenRoyaltyState, S>,
    pub token_prices: StateMap<TokenOwnerInfo<T>, TokenPriceState<A>, S>,
}

impl State {
    /// Creates a new state with the given commission.
    /// The commission is given as a percentage basis, i.e. 10000 is 100%.
    pub fn new(state_builder: &mut StateBuilder, params: InitParams) -> Self {
        State {
            commission: Commission(params.commission),
            pixp_client: PixPToken(params.pixp_id, params.pixp_address),
            admin: params.admin,
            token_list: state_builder.new_map(),
            token_royalties: state_builder.new_map(),
            token_prices: state_builder.new_map(),
        }
    }

    #[inline]
    #[must_use]
    pub fn add_token(
        &mut self,
        owner: &AccountAddress,
        token_params: ListParams,
    ) -> bool {
        match self.token_list.entry(TokenIdentifier {
            id: token_params.id,
            cis2_address: token_params.cis2_address,
        }) {
            Entry::Occupied(_) => return false,
            Entry::Vacant(entry) => {
                let _ = entry.insert(TokenDetails {
                    price: token_params.price,
                    quantity: token_params.quantity.into(),
                    owner: *owner,
                });

                return true;
            }
        }
    }

    /// Adds a token to Buyable Token List.
    #[allow(
        unused_must_use,
        reason = "Its calculated, we dont care about state-map return type"
    )]
    pub fn list_token(
        &mut self,
        token_info: &TokenInfo,
        owner: &AccountAddress,
        price: Amount,
        royalty: u16,
        quantity: ContractTokenAmount,
    ) {
        match self.token_royalties.get(token_info) {
            // If the token is already listed, do nothing.
            Some(_) => None,
            // If the token is not listed, add it to the list.
            None => self.token_royalties.insert(
                token_info.clone(),
                TokenRoyaltyState {
                    primary_owner: *owner,
                    royalty,
                },
            ),
        };

        // Add the token to the buyable token list.
        // If the token is already listed, update the price.
        self.token_prices.insert(
            TokenOwnerInfo::from(token_info.clone(), owner),
            TokenPriceState { price, quantity },
        );
    }

    pub(crate) fn decrease_listed_quantity(
        &mut self,
        token_info: &TokenOwnerInfo,
        delta: ContractTokenAmount,
    ) {
        if let Some(mut price) = self.token_prices.get_mut(token_info) {
            price.quantity = price.quantity - delta;
        }
    }

    pub fn get_token(&self, id: ContractTokenId, cis2_address: ContractAddress) -> Result<TokenDetails, Error> {
        if let Some(details) = self.token_list.get(&TokenIdentifier { id, cis2_address }) {
            return Ok(*details);
        }

        return Err(Error::NotFound);
    }

    /// Gets a token from the buyable token list.
    pub fn get_listed(
        &self,
        token_info: &TokenInfo,
        owner: &AccountAddress,
    ) -> Option<(TokenRoyaltyState, TokenPriceState)> {
        match self.token_royalties.get(token_info) {
            Some(r) => self
                .token_prices
                .get(&TokenOwnerInfo::from(token_info.clone(), owner))
                .map(|p| (*r, *p)),
            None => Option::None,
        }
    }

    /// Gets a list of all tokens in the buyable token list.
    pub fn list(&self) -> Vec<TokenListItem> {
        self.token_prices
            .iter()
            .filter_map(|p| -> Option<TokenListItem> {
                let token_info = TokenInfo {
                    id: p.0.id,
                    address: p.0.address,
                };

                match self.token_royalties.get(&token_info) {
                    Option::None => Option::None,
                    Option::Some(r) => Option::Some(TokenListItem {
                        token_id: token_info.id,
                        contract: token_info.address,
                        price: p.1.price,
                        owner: p.0.owner,
                        royalty: r.royalty,
                        primary_owner: r.primary_owner,
                        quantity: p.1.quantity,
                    }),
                }
            })
            .collect()
    }
}

#[derive(Serial, Deserial, SchemaType, Clone, Copy)]
pub struct PixPToken(pub ContractTokenId, pub ContractAddress);

#[derive(Serial, Deserial, SchemaType)]
pub struct TokenIdentifier {
    pub id: ContractTokenId,
    pub cis2_address: ContractAddress,
}

#[derive(Serial, Deserial, SchemaType, Clone, Copy)]
pub struct TokenDetails {
    pub price: Price,
    pub quantity: u64,
    pub owner: AccountAddress,
}

#[derive(Serial, Deserial, SchemaType, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Price {
    CCD(u64),
    PIXP(u64),
}

impl Price {
    #[inline(always)]
    pub fn value(&self) -> u64 {
        match self {
            Price::PIXP(amount) => *amount,
            Price::CCD(amount) => *amount,
        }
    }
}
