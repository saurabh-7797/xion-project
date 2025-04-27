use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, 
    Uint128, entry_point, Storage, Attribute, Event,
};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str;

use crate::errors::ContractError;

// Username validation constants
const MIN_USERNAME_LENGTH: usize = 3;
const MAX_USERNAME_LENGTH: usize = 32;
const ALLOWED_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";

// Storage items
pub const CONFIG: Item<Config> = Item::new("config");
pub const NEXT_TOKEN_ID: Item<u64> = Item::new("next_token_id");
pub const TOKENS: Map<&[u8], TokenInfo> = Map::new("tokens");
pub const USERNAME_MAP: Map<&str, u64> = Map::new("username_map"); // Stores username -> tokenId+1 (to handle tokenId 0)
pub const TOKEN_USERNAME: Map<&[u8], String> = Map::new("token_username");
pub const TOKEN_METADATA: Map<&[u8], String> = Map::new("token_metadata");
pub const TOKEN_OWNER: Map<&[u8], Addr> = Map::new("token_owner");

// Helper functions for storage key conversions
fn u64_to_key(val: u64) -> Vec<u8> {
    val.to_be_bytes().to_vec()
}

// Storage
pub const ADMIN: Item<Addr> = Item::new("admin");
pub const PAUSED: Item<bool> = Item::new("paused");
pub const USER_NFT: Map<&Addr, ProfileNFT> = Map::new("user_nft");
pub const NFT_COUNT: Item<u64> = Item::new("nft_count");
pub const WHITELIST: Map<&Addr, bool> = Map::new("whitelist");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub role_manager: Addr,
    pub name: String,
    pub symbol: String,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub role_manager: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateProfile {
        username: String,
        metadata_uri: String,
    },
    UpdateProfileMetadata {
        token_id: u64,
        new_metadata_uri: String,
    },
    TransferNft {
        recipient: String,
        token_id: u64,
    },
    // Add snake_case aliases for compatibility
    create_profile {
        username: String,
        metadata_uri: String,
    },
    update_profile_metadata {
        token_id: u64,
        new_metadata_uri: String,
    },
    transfer_nft {
        recipient: String,
        token_id: u64,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    UsernameExists {
        username: String,
    },
    GetProfileByTokenId {
        token_id: u64,
    },
    GetTokenIdByUsername {
        username: String,
    },
    SupportsInterface {
        interface_id: String,
    },
    OwnerOf {
        token_id: u64,
    },
    BalanceOf {
        owner: String,
    },
    // Add aliases with snake_case names for compatibility
    username_exists {
        username: String,
    },
    get_profile_by_token_id {
        token_id: u64,
    },
    get_token_id_by_username {
        username: String,
    },
    supports_interface {
        interface_id: String,
    },
    owner_of {
        token_id: u64,
    },
    balance_of {
        owner: String,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BoolResponse {
    pub result: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProfileResponse {
    pub username: String,
    pub metadata_uri: String,
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenIdResponse {
    pub token_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OwnerResponse {
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProfileNFT {
    pub token_id: u64,
    pub owner: Addr,
    pub name: String,
    pub description: String,
    pub image_uri: String,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TokenInfo {
    pub token_id: u64,
    pub owner: Addr,
    pub name: String,
    pub metadata_uri: String,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProfileNFTResponse {
    pub exists: bool,
    pub nft: Option<ProfileNFT>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TokenResponse {
    pub exists: bool,
    pub token: Option<TokenInfo>,
}

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let role_manager_addr = deps.api.addr_validate(&msg.role_manager)?;
    
    let config = Config {
        role_manager: role_manager_addr,
        name: msg.name,
        symbol: msg.symbol,
        owner: info.sender.clone(),
    };
    
    // Save config
    CONFIG.save(deps.storage, &config)?;
    
    // Initialize the next token ID as 0
    NEXT_TOKEN_ID.save(deps.storage, &0u64)?;
    
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", info.sender))
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::CreateProfile { username, metadata_uri } => {
            createProfile(deps, env, info, username, metadata_uri)
                .map_err(|e| cosmwasm_std::StdError::generic_err(format!("{:?}", e)))
        },
        ExecuteMsg::UpdateProfileMetadata { token_id, new_metadata_uri } => {
            updateProfileMetadata(deps, info, token_id, new_metadata_uri)
                .map_err(|e| cosmwasm_std::StdError::generic_err(format!("{:?}", e)))
        },
        ExecuteMsg::TransferNft { recipient, token_id } => {
            transferFrom(deps, env, info, recipient, token_id)
                .map_err(|e| cosmwasm_std::StdError::generic_err(format!("{:?}", e)))
        },
        // Handle snake_case aliases
        ExecuteMsg::create_profile { username, metadata_uri } => {
            createProfile(deps, env, info, username, metadata_uri)
                .map_err(|e| cosmwasm_std::StdError::generic_err(format!("{:?}", e)))
        },
        ExecuteMsg::update_profile_metadata { token_id, new_metadata_uri } => {
            updateProfileMetadata(deps, info, token_id, new_metadata_uri)
                .map_err(|e| cosmwasm_std::StdError::generic_err(format!("{:?}", e)))
        },
        ExecuteMsg::transfer_nft { recipient, token_id } => {
            transferFrom(deps, env, info, recipient, token_id)
                .map_err(|e| cosmwasm_std::StdError::generic_err(format!("{:?}", e)))
        },
    }
}

pub fn createProfile(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    username: String,
    metadata_uri: String,
) -> Result<Response, ContractError> {
    // Validate username
    if !_validateUsername(&username) {
        return Err(ContractError::InvalidUsername {});
    }
    
    let lower_username = _toLowerCase(&username);
    
    // Check if username exists (like Solidity's usernameExists)
    if usernameExists(deps.storage, &lower_username)? {
        return Err(ContractError::UsernameTaken {});
    }
    
    // Get the next token ID
    let token_id = NEXT_TOKEN_ID.load(deps.storage)?;
    NEXT_TOKEN_ID.save(deps.storage, &(token_id + 1))?;
    
    // Store the NFT info (like Solidity's _safeMint and _setTokenURI)
    let token = TokenInfo {
        token_id,
        owner: info.sender.clone(),
        name: username.clone(),
        metadata_uri: metadata_uri.clone(),
        created_at: env.block.time.seconds(),
    };
    
    TOKENS.save(deps.storage, &token_id.to_be_bytes(), &token)?;
    
    // Store username to token ID mapping (Solidity adds 1 to handle token ID 0)
    USERNAME_MAP.save(deps.storage, &lower_username, &(token_id + 1))?;
    
    // Store token ID to username mapping
    TOKEN_USERNAME.save(deps.storage, &token_id.to_be_bytes(), &username)?;
    
    // Store token metadata
    TOKEN_METADATA.save(deps.storage, &token_id.to_be_bytes(), &metadata_uri)?;
    
    // Store token owner
    TOKEN_OWNER.save(deps.storage, &token_id.to_be_bytes(), &info.sender)?;
    
    Ok(Response::new()
        .add_attribute("action", "create_profile")
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("owner", info.sender)
        .add_attribute("username", username))
}

pub fn updateProfileMetadata(
    deps: DepsMut,
    info: MessageInfo,
    token_id: u64,
    new_metadata_uri: String,
) -> Result<Response, ContractError> {
    // Check if token exists and verify sender is the owner
    let token = TOKENS.may_load(deps.storage, &token_id.to_be_bytes())?
        .ok_or(ContractError::CustomError { message: "Token does not exist".to_string() })?;
    
    if token.owner != info.sender {
        return Err(ContractError::NotTokenOwner {});
    }
    
    // Update token metadata
    TOKEN_METADATA.save(deps.storage, &token_id.to_be_bytes(), &new_metadata_uri)?;
    
    // Update token info
    let updated_token = TokenInfo {
        token_id,
        owner: token.owner,
        name: token.name,
        metadata_uri: new_metadata_uri.clone(),
        created_at: token.created_at,
    };
    TOKENS.save(deps.storage, &token_id.to_be_bytes(), &updated_token)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_profile_metadata")
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("new_metadata_uri", new_metadata_uri))
}

pub fn transferFrom(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: u64,
) -> Result<Response, ContractError> {
    // Check if token exists
    let token = TOKENS.may_load(deps.storage, &token_id.to_be_bytes())?
        .ok_or(ContractError::CustomError { message: "Token does not exist".to_string() })?;
    
    // Verify sender is the token owner
    if token.owner != info.sender {
        return Err(ContractError::NotTokenOwner {});
    }
    
    // Validate recipient address
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    
    // Update token owner
    let updated_token = TokenInfo {
        token_id,
        owner: recipient_addr.clone(),
        name: token.name,
        metadata_uri: token.metadata_uri,
        created_at: token.created_at,
    };
    
    TOKENS.save(deps.storage, &token_id.to_be_bytes(), &updated_token)?;
    TOKEN_OWNER.save(deps.storage, &token_id.to_be_bytes(), &recipient_addr)?;
    
    Ok(Response::new()
        .add_attribute("action", "transfer_nft")
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("from", info.sender)
        .add_attribute("to", recipient))
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::UsernameExists { username } => to_json_binary(&usernameExists_query(deps, username)?),
        QueryMsg::GetProfileByTokenId { token_id } => to_json_binary(&getProfileByTokenId(deps, token_id)?),
        QueryMsg::GetTokenIdByUsername { username } => to_json_binary(&getTokenIdByUsername(deps, username)?),
        QueryMsg::SupportsInterface { interface_id } => to_json_binary(&supportsInterface(deps, interface_id)?),
        QueryMsg::OwnerOf { token_id } => to_json_binary(&ownerOf(deps, token_id)?),
        QueryMsg::BalanceOf { owner } => to_json_binary(&balanceOf(deps, owner)?),
        QueryMsg::username_exists { username } => to_json_binary(&usernameExists_query(deps, username)?),
        QueryMsg::get_profile_by_token_id { token_id } => to_json_binary(&getProfileByTokenId(deps, token_id)?),
        QueryMsg::get_token_id_by_username { username } => to_json_binary(&getTokenIdByUsername(deps, username)?),
        QueryMsg::supports_interface { interface_id } => to_json_binary(&supportsInterface(deps, interface_id)?),
        QueryMsg::owner_of { token_id } => to_json_binary(&ownerOf(deps, token_id)?),
        QueryMsg::balance_of { owner } => to_json_binary(&balanceOf(deps, owner)?),
    }
}

pub fn usernameExists_query(deps: Deps, username: String) -> StdResult<BoolResponse> {
    let lower_username = _toLowerCase(&username);
    let result = usernameExists(deps.storage, &lower_username)?;
    Ok(BoolResponse { result })
}

fn usernameExists(storage: &dyn cosmwasm_std::Storage, username: &str) -> StdResult<bool> {
    Ok(USERNAME_MAP.may_load(storage, username)?.is_some())
}

pub fn getProfileByTokenId(deps: Deps, token_id: u64) -> StdResult<ProfileResponse> {
    // If token doesn't exist, this will return an error (similar to ownerOf revert in Solidity)
    let token_owner = TOKEN_OWNER.may_load(deps.storage, &token_id.to_be_bytes())?
        .ok_or_else(|| cosmwasm_std::StdError::generic_err("Token does not exist"))?;
    
    // Check that username exists for this token (similar to the Solidity require statement)
    let username = TOKEN_USERNAME.may_load(deps.storage, &token_id.to_be_bytes())?
        .ok_or_else(|| cosmwasm_std::StdError::generic_err("Profile does not exist"))?;
    
    // Get the metadata URI (similar to tokenURI in Solidity)
    let metadata_uri = TOKEN_METADATA.may_load(deps.storage, &token_id.to_be_bytes())?
        .ok_or_else(|| cosmwasm_std::StdError::generic_err("Token metadata not found"))?;
    
    // Return the same structure as the Solidity function
    Ok(ProfileResponse {
        username,
        metadata_uri,
        owner: token_owner.to_string(),
    })
}

pub fn getTokenIdByUsername(deps: Deps, username: String) -> StdResult<TokenIdResponse> {
    // Convert to lowercase, just like in the Solidity function
    let lower_username = _toLowerCase(&username);
    
    // Get the stored ID, requiring it to exist (like the Solidity require statement)
    let stored_id = USERNAME_MAP.may_load(deps.storage, &lower_username)?
        .ok_or_else(|| cosmwasm_std::StdError::generic_err("Username does not exist"))?;
    
    // Adjust for the +1 storage pattern used in Solidity
    Ok(TokenIdResponse { token_id: stored_id - 1 })
}

// Similar to Solidity's supportsInterface
fn supportsInterface(_deps: Deps, interface_id: String) -> StdResult<BoolResponse> {
    // Implement logic to check interface support here
    // For ERC721, we would check common interfaces
    let supports = match interface_id.as_str() {
        "0x80ac58cd" => true, // ERC721 interface
        "0x5b5e139f" => true, // ERC721Metadata interface
        _ => false,
    };
    
    Ok(BoolResponse { result: supports })
}

pub fn ownerOf(deps: Deps, token_id: u64) -> StdResult<OwnerResponse> {
    let token = TOKENS.may_load(deps.storage, &token_id.to_be_bytes())?
        .ok_or_else(|| cosmwasm_std::StdError::generic_err("Token does not exist"))?;
    
    Ok(OwnerResponse { owner: token.owner.to_string() })
}

// Validate username, matching Solidity's _validateUsername
pub fn _validateUsername(username: &str) -> bool {
    let username_bytes = username.as_bytes();
    if username_bytes.len() < MIN_USERNAME_LENGTH || username_bytes.len() > MAX_USERNAME_LENGTH {
        return false;
    }
    
    for c in username.chars() {
        if !ALLOWED_CHARS.contains(c) {
            return false;
        }
    }
    
    true
}

// Convert string to lowercase, matching Solidity's _toLowerCase
pub fn _toLowerCase(s: &str) -> String {
    s.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        
        let msg = InstantiateMsg {
            role_manager: "role_manager".to_string(),
            name: "Profile NFT".to_string(),
            symbol: "PROFILE".to_string(),
        };
        
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        
        // Check config values
        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!("Profile NFT", config.name);
        assert_eq!("PROFILE", config.symbol);
    }

    #[test]
    fn test_create_profile() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        
        // Instantiate contract
        let msg = InstantiateMsg {
            role_manager: "role_manager".to_string(),
            name: "Profile NFT".to_string(),
            symbol: "PROFILE".to_string(),
        };
        
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        
        // Create a profile
        let create_msg = ExecuteMsg::CreateProfile {
            username: "alice".to_string(),
            metadata_uri: "ipfs://Qm...1".to_string(),
        };
        
        let res = execute(deps.as_mut(), mock_env(), info.clone(), create_msg).unwrap();
        assert_eq!("create_profile", res.attributes[0].value);
        assert_eq!("0", res.attributes[1].value); // token_id
        
        // Check that the profile was created correctly
        let profile = getProfileByTokenId(deps.as_ref(), 0).unwrap();
        assert_eq!("alice", profile.username);
        assert_eq!("ipfs://Qm...1", profile.metadata_uri);
        assert_eq!("creator", profile.owner);
        
        // Check that token ID can be retrieved by username
        let token_id_response = getTokenIdByUsername(deps.as_ref(), "alice".to_string()).unwrap();
        assert_eq!(0, token_id_response.token_id);
        
        // Check username exists query
        let exists = usernameExists_query(deps.as_ref(), "alice".to_string()).unwrap();
        assert_eq!(true, exists.result);
        
        // Check owner of token
        let owner = ownerOf(deps.as_ref(), 0).unwrap();
        assert_eq!("creator", owner.owner);
    }
    
    #[test]
    fn test_update_metadata() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        
        // Instantiate and create profile
        let msg = InstantiateMsg {
            role_manager: "role_manager".to_string(),
            name: "Profile NFT".to_string(),
            symbol: "PROFILE".to_string(),
        };
        
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        
        let create_msg = ExecuteMsg::CreateProfile {
            username: "alice".to_string(),
            metadata_uri: "ipfs://Qm...1".to_string(),
        };
        
        execute(deps.as_mut(), mock_env(), info.clone(), create_msg).unwrap();
        
        // Update metadata
        let update_msg = ExecuteMsg::UpdateProfileMetadata {
            token_id: 0,
            new_metadata_uri: "ipfs://Qm...2".to_string(),
        };
        
        let res = execute(deps.as_mut(), mock_env(), info.clone(), update_msg).unwrap();
        assert_eq!("update_profile_metadata", res.attributes[0].value);
        
        // Check that metadata was updated
        let profile = getProfileByTokenId(deps.as_ref(), 0).unwrap();
        assert_eq!("ipfs://Qm...2", profile.metadata_uri);
    }
    
    #[test]
    fn test_transfer_nft() {
        let mut deps = mock_dependencies();
        let creator = mock_info("creator", &[]);
        let recipient = "recipient";
        
        // Instantiate and create profile
        let msg = InstantiateMsg {
            role_manager: "role_manager".to_string(),
            name: "Profile NFT".to_string(),
            symbol: "PROFILE".to_string(),
        };
        
        instantiate(deps.as_mut(), mock_env(), creator.clone(), msg).unwrap();
        
        let create_msg = ExecuteMsg::CreateProfile {
            username: "alice".to_string(),
            metadata_uri: "ipfs://Qm...1".to_string(),
        };
        
        execute(deps.as_mut(), mock_env(), creator.clone(), create_msg).unwrap();
        
        // Transfer NFT
        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: 0,
        };
        
        let res = execute(deps.as_mut(), mock_env(), creator.clone(), transfer_msg).unwrap();
        assert_eq!("transfer_nft", res.attributes[0].value);
        
        // Check new owner
        let owner = ownerOf(deps.as_ref(), 0).unwrap();
        assert_eq!(recipient, owner.owner);
    }
}

// Replace deprecated from_binary with to_json_binary
fn from_json<T: serde::de::DeserializeOwned>(data: &Binary) -> StdResult<T> {
    cosmwasm_std::from_json(data)
}

// Mint profile NFT
pub fn mintProfileNFT(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    description: String,
    image_uri: String,
) -> Result<Response, ContractError> {
    // Check if contract is paused
    let paused = PAUSED.may_load(deps.storage)?.unwrap_or(false);
    if paused {
        return Err(ContractError::CustomError { 
            message: "Contract is paused".to_string() 
        });
    }
    
    // Check if user already has a profile NFT
    if USER_NFT.may_load(deps.storage, &info.sender)?.is_some() {
        return Err(ContractError::CustomError { 
            message: "User already has a profile NFT".to_string() 
        });
    }
    
    // Generate token ID
    let mut next_token_id = NEXT_TOKEN_ID.load(deps.storage)?;
    let token_id = next_token_id;
    next_token_id += 1;
    NEXT_TOKEN_ID.save(deps.storage, &next_token_id)?;
    
    // Create profile NFT
    let token = TokenInfo {
        token_id,
        owner: info.sender.clone(),
        name: name.clone(),
        metadata_uri: description.clone(),
        created_at: env.block.time.seconds(),
    };
    
    // Save token
    TOKENS.save(deps.storage, &u64_to_key(token_id), &token)?;
    
    // Update NFT count
    NFT_COUNT.save(deps.storage, &token_id)?;
    
    // Update user NFT
    let profile_nft = ProfileNFT {
        token_id,
        owner: info.sender.clone(),
        name,
        description,
        image_uri,
        created_at: env.block.time.seconds(),
    };
    
    USER_NFT.save(deps.storage, &info.sender, &profile_nft)?;
    
    Ok(Response::new()
        .add_attribute("action", "mint_profile_nft")
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("owner", info.sender.to_string()))
}

// Update profile NFT
pub fn updateProfileNFT(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: Option<String>,
    description: Option<String>,
    image_uri: Option<String>,
) -> Result<Response, ContractError> {
    // Check if contract is paused
    let paused = PAUSED.may_load(deps.storage)?.unwrap_or(false);
    if paused {
        return Err(ContractError::CustomError { 
            message: "Contract is paused".to_string() 
        });
    }
    
    // Get existing profile
    let profile_nft = match USER_NFT.may_load(deps.storage, &info.sender)? {
        Some(nft) => nft,
        None => return Err(ContractError::CustomError { 
            message: "User does not have a profile NFT".to_string() 
        }),
    };
    
    // Get token
    let mut token = TOKENS.load(deps.storage, &u64_to_key(profile_nft.token_id))?;
    
    // Update fields if provided
    if let Some(name_val) = name {
        token.name = name_val;
    }
    
    if let Some(desc_val) = description {
        token.metadata_uri = desc_val;
    }
    
    if let Some(uri_val) = image_uri {
        token.metadata_uri = uri_val;
    }
    
    // Save updated token
    TOKENS.save(deps.storage, &u64_to_key(token.token_id), &token)?;
    
    // Save updated profile NFT
    USER_NFT.save(deps.storage, &info.sender, &profile_nft)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_profile_nft")
        .add_attribute("token_id", profile_nft.token_id.to_string())
        .add_attribute("owner", info.sender.to_string()))
}

