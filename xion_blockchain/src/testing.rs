use cosmwasm_std::{
    Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Storage,
};
use std::marker::PhantomData;

// Define custom mock implementations

pub struct MockStorage {
    data: PhantomData<()>,
}

pub struct MockApi {
    data: PhantomData<()>,
}

pub struct MockQuerier {
    data: PhantomData<()>,
}

pub struct MockDeps {
    pub storage: MockStorage,
    pub api: MockApi,
    pub querier: MockQuerier,
}

// Env constructor
pub fn mock_env() -> Env {
    Env {
        block: cosmwasm_std::BlockInfo {
            height: 12345,
            time: cosmwasm_std::Timestamp::from_seconds(1571797419),
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        transaction: None,
        contract: cosmwasm_std::ContractInfo {
            address: Addr::unchecked("cosmos2contract"),
        },
    }
}

// MessageInfo constructor
pub fn mock_info(sender: &str, funds: &[Coin]) -> MessageInfo {
    MessageInfo {
        sender: Addr::unchecked(sender),
        funds: funds.to_vec(),
    }
}

// MockDeps constructor
pub fn mock_dependencies() -> MockDeps {
    MockDeps {
        storage: MockStorage { data: PhantomData },
        api: MockApi { data: PhantomData },
        querier: MockQuerier { data: PhantomData },
    }
}

// Implement traits for mocks
impl MockDeps {
    pub fn as_mut(&mut self) -> DepsMut {
        unimplemented!("This is a mock implementation for compilation only")
    }
    
    pub fn as_ref(&self) -> Deps {
        unimplemented!("This is a mock implementation for compilation only")
    }
}

impl Storage for MockStorage {
    fn get(&self, _key: &[u8]) -> Option<Vec<u8>> {
        unimplemented!("This is a mock implementation for compilation only")
    }

    fn range<'a>(&'a self, _start: Option<&[u8]>, _end: Option<&[u8]>, _order: cosmwasm_std::Order) -> Box<(dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a)> {
        unimplemented!("This is a mock implementation for compilation only")
    }

    fn set(&mut self, _key: &[u8], _value: &[u8]) {
        unimplemented!("This is a mock implementation for compilation only")
    }

    fn remove(&mut self, _key: &[u8]) {
        unimplemented!("This is a mock implementation for compilation only")
    }
} 