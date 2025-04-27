use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, 
    entry_point, Storage, Order, Uint128, SubMsg, WasmMsg, CosmosMsg, BankMsg, Coin,
};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::convert::TryFrom;
use crate::errors::ContractError;
use hex;

// Define structs and enums that match the Solidity contract

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum NFTType {
    ERC721,
    ERC1155,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum JoinType {
    PUBLIC,
    PRIVATE,
    INVITE_CODE,
    NFT_GATED,
    MULTI_NFT,
    ANY_NFT,
    NFTRequired,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum MemberStatus {
    NONE,
    PENDING,
    ACTIVE,
    BANNED,
    Admin,
    Member,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NFTRequirement {
    pub nft_contract: Option<Addr>,
    pub nft_type: NFTType,
    pub is_mandatory: bool,
    pub min_amount: u64,
    pub token_ids: Vec<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InviteCode {
    pub code_hash: Vec<u8>,
    pub max_uses: u64,
    pub used_count: u64,
    pub expiry_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MergeRequest {
    pub source_tribe_id: u64,
    pub target_tribe_id: u64,
    pub request_time: u64,
    pub approved: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TribeConfigView {
    pub join_type: JoinType,
    pub entry_fee: Uint128,
    pub nft_requirements: Vec<NFTRequirement>,
    pub can_merge: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub role_manager: Addr,
    pub next_tribe_id: u64,
    pub next_merge_request_id: u64,
}

// Storage definitions using cw-storage-plus
const CONFIG: Item<Config> = Item::new("config");
const TRIBES: Map<&[u8], TribeData> = Map::new("tribes");
const TRIBE_META: Map<&str, TribeMeta> = Map::new("tribe_meta");
const MEMBER_STATUS: Map<&str, MemberStatus> = Map::new("member_status");
const MEMBER_COUNT: Map<&str, u64> = Map::new("member_count");
const IS_MEMBER: Map<&str, bool> = Map::new("is_member");
const MERGE_REQUEST: Map<&str, MergeRequest> = Map::new("merge_request");
const INVITE_CODE: Map<&str, InviteCode> = Map::new("invite_code");
const TRIBE_COUNT: Item<u64> = Item::new("tribe_count");
const TRIBE_MEMBERS: Map<(Vec<u8>, &Addr), TribeMember> = Map::new("tribe_members");
const USER_TRIBES: Map<&Addr, Vec<u64>> = Map::new("user_tribes");
const WHITELIST: Map<&Addr, bool> = Map::new("whitelist");

// Tribe metadata that doesn't fit in the main mapping structure
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TribeMeta {
    pub name: String,
    pub metadata: String,
    pub admin: Addr,
    pub whitelist: Vec<Addr>,
    pub join_type: JoinType,
    pub entry_fee: Uint128,
    pub nft_requirements: Vec<NFTRequirement>,
    pub can_merge: bool,
    pub is_active: bool,
    pub member_count: Option<u64>,
}

// Add the missing TribeData struct definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TribeData {
    pub id: u64,
    pub name: String,
    pub admin: Addr,
    pub join_type: JoinType,
    pub nft_contract: Option<Addr>,
    pub nft_type: Option<NFTType>,
    pub nft_required: Option<u64>,
    pub entry_fee: Uint128,
    pub is_active: bool,
}

// Add the missing TribeMember struct definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TribeMember {
    pub tribe_id: u64,
    pub member: Addr,
    pub status: MemberStatus,
    pub joined_at: u64,
}

// Messages

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub role_manager: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateTribe {
        name: String,
        metadata: String,
        admins: Vec<String>,
        join_type: JoinType,
        entry_fee: Uint128,
        nft_requirements: Vec<NFTRequirement>,
    },
    UpdateTribe {
        tribe_id: u64,
        new_metadata: String,
        updated_whitelist: Vec<String>,
    },
    UpdateTribeConfig {
        tribe_id: u64,
        join_type: JoinType,
        entry_fee: Uint128,
        nft_requirements: Vec<NFTRequirement>,
    },
    JoinTribe {
        tribe_id: u64,
    },
    RequestToJoinTribe {
        tribe_id: u64,
    },
    ApproveMember {
        tribe_id: u64,
        member: String,
    },
    RejectMember {
        tribe_id: u64,
        member: String,
    },
    BanMember {
        tribe_id: u64,
        member: String,
    },
    JoinTribeWithCode {
        tribe_id: u64,
        invite_code: Vec<u8>,
    },
    CreateInviteCode {
        tribe_id: u64,
        code: String,
        max_uses: u64,
        expiry_time: u64,
    },
    RequestMerge {
        source_tribe_id: u64,
        target_tribe_id: u64,
    },
    ApproveMerge {
        merge_request_id: u64,
    },
    ExecuteMerge {
        merge_request_id: u64,
    },
    RevokeInviteCode {
        tribe_id: u64,
        code: String,
    },
    CancelMerge {
        merge_request_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTribeAdmin {
        tribe_id: u64,
    },
    GetTribeWhitelist {
        tribe_id: u64,
    },
    IsAddressWhitelisted {
        tribe_id: u64,
        user: String,
    },
    GetMemberStatus {
        tribe_id: u64,
        member: String,
    },
    GetTribeConfigView {
        tribe_id: u64,
    },
    GetMemberCount {
        tribe_id: u64,
    },
    GetUserTribes {
        user: String,
    },
    GetInviteCodeStatus {
        tribe_id: u64,
        code: String,
    },
    GetMergeRequest {
        request_id: u64,
    },
    GetTribeDetails {
        tribe_id: u64,
    },
}

// Query responses
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AdminResponse {
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistResponse {
    pub whitelist: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BoolResponse {
    pub result: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MemberStatusResponse {
    pub status: MemberStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TribeConfigViewResponse {
    pub config: TribeConfigView,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MemberCountResponse {
    pub count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserTribesResponse {
    pub tribe_ids: Vec<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InviteCodeStatusResponse {
    pub valid: bool,
    pub remaining_uses: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MergeRequestResponse {
    pub request: MergeRequest,
}

// External contract interfaces

// Simple interface for querying role manager
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoleManagerQuery {
    HasRole {
        user: String,
        role: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoleResponse {
    pub has_role: bool,
}

// Simple interfaces for NFT contracts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Erc721Query {
    BalanceOf {
        owner: String,
    },
    OwnerOf {
        token_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Erc1155Query {
    BalanceOf {
        account: String,
        id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OwnerResponse {
    pub owner: String,
}

// Contract implementation
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let role_manager = deps.api.addr_validate(&msg.role_manager)?;
    
    let config = Config {
        role_manager,
        next_tribe_id: 0,
        next_merge_request_id: 0,
    };
    
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::CreateTribe {
            name,
            metadata,
            admins,
            join_type,
            entry_fee,
            nft_requirements,
        } => {
            createTribe(deps, env, info, name, metadata, admins, join_type, entry_fee, nft_requirements)
                .map_err(|e| cosmwasm_std::StdError::generic_err(format!("{:?}", e)))
        },
        ExecuteMsg::UpdateTribe { 
            tribe_id, 
            new_metadata, 
            updated_whitelist 
        } => updateTribe(deps, env, info, tribe_id, new_metadata, updated_whitelist),
        ExecuteMsg::UpdateTribeConfig { 
            tribe_id, 
            join_type, 
            entry_fee, 
            nft_requirements 
        } => updateTribeConfig(deps, env, info, tribe_id, join_type, entry_fee, nft_requirements),
        ExecuteMsg::JoinTribe { tribe_id } => {
            joinTribe(deps, env, info, tribe_id)
                .map_err(|e| cosmwasm_std::StdError::generic_err(format!("{:?}", e)))
        },
        ExecuteMsg::RequestToJoinTribe { tribe_id } => requestToJoinTribe(deps, env, info, tribe_id),
        ExecuteMsg::ApproveMember { tribe_id, member } => approveMember(deps, env, info, tribe_id, member),
        ExecuteMsg::RejectMember { tribe_id, member } => rejectMember(deps, env, info, tribe_id, member),
        ExecuteMsg::BanMember { tribe_id, member } => banMember(deps, env, info, tribe_id, member),
        ExecuteMsg::JoinTribeWithCode { tribe_id, invite_code } => joinTribeWithCode(deps, env, info, tribe_id, invite_code),
        ExecuteMsg::CreateInviteCode { tribe_id, code, max_uses, expiry_time } => createInviteCode(deps, env, info, tribe_id, code, max_uses, expiry_time),
        ExecuteMsg::RequestMerge { source_tribe_id, target_tribe_id } => requestMerge(deps, env, info, source_tribe_id, target_tribe_id),
        ExecuteMsg::ApproveMerge { merge_request_id } => approveMerge(deps, env, info, merge_request_id),
        ExecuteMsg::ExecuteMerge { merge_request_id } => executeMerge(deps, env, info, merge_request_id),
        ExecuteMsg::RevokeInviteCode { tribe_id, code } => revokeInviteCode(deps, env, info, tribe_id, code),
        ExecuteMsg::CancelMerge { merge_request_id } => cancelMerge(deps, env, info, merge_request_id),
        _ => Err(cosmwasm_std::StdError::generic_err("Unsupported operation")),
    }
}

// Helper function to validate a tribe admin
fn is_tribe_admin(
    deps: Deps,
    storage: &dyn Storage,
    tribe_id: u64,
    addr: &Addr,
) -> StdResult<bool> {
    let tribe_meta = TRIBE_META.load(storage, &tribe_id.to_string())?;
    
    if tribe_meta.admin == *addr {
        return Ok(true);
    }
    
    // Check if addr has MODERATOR_ROLE
    let config = get_config(storage)?;
    let msg = to_json_binary(&RoleManagerQuery::HasRole {
        user: addr.to_string(),
        role: "MODERATOR_ROLE".to_string(),
    })?;
    
    let query = cosmwasm_std::WasmQuery::Smart {
        contract_addr: config.role_manager.to_string(),
        msg,
    };
    
    match deps.querier.query::<RoleResponse>(&query.into()) {
        Ok(response) => Ok(response.has_role),
        Err(_) => Ok(false),
    }
}

fn get_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

// Helper functions for storage key conversions
fn u64_to_key(val: u64) -> Vec<u8> {
    val.to_be_bytes().to_vec()
}

pub fn createTribe(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    metadata: String,
    admins: Vec<String>,
    join_type: JoinType,
    entry_fee: Uint128,
    nft_requirements: Vec<NFTRequirement>,
) -> Result<Response, ContractError> {
    // Check if whitelisted
    if !is_whitelisted(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }
    
    // Validate NFT requirements if join type is NFT_REQUIRED
    if join_type == JoinType::NFTRequired {
        if nft_requirements.is_empty() {
            return Err(ContractError::CustomError { message: "NFT details required for NFT_REQUIRED join type".to_string() });
        }
    }
    
    // Get new tribe ID
    let mut config = CONFIG.load(deps.storage)?;
    let tribe_id = config.next_tribe_id;
    config.next_tribe_id += 1;
    CONFIG.save(deps.storage, &config)?;
    
    // Validate admins addresses
    let mut validated_admins: Vec<Addr> = Vec::with_capacity(admins.len());
    for admin in admins {
        validated_admins.push(deps.api.addr_validate(&admin)?);
    }
    
    // Create tribe metadata
    let tribe_meta = TribeMeta {
        name: name.clone(),
        metadata,
        admin: info.sender.clone(),
        whitelist: validated_admins,
        join_type: join_type.clone(),
        entry_fee,
        nft_requirements,
        can_merge: true,
        is_active: true,
        member_count: None,
    };
    
    // Save tribe metadata
    TRIBE_META.save(deps.storage, &tribe_id.to_string(), &tribe_meta)?;
    
    // Add creator as active member
    let member_status_key = format!("{}:{}", tribe_id, info.sender);
    MEMBER_STATUS.save(deps.storage, &member_status_key, &MemberStatus::ACTIVE)?;
    
    // Set member count to 1
    MEMBER_COUNT.save(deps.storage, &tribe_id.to_string(), &1u64)?;
    
    // Mark creator as member
    let is_member_key = format!("{}:{}", tribe_id, info.sender);
    IS_MEMBER.save(deps.storage, &is_member_key, &true)?;
    
    Ok(Response::new()
        .add_attribute("action", "create_tribe")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("creator", info.sender)
        .add_attribute("name", name)
        .add_attribute("join_type", format!("{:?}", join_type)))
}

// This macro ensures the caller is a tribe admin
macro_rules! only_tribe_admin {
    ($deps:expr, $info:expr, $tribe_id:expr) => {{
        if !is_tribe_admin($deps.as_ref(), $deps.storage, $tribe_id, &$info.sender)? {
            return Err(cosmwasm_std::StdError::generic_err("Not tribe admin"));
        }
    }};
}

pub fn updateTribe(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    new_metadata: String,
    updated_whitelist: Vec<String>,
) -> StdResult<Response> {
    // Check if sender is tribe admin
    only_tribe_admin!(deps, info, tribe_id);
    
    // Get tribe metadata
    let mut tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    // Update metadata
    tribe_meta.metadata = new_metadata;
    
    // Convert and validate whitelist addresses
    let mut validated_whitelist: Vec<Addr> = Vec::with_capacity(updated_whitelist.len());
    for addr in updated_whitelist {
        validated_whitelist.push(deps.api.addr_validate(&addr)?);
    }
    tribe_meta.whitelist = validated_whitelist;
    
    // Save updated tribe metadata
    TRIBE_META.save(deps.storage, &tribe_id.to_string(), &tribe_meta)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_tribe")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("join_type", format!("{:?}", tribe_meta.join_type))
        .add_attribute("entry_fee", tribe_meta.entry_fee.to_string()))
}

pub fn updateTribeConfig(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    join_type: JoinType,
    entry_fee: Uint128,
    nft_requirements: Vec<NFTRequirement>,
) -> StdResult<Response> {
    // Check if sender is tribe admin
    only_tribe_admin!(deps, info, tribe_id);
    
    // Get tribe metadata
    let mut tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    // Update config
    tribe_meta.join_type = join_type.clone();
    tribe_meta.entry_fee = entry_fee;
    tribe_meta.nft_requirements = nft_requirements;
    
    // Save updated tribe metadata
    TRIBE_META.save(deps.storage, &tribe_id.to_string(), &tribe_meta)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_tribe_config")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("join_type", format!("{:?}", join_type))
        .add_attribute("entry_fee", entry_fee.to_string()))
}

pub fn joinTribe(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tribe_id: u64,
) -> Result<Response, ContractError> {
    // Validate tribe ID
    let config = get_config(deps.storage)?;
    if tribe_id >= config.next_tribe_id {
        return Err(ContractError::CustomError { message: "Invalid tribe ID".to_string() });
    }
    
    // Check if user is already a member
    let is_member_key = format!("{}:{}", tribe_id, info.sender);
    if IS_MEMBER.may_load(deps.storage, &is_member_key)?.is_some() {
        return Err(ContractError::CustomError { message: "Already a member".to_string() });
    }
    
    // Check if banned
    let member_status_key = format!("{}:{}", tribe_id, info.sender);
    if let Some(status) = MEMBER_STATUS.may_load(deps.storage, &member_status_key)? {
        if status == MemberStatus::BANNED {
            return Err(ContractError::CustomError { message: "User is banned".to_string() });
        }
    }
    
    // Get tribe metadata
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    // Check if tribe is active
    if !tribe_meta.is_active {
        return Err(ContractError::CustomError { message: "Tribe not active".to_string() });
    }
    
    // Check join type
    if tribe_meta.join_type == JoinType::PRIVATE || tribe_meta.join_type == JoinType::INVITE_CODE {
        return Err(ContractError::CustomError { message: "Tribe not public or requires invite code".to_string() });
    }
    
    // Check NFT requirements
    if tribe_meta.join_type == JoinType::NFT_GATED || 
       tribe_meta.join_type == JoinType::MULTI_NFT || 
       tribe_meta.join_type == JoinType::ANY_NFT {
        if !_validateNFTRequirements(deps.as_ref(), tribe_id, &info.sender)? {
            return Err(ContractError::CustomError { message: "NFT requirements not met".to_string() });
        }
    }
    
    // Add as active member
    MEMBER_STATUS.save(deps.storage, &member_status_key, &MemberStatus::ACTIVE)?;
    
    // Mark as member
    IS_MEMBER.save(deps.storage, &is_member_key, &true)?;
    
    // Update member count
    let current_count = MEMBER_COUNT.may_load(deps.storage, &tribe_id.to_string())?.unwrap_or(0);
    MEMBER_COUNT.save(deps.storage, &tribe_id.to_string(), &(current_count + 1))?;
    
    // Add to whitelist
    let mut tribe_meta = tribe_meta;
    tribe_meta.whitelist.push(info.sender.clone());
    TRIBE_META.save(deps.storage, &tribe_id.to_string(), &tribe_meta)?;
    
    Ok(Response::new()
        .add_attribute("action", "join_tribe")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("member", info.sender.to_string()))
}

fn _validateNFTRequirements(
    deps: Deps,
    tribe_id: u64,
    user: &Addr,
) -> StdResult<bool> {
    // Get tribe metadata
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    let requirements = &tribe_meta.nft_requirements;
    if requirements.is_empty() {
        return Ok(true);
    }
    
    match tribe_meta.join_type {
        JoinType::MULTI_NFT => {
            // All mandatory NFTs must be held
            for req in requirements {
                if req.is_mandatory && !_validateSingleNFTRequirement(deps, req, user)? {
                    return Ok(false);
                }
            }
            Ok(true)
        },
        JoinType::ANY_NFT => {
            // At least one NFT must be held
            for req in requirements {
                if _validateSingleNFTRequirement(deps, req, user)? {
                    return Ok(true);
                }
            }
            Ok(false)
        },
        _ => {
            // Default behavior (NFT_GATED) - all NFTs must be held
            for req in requirements {
                if !_validateSingleNFTRequirement(deps, req, user)? {
                    return Ok(false);
                }
            }
            Ok(true)
        },
    }
}

fn _validateSingleNFTRequirement(
    deps: Deps,
    req: &NFTRequirement,
    user: &Addr,
) -> StdResult<bool> {
    let nft_contract = match &req.nft_contract {
        Some(addr) => addr,
        None => return Ok(false),
    };
    
    match req.nft_type {
        NFTType::ERC721 => {
            // Query balance
            let balance_msg = to_json_binary(&Erc721Query::BalanceOf {
                owner: user.to_string(),
            })?;
            
            let balance_query = cosmwasm_std::WasmQuery::Smart {
                contract_addr: nft_contract.to_string(),
                msg: balance_msg,
            };
            
            match deps.querier.query::<BalanceResponse>(&balance_query.into()) {
                Ok(balance_resp) => {
                    if balance_resp.balance.u128() < req.min_amount as u128 {
                        return Ok(false);
                    }
                    
                    // If no specific tokens required, just check balance
                    if req.token_ids.is_empty() {
                        return Ok(true);
                    }
                    
                    // Check specific token IDs if specified
                    for token_id in &req.token_ids {
                        let owner_msg = to_json_binary(&Erc721Query::OwnerOf {
                            token_id: *token_id,
                        })?;
                        
                        let owner_query = cosmwasm_std::WasmQuery::Smart {
                            contract_addr: nft_contract.to_string(),
                            msg: owner_msg,
                        };
                        
                        match deps.querier.query::<OwnerResponse>(&owner_query.into()) {
                            Ok(owner_resp) => {
                                let owner_addr = deps.api.addr_validate(&owner_resp.owner)?;
                                if owner_addr != *user {
                                    return Ok(false);
                                }
                            },
                            Err(_) => return Ok(false),
                        }
                    }
                    
                    Ok(true)
                },
                Err(_) => Ok(false),
            }
        },
        NFTType::ERC1155 => {
            if req.token_ids.is_empty() {
                return Ok(false); // ERC1155 must specify token IDs
            }
            
            for token_id in &req.token_ids {
                let balance_msg = to_json_binary(&Erc1155Query::BalanceOf {
                    account: user.to_string(),
                    id: *token_id,
                })?;
                
                let balance_query = cosmwasm_std::WasmQuery::Smart {
                    contract_addr: nft_contract.to_string(),
                    msg: balance_msg,
                };
                
                match deps.querier.query::<BalanceResponse>(&balance_query.into()) {
                    Ok(balance_resp) => {
                        if balance_resp.balance.u128() < req.min_amount as u128 {
                            return Ok(false);
                        }
                    },
                    Err(_) => return Ok(false),
                }
            }
            
            Ok(true)
        },
    }
}

pub fn requestToJoinTribe(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
) -> StdResult<Response> {
    // Check if tribe exists
    let tribe_meta = TRIBE_META.may_load(deps.storage, &tribe_id.to_string())?
        .ok_or_else(|| cosmwasm_std::StdError::generic_err("Tribe not found"))?;
    
    // Check join type
    if tribe_meta.join_type != JoinType::PRIVATE {
        return Err(cosmwasm_std::StdError::generic_err("Tribe is not private. Use joinTribe"));
    }
    
    // Check if address is already a member
    if is_tribe_member(deps.as_ref(), tribe_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Already a member"));
    }
    
    // Check if request already exists
    let member_key = format!("{}:{}", tribe_id, info.sender);
    let member_status = MEMBER_STATUS.may_load(deps.storage, &member_key)?;
    
    if let Some(status) = member_status {
        match status {
            MemberStatus::PENDING => {
                return Err(cosmwasm_std::StdError::generic_err("Request already pending"));
            },
            MemberStatus::NONE => {
                return Err(cosmwasm_std::StdError::generic_err("Request was previously rejected"));
            },
            MemberStatus::BANNED => {
                return Err(cosmwasm_std::StdError::generic_err("User is banned from tribe"));
            },
            _ => { 
                // Continue with joining
            }
        }
    }
    
    // Create pending membership request
    let member = TribeMember {
        tribe_id,
        member: info.sender.clone(),
        status: MemberStatus::PENDING,
        joined_at: 0, // Will be set when approved
    };
    
    // Save to tribe members
    TRIBE_MEMBERS.save(deps.storage, (u64_to_key(tribe_id), &info.sender), &member)?;
    
    // Set member status
    MEMBER_STATUS.save(deps.storage, &member_key, &MemberStatus::PENDING)?;
    
    Ok(Response::new()
        .add_attribute("action", "request_to_join_tribe")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("member", info.sender.to_string()))
}

pub fn approveMember(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    member: String,
) -> StdResult<Response> {
    // Check if sender is tribe admin
    only_tribe_admin!(deps, info, tribe_id);
    
    let member_addr = deps.api.addr_validate(&member)?;
    
    // Get member status
    let member_status_key = format!("{}:{}", tribe_id, member);
    let member_status = MEMBER_STATUS.may_load(deps.storage, &member_status_key)?;
    
    if let Some(status) = member_status {
        if status == MemberStatus::BANNED {
            return Err(cosmwasm_std::StdError::generic_err("User is banned"));
        }
        
        if status != MemberStatus::PENDING {
            return Err(cosmwasm_std::StdError::generic_err("User is not pending"));
        }
    } else {
        return Err(cosmwasm_std::StdError::generic_err("User not found"));
    }
    
    // Update member status to active
    MEMBER_STATUS.save(deps.storage, &member_status_key, &MemberStatus::ACTIVE)?;
    
    Ok(Response::new()
        .add_attribute("action", "approve_member")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("member", member_addr.to_string()))
}

pub fn rejectMember(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    member: String,
) -> StdResult<Response> {
    // Check if sender is tribe admin
    only_tribe_admin!(deps, info, tribe_id);
    
    let member_addr = deps.api.addr_validate(&member)?;
    
    // Get member status
    let member_status_key = format!("{}:{}", tribe_id, member);
    let member_status = MEMBER_STATUS.may_load(deps.storage, &member_status_key)?;
    
    if let Some(status) = member_status {
        if status == MemberStatus::BANNED {
            return Err(cosmwasm_std::StdError::generic_err("User is banned"));
        }
        
        if status != MemberStatus::PENDING {
            return Err(cosmwasm_std::StdError::generic_err("User is not pending"));
        }
    } else {
        return Err(cosmwasm_std::StdError::generic_err("User not found"));
    }
    
    // Remove member
    MEMBER_STATUS.remove(deps.storage, &member_status_key);
    
    Ok(Response::new()
        .add_attribute("action", "reject_member")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("member", member_addr.to_string()))
}

pub fn banMember(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    member: String,
) -> StdResult<Response> {
    // Check if sender is tribe admin
    only_tribe_admin!(deps, info, tribe_id);
    
    let member_addr = deps.api.addr_validate(&member)?;
    
    // Get member status
    let member_status_key = format!("{}:{}", tribe_id, member);
    let member_status = MEMBER_STATUS.may_load(deps.storage, &member_status_key)?;
    
    if let Some(status) = member_status {
        if status == MemberStatus::BANNED {
            return Err(cosmwasm_std::StdError::generic_err("User is already banned"));
        }
    } else {
        return Err(cosmwasm_std::StdError::generic_err("User not found"));
    }
    
    // Update member status to banned
    MEMBER_STATUS.save(deps.storage, &member_status_key, &MemberStatus::BANNED)?;
    
    Ok(Response::new()
        .add_attribute("action", "ban_member")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("member", member_addr.to_string()))
}

pub fn joinTribeWithCode(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    invite_code: Vec<u8>,
) -> StdResult<Response> {
    // Validate tribe ID
    let config = get_config(deps.storage)?;
    if tribe_id >= config.next_tribe_id {
        return Err(cosmwasm_std::StdError::generic_err("Invalid tribe ID"));
    }
    
    // Check if already a member
    let is_member_key = format!("{}:{}", tribe_id, info.sender);
    if IS_MEMBER.may_load(deps.storage, &is_member_key)?.is_some() {
        return Err(cosmwasm_std::StdError::generic_err("Already a member"));
    }
    
    // Check if banned
    let member_status_key = format!("{}:{}", tribe_id, info.sender);
    if let Some(status) = MEMBER_STATUS.may_load(deps.storage, &member_status_key)? {
        if status == MemberStatus::BANNED {
            return Err(cosmwasm_std::StdError::generic_err("User is banned"));
        }
    }
    
    // Get tribe metadata
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    // Check if tribe is active
    if !tribe_meta.is_active {
        return Err(cosmwasm_std::StdError::generic_err("Tribe not active"));
    }
    
    // Check join type
    if tribe_meta.join_type != JoinType::INVITE_CODE {
        return Err(cosmwasm_std::StdError::generic_err("Tribe does not use invite codes"));
    }
    
    // Validate invite code
    let code_found = tribe_meta.whitelist.iter().any(|addr| {
        if let Ok(a) = deps.api.addr_validate(&String::from_utf8_lossy(&invite_code)) {
            return *addr == a;
        }
        false
    });
    
    if !code_found {
        return Err(cosmwasm_std::StdError::generic_err("Invalid invite code"));
    }
    
    // Add as active member
    MEMBER_STATUS.save(deps.storage, &member_status_key, &MemberStatus::ACTIVE)?;
    
    // Mark as member
    IS_MEMBER.save(deps.storage, &is_member_key, &true)?;
    
    // Increment member count
    let current_count = MEMBER_COUNT.may_load(deps.storage, &tribe_id.to_string())?.unwrap_or(0);
    MEMBER_COUNT.save(deps.storage, &tribe_id.to_string(), &(current_count + 1))?;
    
    // Update tribe whitelist
    let mut tribe_meta = tribe_meta;
    tribe_meta.whitelist.push(info.sender.clone());
    TRIBE_META.save(deps.storage, &tribe_id.to_string(), &tribe_meta)?;
    
    Ok(Response::new()
        .add_attribute("action", "join_tribe_with_code")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("member", info.sender.to_string()))
}

pub fn createInviteCode(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    code: String,
    max_uses: u64,
    expiry_time: u64,
) -> StdResult<Response> {
    // Check if sender is tribe admin
    only_tribe_admin!(deps, info, tribe_id);
    
    // Get tribe metadata
    let mut tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    // Create invite code struct
    let invite_code = InviteCode {
        code_hash: Sha256::digest(&code.as_bytes()).to_vec(),
        max_uses,
        used_count: 0,
        expiry_time,
    };

    // Save the invite code with tribe_id prefixed key
    let key = format!("{}:{}", tribe_id, hex::encode(&invite_code.code_hash));
    INVITE_CODE.save(deps.storage, &key, &invite_code)?;
    
    Ok(Response::new()
        .add_attribute("action", "create_invite_code")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("max_uses", max_uses.to_string())
        .add_attribute("expiry_time", expiry_time.to_string()))
}

pub fn requestMerge(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    source_tribe_id: u64,
    target_tribe_id: u64,
) -> StdResult<Response> {
    // Check if sender is source tribe admin
    only_tribe_admin!(deps, info, source_tribe_id);
    
    // Check if target tribe is active
    let target_tribe_meta = TRIBE_META.load(deps.storage, &target_tribe_id.to_string())?;
    if !target_tribe_meta.is_active {
        return Err(cosmwasm_std::StdError::generic_err("Target tribe not active"));
    }
    
    // Check if can merge
    if !target_tribe_meta.can_merge {
        return Err(cosmwasm_std::StdError::generic_err("Target tribe cannot merge"));
    }
    
    // Create merge request
    let current_time = _env.block.time.seconds();
    let merge_request = MergeRequest {
        source_tribe_id,
        target_tribe_id,
        request_time: current_time,
        approved: false,
    };
    
    // Save merge request
    let mut config = CONFIG.load(deps.storage)?;
    let merge_request_id = config.next_merge_request_id;
    config.next_merge_request_id += 1;
    CONFIG.save(deps.storage, &config)?;
    
    MERGE_REQUEST.save(deps.storage, &merge_request_id.to_string(), &merge_request)?;
    
    Ok(Response::new()
        .add_attribute("action", "request_merge")
        .add_attribute("merge_request_id", merge_request_id.to_string())
        .add_attribute("source_tribe_id", source_tribe_id.to_string())
        .add_attribute("target_tribe_id", target_tribe_id.to_string()))
}

pub fn approveMerge(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merge_request_id: u64,
) -> StdResult<Response> {
    // Get merge request
    let mut merge_request = MERGE_REQUEST.load(deps.storage, &merge_request_id.to_string())?;
    
    // Check if sender is target tribe admin
    only_tribe_admin!(deps, info, merge_request.target_tribe_id);
    
    // Update merge request to approved
    merge_request.approved = true;
    
    // Save updated merge request
    MERGE_REQUEST.save(deps.storage, &merge_request_id.to_string(), &merge_request)?;
    
    Ok(Response::new()
        .add_attribute("action", "approve_merge")
        .add_attribute("merge_request_id", merge_request_id.to_string())
        .add_attribute("source_tribe_id", merge_request.source_tribe_id.to_string())
        .add_attribute("target_tribe_id", merge_request.target_tribe_id.to_string()))
}

pub fn executeMerge(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merge_request_id: u64,
) -> StdResult<Response> {
    // Get merge request
    let merge_request = MERGE_REQUEST.load(deps.storage, &merge_request_id.to_string())?;
    
    // Validate merge request
    if !merge_request.approved {
        return Err(cosmwasm_std::StdError::generic_err("Merge request not approved"));
    }
    
    // Check if sender is source tribe admin
    only_tribe_admin!(deps, info, merge_request.source_tribe_id);
    
    // Get source and target tribe IDs
    let source_tribe_id = merge_request.source_tribe_id;
    let target_tribe_id = merge_request.target_tribe_id;
    
    // Update source tribe
    let mut source_tribe_meta = TRIBE_META.load(deps.storage, &source_tribe_id.to_string())?;
    source_tribe_meta.is_active = false;
    TRIBE_META.save(deps.storage, &source_tribe_id.to_string(), &source_tribe_meta)?;
    
    // Update target tribe
    let mut target_tribe_meta = TRIBE_META.load(deps.storage, &target_tribe_id.to_string())?;
    target_tribe_meta.is_active = true;
    TRIBE_META.save(deps.storage, &target_tribe_id.to_string(), &target_tribe_meta)?;
    
    // Remove merge request
    MERGE_REQUEST.remove(deps.storage, &merge_request_id.to_string());
    
    Ok(Response::new()
        .add_attribute("action", "execute_merge")
        .add_attribute("merge_request_id", merge_request_id.to_string())
        .add_attribute("source_tribe_id", source_tribe_id.to_string())
        .add_attribute("target_tribe_id", target_tribe_id.to_string()))
}

pub fn revokeInviteCode(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    code: String,
) -> StdResult<Response> {
    // Check if sender is tribe admin
    only_tribe_admin!(deps, info, tribe_id);
    
    // Get tribe metadata
    let mut tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    // Validate code
    let code_hash = Sha256::digest(code.as_bytes()).to_vec();
    
    // Remove from whitelist if applicable
    if let Some(addr) = deps.api.addr_validate(&code).ok() {
        tribe_meta.whitelist.retain(|a| a != &addr);
    }
    
    // Save updated tribe metadata
    TRIBE_META.save(deps.storage, &tribe_id.to_string(), &tribe_meta)?;
    
    Ok(Response::new()
        .add_attribute("action", "revoke_invite_code")
        .add_attribute("tribe_id", tribe_id.to_string()))
}

pub fn cancelMerge(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merge_request_id: u64,
) -> StdResult<Response> {
    // Get merge request
    let merge_request = MERGE_REQUEST.load(deps.storage, &merge_request_id.to_string())?;
    
    // Check if sender is source tribe admin
    only_tribe_admin!(deps, info, merge_request.source_tribe_id);
    
    // Remove merge request
    MERGE_REQUEST.remove(deps.storage, &merge_request_id.to_string());
    
    Ok(Response::new()
        .add_attribute("action", "cancel_merge")
        .add_attribute("merge_request_id", merge_request_id.to_string())
        .add_attribute("source_tribe_id", merge_request.source_tribe_id.to_string())
        .add_attribute("target_tribe_id", merge_request.target_tribe_id.to_string()))
}

pub fn getMemberStatus(deps: Deps, tribe_id: u64, member: String) -> StdResult<MemberStatusResponse> {
    let member_addr = deps.api.addr_validate(&member)?;
    let member_status_key = format!("{}:{}", tribe_id, member_addr);
    let status = MEMBER_STATUS.may_load(deps.storage, &member_status_key)?.unwrap_or(MemberStatus::NONE);
    Ok(MemberStatusResponse { status })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTribeAdmin { tribe_id } => to_json_binary(&getTribeAdmin(deps, tribe_id)?),
        QueryMsg::GetTribeWhitelist { tribe_id } => to_json_binary(&getTribeWhitelist(deps, tribe_id)?),
        QueryMsg::IsAddressWhitelisted { tribe_id, user } => to_json_binary(&isAddressWhitelisted(deps, tribe_id, user)?),
        QueryMsg::GetMemberStatus { tribe_id, member } => to_json_binary(&getMemberStatus(deps, tribe_id, member)?),
        QueryMsg::GetTribeConfigView { tribe_id } => to_json_binary(&getTribeConfigView(deps, tribe_id)?),
        QueryMsg::GetMemberCount { tribe_id } => to_json_binary(&getMemberCount(deps, tribe_id)?),
        QueryMsg::GetUserTribes { user } => to_json_binary(&getUserTribes(deps, user)?),
        QueryMsg::GetInviteCodeStatus { tribe_id, code } => to_json_binary(&getInviteCodeStatus(deps, tribe_id, code)?),
        QueryMsg::GetMergeRequest { request_id } => to_json_binary(&getMergeRequest(deps, request_id)?),
        QueryMsg::GetTribeDetails { tribe_id } => to_json_binary(&getTribeDetails(deps, tribe_id)?),
    }
}

pub fn getTribeAdmin(deps: Deps, tribe_id: u64) -> StdResult<AdminResponse> {
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    Ok(AdminResponse {
        admin: tribe_meta.admin.to_string(),
    })
}

pub fn getTribeWhitelist(deps: Deps, tribe_id: u64) -> StdResult<WhitelistResponse> {
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    let whitelist: Vec<String> = tribe_meta.whitelist.iter().map(|addr| addr.to_string()).collect();
    Ok(WhitelistResponse { whitelist })
}

pub fn isAddressWhitelisted(deps: Deps, tribe_id: u64, user: String) -> StdResult<BoolResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    let is_whitelisted = tribe_meta.whitelist.iter().any(|addr| *addr == user_addr);
    Ok(BoolResponse { result: is_whitelisted })
}

pub fn getTribeConfigView(deps: Deps, tribe_id: u64) -> StdResult<TribeConfigViewResponse> {
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    let config = TribeConfigView {
        join_type: tribe_meta.join_type,
        entry_fee: tribe_meta.entry_fee,
        nft_requirements: tribe_meta.nft_requirements,
        can_merge: tribe_meta.can_merge,
    };
    Ok(TribeConfigViewResponse { config })
}

pub fn getMemberCount(deps: Deps, tribe_id: u64) -> StdResult<MemberCountResponse> {
    let count = MEMBER_COUNT.may_load(deps.storage, &tribe_id.to_string())?.unwrap_or(0);
    Ok(MemberCountResponse { count })
}

pub fn getUserTribes(deps: Deps, user: String) -> StdResult<UserTribesResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let config = CONFIG.load(deps.storage)?;
    let mut tribe_ids = Vec::new();
    
    for tribe_id in 0..config.next_tribe_id {
        let is_member_key = format!("{}:{}", tribe_id, user_addr);
        if let Some(true) = IS_MEMBER.may_load(deps.storage, &is_member_key)? {
            tribe_ids.push(tribe_id);
        }
    }
    
    Ok(UserTribesResponse { tribe_ids })
}

pub fn getInviteCodeStatus(deps: Deps, tribe_id: u64, code: String) -> StdResult<InviteCodeStatusResponse> {
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    // Check if code is valid for this tribe
    let is_valid = if let Ok(addr) = deps.api.addr_validate(&code) {
        tribe_meta.whitelist.contains(&addr)
    } else {
        false
    };
    
    Ok(InviteCodeStatusResponse {
        valid: is_valid,
        remaining_uses: if is_valid { 1 } else { 0 }, // Simplified
    })
}

pub fn getMergeRequest(deps: Deps, request_id: u64) -> StdResult<MergeRequestResponse> {
    let request = MERGE_REQUEST.load(deps.storage, &request_id.to_string())?;
    Ok(MergeRequestResponse { request })
}

// Helper function to check if a user is whitelisted
pub fn is_whitelisted(deps: Deps, addr: &Addr) -> StdResult<bool> {
    Ok(WHITELIST.may_load(deps.storage, addr)?.unwrap_or(false))
}

// Helper function to check if a user is a tribe member with specific status
pub fn is_tribe_member_with_status(
    deps: Deps, 
    tribe_id: u64, 
    addr: &Addr, 
    status: MemberStatus
) -> StdResult<bool> {
    match TRIBE_MEMBERS.may_load(deps.storage, (u64_to_key(tribe_id), addr))? {
        Some(member) => Ok(member.status == status),
        None => Ok(false),
    }
}

// Helper function to check if a user is a tribe admin
pub fn is_tribe_admin_check(deps: Deps, tribe_id: u64, addr: &Addr) -> StdResult<bool> {
    is_tribe_member_with_status(deps, tribe_id, addr, MemberStatus::Admin)
}

// Helper function to check if a user is any type of tribe member
pub fn is_tribe_member(deps: Deps, tribe_id: u64, addr: &Addr) -> StdResult<bool> {
    Ok(TRIBE_MEMBERS.has(deps.storage, (u64_to_key(tribe_id), addr)))
}

fn is_tribe_admin_internal(
    deps: Deps,
    storage: &dyn Storage,
    tribe_id: u64,
    addr: &Addr,
) -> StdResult<bool> {
    let tribe_meta = TRIBE_META.load(storage, &tribe_id.to_string())?;
    
    if tribe_meta.admin == *addr {
        return Ok(true);
    }
    
    // Check if addr has MODERATOR_ROLE
    let config = get_config(storage)?;
    let msg = to_json_binary(&RoleManagerQuery::HasRole {
        user: addr.to_string(),
        role: "MODERATOR_ROLE".to_string(),
    })?;
    
    let query = cosmwasm_std::WasmQuery::Smart {
        contract_addr: config.role_manager.to_string(),
        msg,
    };
    
    match deps.querier.query::<RoleResponse>(&query.into()) {
        Ok(response) => Ok(response.has_role),
        Err(_) => Ok(false),
    }
}

// Add TribeDetailsView struct to match Solidity contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TribeDetailsView {
    pub name: String,
    pub metadata: String,
    pub admin: Addr,
    pub whitelist: Vec<Addr>,
    pub join_type: JoinType,
    pub entry_fee: Uint128,
    pub nft_requirements: Vec<NFTRequirement>,
    pub member_count: u64,
    pub can_merge: bool,
    pub is_active: bool,
    pub available_invite_codes: Vec<Vec<u8>>,
}

// Add getTribeDetails function to match Solidity function
pub fn getTribeDetails(deps: Deps, tribe_id: u64) -> StdResult<TribeDetailsView> {
    // Load tribe metadata
    let tribe_meta = TRIBE_META.load(deps.storage, &tribe_id.to_string())?;
    
    // Get member count
    let member_count = MEMBER_COUNT.may_load(deps.storage, &tribe_id.to_string())?.unwrap_or(0);
    
    // Mock empty invite codes - in a real implementation, you'd fetch actual invite codes
    let available_invite_codes = Vec::new();
    
    Ok(TribeDetailsView {
        name: tribe_meta.name,
        metadata: tribe_meta.metadata,
        admin: tribe_meta.admin,
        whitelist: tribe_meta.whitelist,
        join_type: tribe_meta.join_type,
        entry_fee: tribe_meta.entry_fee,
        nft_requirements: tribe_meta.nft_requirements,
        member_count,
        can_merge: tribe_meta.can_merge,
        is_active: tribe_meta.is_active,
        available_invite_codes,
    })
} 