// Query profile NFT
pub fn queryProfileNFT(deps: Deps, owner: Addr) -> StdResult<Binary> {
    match USER_NFT.may_load(deps.storage, &owner)? {
        Some(nft) => to_json_binary(&nft),
        None => to_json_binary(&ProfileNFTResponse { exists: false, nft: None }),
    }
}

// Helper function to check if a user is whitelisted
pub fn is_whitelisted(deps: Deps, addr: &Addr) -> StdResult<bool> {
    Ok(WHITELIST.may_load(deps.storage, addr)?.unwrap_or(false))
}

// Set admin
pub fn setAdmin(deps: DepsMut, info: MessageInfo, new_admin: Addr) -> Result<Response, ContractError> {
    // Check if sender is admin
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }
    
    // Update admin
    ADMIN.save(deps.storage, &new_admin)?;
    
    Ok(Response::new()
        .add_attribute("action", "set_admin")
        .add_attribute("new_admin", new_admin.to_string()))
}

// Set paused state
pub fn setPaused(deps: DepsMut, info: MessageInfo, paused: bool) -> Result<Response, ContractError> {
    // Check if sender is admin
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }
    
    // Update paused state
    PAUSED.save(deps.storage, &paused)?;
    
    Ok(Response::new()
        .add_attribute("action", "set_paused")
        .add_attribute("paused", paused.to_string()))
}

// Add to whitelist
pub fn addToWhitelist(deps: DepsMut, info: MessageInfo, user: Addr) -> Result<Response, ContractError> {
    // Check if sender is admin
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }
    
    // Add to whitelist
    WHITELIST.save(deps.storage, &user, &true)?;
    
    Ok(Response::new()
        .add_attribute("action", "add_to_whitelist")
        .add_attribute("user", user.to_string()))
}

// Remove from whitelist
pub fn removeFromWhitelist(deps: DepsMut, info: MessageInfo, user: Addr) -> Result<Response, ContractError> {
    // Check if sender is admin
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }
    
    // Remove from whitelist
    WHITELIST.remove(deps.storage, &user);
    
    Ok(Response::new()
        .add_attribute("action", "remove_from_whitelist")
        .add_attribute("user", user.to_string()))
}

// Query Token info
pub fn queryToken(deps: Deps, token_id: u64) -> StdResult<Binary> {
    match TOKENS.may_load(deps.storage, &u64_to_key(token_id))? {
        Some(token) => to_json_binary(&token),
        None => to_json_binary(&TokenResponse { exists: false, token: None }),
    }
}

// Add after ownerOf function
pub fn balanceOf(deps: Deps, owner: String) -> StdResult<Uint128> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let mut count = 0u128;
    
    // Get the next token ID to know how many tokens to check
    let next_token_id = NEXT_TOKEN_ID.load(deps.storage)?;
    
    // Iterate through all tokens and count those owned by the specified address
    for token_id in 0..next_token_id {
        if let Ok(token_owner) = TOKEN_OWNER.load(deps.storage, &token_id.to_be_bytes()) {
            if token_owner == owner_addr {
                count += 1;
            }
        }
    }
    
    Ok(Uint128::from(count))
} 