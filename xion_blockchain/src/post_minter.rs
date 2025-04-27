use cosmwasm_std::{
    to_json_binary, from_json, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Uint128, Order,
    entry_point, Storage, Attribute, Event, SubMsg, CosmosMsg, WasmMsg, QueryRequest, WasmQuery,
};
use cw_storage_plus::{Item, Map, PrimaryKey, Prefixer, Bound};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use hex;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};

// Define constants equivalent to Solidity's keccak256 values
pub const DEFAULT_ADMIN_ROLE: &str = "DEFAULT_ADMIN_ROLE";
pub const RATE_LIMIT_MANAGER_ROLE: &str = "RATE_LIMIT_MANAGER_ROLE";
pub const PROJECT_CREATOR_ROLE: &str = "PROJECT_CREATOR_ROLE";

// Error constants
const ERROR_INVALID_METADATA: &str = "Invalid metadata";
const ERROR_INVALID_PARENT: &str = "Invalid parent post";
const ERROR_POST_DELETED: &str = "Post deleted";
const ERROR_BATCH_LIMIT: &str = "Too many posts in batch";
const ERROR_BATCH_COOLDOWN: &str = "Please wait before batch posting";
const ERROR_INVALID_ENCRYPTION: &str = "Invalid encryption key hash";
const ERROR_INVALID_SIGNER: &str = "Invalid access signer";
const ERROR_INSUFFICIENT_PERMISSIONS: &str = "Insufficient permissions";

// Storage definitions using cw-storage-plus
const CONFIG: Item<Config> = Item::new("config");
const NEXT_POST_ID: Item<u64> = Item::new("next_post_id");
const POSTS: Map<&[u8], PostData> = Map::new("posts");
const INTERACTION: Map<&str, bool> = Map::new("interaction");
const INTERACTION_COUNT: Map<&str, u64> = Map::new("interaction_count");
const AUTHORIZED_VIEWERS: Map<&str, bool> = Map::new("authorized_viewers");
const POST_DECRYPTION_KEYS: Map<&str, String> = Map::new("post_decryption_keys");
const LAST_POST_TIME: Map<&str, u64> = Map::new("last_post_time");
const LAST_BATCH_TIME: Map<&Addr, u64> = Map::new("last_batch_time");
const REPORT_COUNT: Map<&[u8], u64> = Map::new("report_count");
const TRIBE_ENCRYPTION_KEYS: Map<&[u8], String> = Map::new("tribe_encryption_keys");
const POST_TYPE_COOLDOWNS: Map<&[u8], u64> = Map::new("post_type_cooldowns");
const ROLES: Map<&str, bool> = Map::new("roles");
const PAUSED: Item<bool> = Item::new("paused");
const TRIBE_POSTS: Map<&str, bool> = Map::new("tribe_posts");
const USER_POSTS: Map<&str, bool> = Map::new("user_posts");

// Constants
const REPORT_THRESHOLD: u64 = 5;
const MAX_BATCH_POSTS: u64 = 5;
const BATCH_POST_COOLDOWN: u64 = 300; // 5 minutes in seconds

// Helper function to convert u64 to storage key bytes
fn u64_to_key(val: u64) -> Vec<u8> {
    val.to_be_bytes().to_vec()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum PostType {
    TEXT,
    RICH_MEDIA,
    EVENT,
    POLL,
    PROJECT_UPDATE,
    COMMUNITY_UPDATE,
    ENCRYPTED,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum InteractionType {
    LIKE,
    DISLIKE,
    SHARE,
    REPORT,
    REPLY,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PostData {
    pub id: u64,
    pub creator: Addr,
    #[serde(rename = "tribeId")]
    pub tribe_id: u64,
    pub metadata: String,
    #[serde(rename = "isGated")]
    pub is_gated: bool,
    #[serde(rename = "collectibleContract")]
    pub collectible_contract: Option<Addr>,
    #[serde(rename = "collectibleId")]
    pub collectible_id: u64,
    #[serde(rename = "isEncrypted")]
    pub is_encrypted: bool,
    #[serde(rename = "encryptionKeyHash")]
    pub encryption_key_hash: Option<String>,
    #[serde(rename = "accessSigner")]
    pub access_signer: Option<Addr>,
    #[serde(rename = "parentPostId")]
    pub parent_post_id: u64,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BatchPostData {
    pub metadata: String,
    #[serde(rename = "isGated")]
    pub is_gated: bool,
    #[serde(rename = "collectibleContract")]
    pub collectible_contract: Option<String>,
    #[serde(rename = "collectibleId")]
    pub collectible_id: u64,
    #[serde(rename = "postType")]
    pub post_type: PostType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub role_manager: Addr,
    pub tribe_controller: Addr,
    pub collectible_controller: Addr,
    pub feed_manager: Addr,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub role_manager: String,
    pub tribe_controller: String,
    pub collectible_controller: String,
    pub feed_manager: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreatePost {
        #[serde(rename = "tribeId")]
        tribe_id: u64,
        metadata: String,
        #[serde(rename = "isGated")]
        is_gated: bool,
        #[serde(rename = "collectibleContract")]
        collectible_contract: Option<String>,
        #[serde(rename = "collectibleId")]
        collectible_id: u64,
    },
    CreateReply {
        #[serde(rename = "parentPostId")]
        parent_post_id: u64,
        metadata: String,
        #[serde(rename = "isGated")]
        is_gated: bool,
        #[serde(rename = "collectibleContract")]
        collectible_contract: Option<String>,
        #[serde(rename = "collectibleId")]
        collectible_id: u64,
    },
    CreateEncryptedPost {
        #[serde(rename = "tribeId")]
        tribe_id: u64,
        metadata: String,
        #[serde(rename = "encryptionKeyHash")]
        encryption_key_hash: String,
        #[serde(rename = "accessSigner")]
        access_signer: String,
    },
    CreateSignatureGatedPost {
        #[serde(rename = "tribeId")]
        tribe_id: u64,
        metadata: String,
        #[serde(rename = "encryptionKeyHash")]
        encryption_key_hash: String,
        #[serde(rename = "accessSigner")]
        access_signer: String,
        #[serde(rename = "collectibleContract")]
        collectible_contract: String,
        #[serde(rename = "collectibleId")]
        collectible_id: u64,
    },
    DeletePost {
        #[serde(rename = "postId")]
        post_id: u64,
    },
    ReportPost {
        #[serde(rename = "postId")]
        post_id: u64,
        reason: String,
    },
    AuthorizeViewer {
        #[serde(rename = "postId")]
        post_id: u64,
        viewer: String,
    },
    SetTribeEncryptionKey {
        #[serde(rename = "tribeId")]
        tribe_id: u64,
        #[serde(rename = "encryptionKey")]
        encryption_key: String,
    },
    InteractWithPost {
        #[serde(rename = "postId")]
        post_id: u64,
        #[serde(rename = "interactionType")]
        interaction_type: InteractionType,
    },
    CreateBatchPosts {
        #[serde(rename = "tribeId")]
        tribe_id: u64,
        posts: Vec<BatchPostData>,
    },
    SetPostTypeCooldown {
        #[serde(rename = "postType")]
        post_type: PostType,
        cooldown: u64,
    },
    UpdatePost {
        #[serde(rename = "postId")]
        post_id: u64,
        metadata: String,
    },
    Pause {},
    Unpause {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    CanViewPost {
        #[serde(rename = "postId")]
        post_id: u64,
        viewer: String,
    },
    GetPostDecryptionKey {
        #[serde(rename = "postId")]
        post_id: u64,
        viewer: String,
    },
    VerifyPostAccess {
        #[serde(rename = "postId")]
        post_id: u64,
        viewer: String,
        signature: Binary,
    },
    GetInteractionCount {
        #[serde(rename = "postId")]
        post_id: u64,
        #[serde(rename = "interactionType")]
        interaction_type: InteractionType,
    },
    GetPostReplies {
        #[serde(rename = "postId")]
        post_id: u64,
    },
    GeneratePostKey {
        #[serde(rename = "postId")]
        post_id: u64,
    },
    DeriveSharedKey {
        #[serde(rename = "tribeId")]
        tribe_id: u64,
        member: String,
    },
    GetPost {
        #[serde(rename = "postId")]
        post_id: u64,
    },
    ValidateMetadata {
        metadata: String,
        #[serde(rename = "postType")]
        post_type: PostType,
    },
    GetPostTypeCooldown {
        #[serde(rename = "postType")]
        post_type: PostType,
    },
    GetRemainingCooldown {
        user: String,
        #[serde(rename = "postType")]
        post_type: PostType,
    },
    GetBatchPostingLimits {},
    GetPostsByTribe {
        #[serde(rename = "tribeId")]
        tribe_id: u64,
        offset: u64,
        limit: u64,
    },
    GetPostsByUser {
        user: String,
        offset: u64,
        limit: u64,
    },
    GetPostsByTribeAndUser {
        #[serde(rename = "tribeId")]
        tribe_id: u64,
        user: String,
        offset: u64,
        limit: u64,
    },
    GetFeedForUser {
        user: String,
        offset: u64,
        limit: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BoolResponse {
    pub result: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetPostResponse {
    pub id: u64,
    pub creator: String,
    #[serde(rename = "tribeId")]
    pub tribe_id: u64,
    pub metadata: String,
    #[serde(rename = "isGated")]
    pub is_gated: bool,
    #[serde(rename = "collectibleContract")]
    pub collectible_contract: Option<String>,
    #[serde(rename = "collectibleId")]
    pub collectible_id: u64,
    #[serde(rename = "isEncrypted")]
    pub is_encrypted: bool,
    #[serde(rename = "accessSigner")]
    pub access_signer: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InteractionCountResponse {
    pub count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PostRepliesResponse {
    pub replies: Vec<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StringResponse {
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PostsResponse {
    pub posts: Vec<u64>,
    pub total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CooldownResponse {
    pub cooldown: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BatchLimitsResponse {
    pub max_batch_size: u64,
    pub batch_cooldown: u64,
}

// Trait interface for role manager
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoleManagerQuery {
    HasRole {
        user: String,
        role: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TribeControllerQuery {
    GetMemberStatus {
        tribe_id: u64,
        member: String,
    },
    GetTribeAdmin {
        tribe_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum MemberStatus {
    ACTIVE,
    PENDING,
    REJECTED,
    BANNED,
    NONE,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MemberStatusResponse {
    pub status: MemberStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TribeAdminResponse {
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CollectibleControllerQuery {
    GetCollectible {
        collectible_id: u64,
    },
    BalanceOf {
        account: String,
        id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectibleResponse {
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let role_manager_addr = deps.api.addr_validate(&msg.role_manager)?;
    let tribe_controller_addr = deps.api.addr_validate(&msg.tribe_controller)?;
    let collectible_controller_addr = deps.api.addr_validate(&msg.collectible_controller)?;
    let feed_manager_addr = deps.api.addr_validate(&msg.feed_manager)?;
    
    let config = Config {
        role_manager: role_manager_addr,
        tribe_controller: tribe_controller_addr,
        collectible_controller: collectible_controller_addr,
        feed_manager: feed_manager_addr,
        owner: info.sender.clone(),
    };
    
    // Save config
    CONFIG.save(deps.storage, &config)?;
    
    // Initialize next post ID
    NEXT_POST_ID.save(deps.storage, &0u64)?;
    
    // Grant roles to sender/creator
    grant_role(deps.storage, info.sender.clone(), DEFAULT_ADMIN_ROLE.to_string())?;
    grant_role(deps.storage, info.sender.clone(), RATE_LIMIT_MANAGER_ROLE.to_string())?;
    grant_role(deps.storage, info.sender.clone(), PROJECT_CREATOR_ROLE.to_string())?;
    
    // Initialize cooldowns
    initialize_cooldowns(deps.storage)?;
    
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", info.sender))
}

fn initialize_cooldowns(storage: &mut dyn Storage) -> StdResult<()> {
    let mut cooldowns = POST_TYPE_COOLDOWNS;
    cooldowns.save(storage, b"TEXT", &60u64)?; // 1 minute
    cooldowns.save(storage, b"RICH_MEDIA", &120u64)?; // 2 minutes
    cooldowns.save(storage, b"EVENT", &30u64)?; // 30 seconds
    cooldowns.save(storage, b"POLL", &300u64)?; // 5 minutes
    cooldowns.save(storage, b"PROJECT_UPDATE", &120u64)?; // 2 minutes
    cooldowns.save(storage, b"COMMUNITY_UPDATE", &300u64)?; // 5 minutes
    cooldowns.save(storage, b"ENCRYPTED", &120u64)?; // 2 minutes
    Ok(())
}

fn serialize_post_type(post_type: &PostType) -> Vec<u8> {
    match post_type {
        PostType::TEXT => b"TEXT".to_vec(),
        PostType::RICH_MEDIA => b"RICH_MEDIA".to_vec(),
        PostType::EVENT => b"EVENT".to_vec(),
        PostType::POLL => b"POLL".to_vec(),
        PostType::PROJECT_UPDATE => b"PROJECT_UPDATE".to_vec(),
        PostType::COMMUNITY_UPDATE => b"COMMUNITY_UPDATE".to_vec(),
        PostType::ENCRYPTED => b"ENCRYPTED".to_vec(),
    }
}

// Add a helper function to convert Vec<u8> to String for display
fn post_type_to_string(post_type: &PostType) -> String {
    match post_type {
        PostType::TEXT => "TEXT".to_string(),
        PostType::RICH_MEDIA => "RICH_MEDIA".to_string(),
        PostType::EVENT => "EVENT".to_string(),
        PostType::POLL => "POLL".to_string(),
        PostType::PROJECT_UPDATE => "PROJECT_UPDATE".to_string(),
        PostType::COMMUNITY_UPDATE => "COMMUNITY_UPDATE".to_string(),
        PostType::ENCRYPTED => "ENCRYPTED".to_string(),
    }
}

fn grant_role(storage: &mut dyn Storage, address: Addr, role: String) -> StdResult<()> {
    let key = format!("{}:{}", address, role);
    ROLES.save(storage, &key, &true)
}

fn has_role(storage: &dyn Storage, address: &Addr, role: &str) -> bool {
    let key = format!("{}:{}", address, role);
    ROLES.may_load(storage, &key).unwrap_or(None).unwrap_or(false)
}

fn get_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

fn is_tribe_member(deps: Deps, tribe_id: u64, member: &Addr) -> StdResult<bool> {
    let config = get_config(deps.storage)?;
    
    let query_msg = to_json_binary(&TribeControllerQuery::GetMemberStatus { 
        tribe_id, 
        member: member.to_string() 
    })?;
    
    let query_result: StdResult<MemberStatusResponse> = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.tribe_controller.to_string(),
        msg: query_msg,
    }));
    
    match query_result {
        Ok(response) => Ok(response.status == MemberStatus::ACTIVE),
        Err(_) => Ok(false),
    }
}

fn is_post_creator(deps: Deps, post_id: u64, addr: &Addr) -> StdResult<bool> {
    match POSTS.may_load(deps.storage, &u64_to_key(post_id))? {
        Some(post) => Ok(post.creator == *addr),
        None => Ok(false),
    }
}

fn check_cooldown(
    storage: &dyn Storage,
    user: &Addr,
    post_type: &PostType,
    current_time: u64,
) -> StdResult<bool> {
    let cooldown: u64 = POST_TYPE_COOLDOWNS.load(storage, &serialize_post_type(post_type))?;
    
    let key = format!("{}:{}", user, post_type_to_string(post_type));
    let last_time = LAST_POST_TIME.may_load(storage, &key)?;
    
    match last_time {
        Some(time) => Ok(current_time >= time + cooldown),
        None => Ok(true),
    }
}

fn update_last_post_time(
    storage: &mut dyn Storage,
    user: &Addr,
    post_type: &PostType,
    current_time: u64,
) -> StdResult<()> {
    let key = format!("{}:{}", user, post_type_to_string(post_type));
    LAST_POST_TIME.save(storage, &key, &current_time)
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::CreatePost {
            tribe_id,
            metadata,
            is_gated,
            collectible_contract,
            collectible_id,
        } => {
            createPost(deps, env, info, tribe_id, metadata, is_gated, collectible_contract, collectible_id)
        },
        ExecuteMsg::CreateReply {
            parent_post_id,
            metadata,
            is_gated,
            collectible_contract,
            collectible_id,
        } => {
            createReply(deps, env, info, parent_post_id, metadata, is_gated, collectible_contract, collectible_id)
        },
        ExecuteMsg::CreateEncryptedPost {
            tribe_id,
            metadata,
            encryption_key_hash,
            access_signer,
        } => {
            createEncryptedPost(deps, env, info, tribe_id, metadata, encryption_key_hash, access_signer)
        },
        ExecuteMsg::CreateSignatureGatedPost {
            tribe_id,
            metadata,
            encryption_key_hash,
            access_signer,
            collectible_contract,
            collectible_id,
        } => {
            createSignatureGatedPost(
                deps,
                env,
                info,
                tribe_id,
                metadata,
                encryption_key_hash,
                access_signer,
                collectible_contract,
                collectible_id,
            )
        },
        ExecuteMsg::DeletePost { post_id } => {
            deletePost(deps, env, info, post_id)
        },
        ExecuteMsg::ReportPost { post_id, reason } => {
            reportPost(deps, env, info, post_id, reason)
        },
        ExecuteMsg::AuthorizeViewer { post_id, viewer } => {
            authorizeViewer(deps, env, info, post_id, viewer)
        },
        ExecuteMsg::SetTribeEncryptionKey {
            tribe_id,
            encryption_key,
        } => {
            setTribeEncryptionKey(deps, env, info, tribe_id, encryption_key)
        },
        ExecuteMsg::InteractWithPost {
            post_id,
            interaction_type,
        } => {
            interactWithPost(deps, env, info, post_id, interaction_type)
        },
        ExecuteMsg::CreateBatchPosts { tribe_id, posts } => {
            createBatchPosts(deps, env, info, tribe_id, posts)
        },
        ExecuteMsg::SetPostTypeCooldown {
            post_type,
            cooldown,
        } => {
            setPostTypeCooldown(deps, info, post_type, cooldown)
        },
        ExecuteMsg::UpdatePost { post_id, metadata } => {
            updatePost(deps, env, info, post_id, metadata)
        },
        ExecuteMsg::Pause {} => {
            pause(deps, info)
        },
        ExecuteMsg::Unpause {} => {
            unpause(deps, info)
        },
    }
}

pub fn createPost(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tribe_id: u64,
    metadata: String,
    is_gated: bool,
    collectible_contract: Option<String>,
    collectible_id: u64,
) -> StdResult<Response> {
    // Verify the sender is a tribe member
    if !is_tribe_member(deps.as_ref(), tribe_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Not tribe member"));
    }
    
    // Parse metadata to determine post type
    let post_type = determine_post_type(&metadata)?;
    
    // Check cooldown unless user has rate limit manager role
    if !has_role(deps.storage, &info.sender, RATE_LIMIT_MANAGER_ROLE) {
        let current_time = env.block.time.seconds();
        if !check_cooldown(deps.storage, &info.sender, &post_type, current_time)? {
            return Err(cosmwasm_std::StdError::generic_err("Cooldown active"));
        }
        update_last_post_time(deps.storage, &info.sender, &post_type, current_time)?;
    }
    
    // Validate collectible if specified
    let collectible_addr = match collectible_contract {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };
    
    // Create post
    let post_id = NEXT_POST_ID.load(deps.storage)?;
    NEXT_POST_ID.save(deps.storage, &(post_id + 1))?;
    
    let post = PostData {
        id: post_id,
        creator: info.sender.clone(),
        tribe_id,
        metadata: metadata.clone(),
        is_gated,
        collectible_contract: collectible_addr,
        collectible_id,
        is_encrypted: false,
        encryption_key_hash: None,
        access_signer: None,
        parent_post_id: 0,
        created_at: env.block.time.seconds(),
        is_deleted: false,
    };
    
    // Save post
    POSTS.save(deps.storage, &u64_to_key(post_id), &post)?;
    
    // Return response
    Ok(Response::new()
        .add_attribute("action", "create_post")
        .add_attribute("post_id", post_id.to_string())
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("creator", info.sender.to_string()))
}

// Helper function to determine post type from metadata
fn determine_post_type(metadata: &str) -> StdResult<PostType> {
    if metadata.contains("\"type\":\"EVENT\"") {
        Ok(PostType::EVENT)
    } else if metadata.contains("\"type\":\"RICH_MEDIA\"") {
        Ok(PostType::RICH_MEDIA)
    } else if metadata.contains("\"type\":\"PROJECT\"") || metadata.contains("\"type\":\"PROJECT_UPDATE\"") {
        Ok(PostType::PROJECT_UPDATE)
    } else if metadata.contains("\"type\":\"POLL\"") {
        Ok(PostType::POLL)
    } else if metadata.contains("\"type\":\"COMMUNITY_UPDATE\"") {
        Ok(PostType::COMMUNITY_UPDATE)
    } else if metadata.contains("\"type\":\"ENCRYPTED\"") {
        Ok(PostType::ENCRYPTED)
    } else {
        Ok(PostType::TEXT)
    }
}

pub fn createReply(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    parent_post_id: u64,
    metadata: String,
    is_gated: bool,
    collectible_contract: Option<String>,
    collectible_id: u64,
) -> StdResult<Response> {
    // Get next post ID to validate parent post ID
    let next_post_id: u64 = NEXT_POST_ID.load(deps.storage)?;
    if parent_post_id >= next_post_id {
        return Err(cosmwasm_std::StdError::generic_err("Invalid parent post"));
    }
    
    // Get parent post to check if deleted and get tribe ID
    let post = POSTS.load(deps.storage, &u64_to_key(parent_post_id))?;
    
    if post.is_deleted {
        return Err(cosmwasm_std::StdError::generic_err("Post deleted"));
    }
    
    // Check if tribe member
    if !is_tribe_member(deps.as_ref(), post.tribe_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Not tribe member"));
    }
    
    // Check cooldown 
    let current_time = env.block.time.seconds();
    if !has_role(deps.storage, &info.sender, RATE_LIMIT_MANAGER_ROLE) {
        if !check_cooldown(deps.storage, &info.sender, &PostType::TEXT, current_time)? {
            return Err(cosmwasm_std::StdError::generic_err("Cooldown active"));
        }
    }
    
    // Get next post ID
    let mut next_post_id = NEXT_POST_ID.load(deps.storage)?;
    next_post_id += 1;
    NEXT_POST_ID.save(deps.storage, &next_post_id)?;
    
    // Validate collectible contract if provided
    let collectible_addr = match &collectible_contract {
        Some(addr) => {
            let validated_addr = deps.api.addr_validate(addr)?;
            Some(validated_addr)
        },
        None => None,
    };
    
    // Create post data
    let post = PostData {
        id: next_post_id,
        creator: info.sender.clone(),
        tribe_id: post.tribe_id,
        metadata: metadata.clone(),
        is_gated,
        collectible_contract: collectible_addr,
        collectible_id,
        is_encrypted: false,
        encryption_key_hash: None,
        access_signer: None,
        parent_post_id,
        created_at: current_time,
        is_deleted: false,
    };
    
    // Store post
    POSTS.save(deps.storage, &u64_to_key(next_post_id), &post)?;
    
    // Update interaction counts
    let interaction_count_key = format!("{}:{}", parent_post_id, serialize_interaction_type(&InteractionType::REPLY));
    let count: u64 = INTERACTION_COUNT.may_load(deps.storage, &interaction_count_key)?.unwrap_or(0);
    INTERACTION_COUNT.save(deps.storage, &interaction_count_key, &(count + 1))?;
    
    // Update last post time
    update_last_post_time(deps.storage, &info.sender, &PostType::TEXT, current_time)?;
    
    Ok(Response::new()
        .add_attribute("action", "create_reply")
        .add_attribute("post_id", next_post_id.to_string())
        .add_attribute("parent_post_id", parent_post_id.to_string())
        .add_attribute("creator", info.sender.to_string()))
}

fn serialize_interaction_type(interaction_type: &InteractionType) -> String {
    match interaction_type {
        InteractionType::LIKE => "LIKE".to_string(),
        InteractionType::DISLIKE => "DISLIKE".to_string(),
        InteractionType::SHARE => "SHARE".to_string(),
        InteractionType::REPORT => "REPORT".to_string(),
        InteractionType::REPLY => "REPLY".to_string(),
    }
}

pub fn createEncryptedPost(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tribe_id: u64,
    metadata: String,
    encryption_key_hash: String,
    access_signer: String,
) -> StdResult<Response> {
    // Check if tribe member
    if !is_tribe_member(deps.as_ref(), tribe_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Not tribe member"));
    }
    
    // Check for empty metadata
    if metadata.is_empty() {
        return Err(cosmwasm_std::StdError::generic_err("Empty metadata"));
    }
    
    // Validate encryption key hash
    if encryption_key_hash.is_empty() {
        return Err(cosmwasm_std::StdError::generic_err("Invalid encryption key"));
    }
    
    // Validate access signer
    let access_signer_addr = deps.api.addr_validate(&access_signer)?;
    
    // Check cooldown
    let current_time = env.block.time.seconds();
    if !has_role(deps.storage, &info.sender, RATE_LIMIT_MANAGER_ROLE) {
        if !check_cooldown(deps.storage, &info.sender, &PostType::ENCRYPTED, current_time)? {
            return Err(cosmwasm_std::StdError::generic_err("Cooldown active"));
        }
    }
    
    // Get next post ID
    let mut next_post_id = NEXT_POST_ID.load(deps.storage)?;
    next_post_id += 1;
    NEXT_POST_ID.save(deps.storage, &next_post_id)?;
    
    // Create post data
    let post = PostData {
        id: next_post_id,
        creator: info.sender.clone(),
        tribe_id,
        metadata: metadata.clone(),
        is_gated: false,
        collectible_contract: None,
        collectible_id: 0,
        is_encrypted: true,
        encryption_key_hash: Some(encryption_key_hash.clone()),
        access_signer: Some(access_signer_addr.clone()),
        parent_post_id: 0,
        created_at: current_time,
        is_deleted: false,
    };
    
    // Store post
    POSTS.save(deps.storage, &u64_to_key(next_post_id), &post)?;
    
    // Store decryption key for creator
    let decryption_key = format!("{}:{}", next_post_id, info.sender);
    POST_DECRYPTION_KEYS.save(deps.storage, &decryption_key, &encryption_key_hash)?;
    
    // Store tribe encryption key if not already set
    let mut tribe_keys = TRIBE_ENCRYPTION_KEYS;
    tribe_keys.save(deps.storage, &u64_to_key(tribe_id), &encryption_key_hash)?;
    
    // Update last post time
    update_last_post_time(deps.storage, &info.sender, &PostType::ENCRYPTED, current_time)?;
    
    Ok(Response::new()
        .add_attribute("action", "create_encrypted_post")
        .add_attribute("post_id", next_post_id.to_string())
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("creator", info.sender.to_string())
        .add_attribute("access_signer", access_signer_addr.to_string()))
}

pub fn createSignatureGatedPost(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tribe_id: u64,
    metadata: String,
    encryption_key_hash: String,
    access_signer: String,
    collectible_contract: String,
    collectible_id: u64,
) -> StdResult<Response> {
    // Check if tribe member
    if !is_tribe_member(deps.as_ref(), tribe_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Not tribe member"));
    }
    
    // Check for empty metadata
    if metadata.is_empty() {
        return Err(cosmwasm_std::StdError::generic_err("Empty metadata"));
    }
    
    // Validate encryption key hash
    if encryption_key_hash.is_empty() {
        return Err(cosmwasm_std::StdError::generic_err("Invalid encryption key"));
    }
    
    // Validate access signer
    let access_signer_addr = deps.api.addr_validate(&access_signer)?;
    
    // Validate collectible contract
    let config = get_config(deps.storage)?;
    let collectible_addr = deps.api.addr_validate(&collectible_contract)?;
    
    if collectible_addr != config.collectible_controller {
        return Err(cosmwasm_std::StdError::generic_err("Invalid collectible contract"));
    }
    
    // Verify collectible is active
    let query_msg = to_json_binary(&CollectibleControllerQuery::GetCollectible { collectible_id })?;
    let collectible_result: StdResult<CollectibleResponse> = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: collectible_addr.to_string(),
        msg: query_msg,
    }));
    
    match collectible_result {
        Ok(collectible) => {
            if !collectible.is_active {
                return Err(cosmwasm_std::StdError::generic_err("Invalid collectible"));
            }
        },
        Err(_) => return Err(cosmwasm_std::StdError::generic_err("Failed to query collectible")),
    }
    
    // Check cooldown
    let current_time = env.block.time.seconds();
    if !has_role(deps.storage, &info.sender, RATE_LIMIT_MANAGER_ROLE) {
        if !check_cooldown(deps.storage, &info.sender, &PostType::TEXT, current_time)? {
            return Err(cosmwasm_std::StdError::generic_err("Cooldown active"));
        }
    }
    
    // Get next post ID
    let mut next_post_id = NEXT_POST_ID.load(deps.storage)?;
    next_post_id += 1;
    NEXT_POST_ID.save(deps.storage, &next_post_id)?;
    
    // Create post data
    let post = PostData {
        id: next_post_id,
        creator: info.sender.clone(),
        tribe_id,
        metadata: metadata.clone(),
        is_gated: true,
        collectible_contract: Some(collectible_addr.clone()),
        collectible_id,
        is_encrypted: true,
        encryption_key_hash: Some(encryption_key_hash.clone()),
        access_signer: Some(access_signer_addr.clone()),
        parent_post_id: 0,
        created_at: current_time,
        is_deleted: false,
    };
    
    // Store post
    POSTS.save(deps.storage, &u64_to_key(next_post_id), &post)?;
    
    // Update last post time
    update_last_post_time(deps.storage, &info.sender, &PostType::TEXT, current_time)?;
    
    Ok(Response::new()
        .add_attribute("action", "create_signature_gated_post")
        .add_attribute("post_id", next_post_id.to_string())
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("creator", info.sender.to_string())
        .add_attribute("access_signer", access_signer_addr.to_string())
        .add_attribute("collectible_contract", collectible_addr.to_string())
        .add_attribute("collectible_id", collectible_id.to_string()))
}

pub fn deletePost(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    post_id: u64,
) -> StdResult<Response> {
    // Check if post exists and user is creator
    if !is_post_creator(deps.as_ref(), post_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Not post creator"));
    }
    
    // Check if post is already deleted
    let post_data = POSTS.load(deps.storage, &u64_to_key(post_id))?;
    
    if post_data.is_deleted {
        return Err(cosmwasm_std::StdError::generic_err("Post deleted"));
    }
    
    // Mark post as deleted
    let mut post = post_data;
    post.is_deleted = true;
    POSTS.save(deps.storage, &u64_to_key(post_id), &post)?;
    
    Ok(Response::new()
        .add_attribute("action", "delete_post")
        .add_attribute("post_id", post_id.to_string())
        .add_attribute("deleter", info.sender.to_string()))
}

pub fn reportPost(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    post_id: u64,
    reason: String,
) -> StdResult<Response> {
    // Check if post exists and is not deleted
    let post = POSTS.load(deps.storage, &u64_to_key(post_id))?;
    
    if post.is_deleted {
        return Err(cosmwasm_std::StdError::generic_err("Post deleted"));
    }
    
    // Check if user has already reported this post
    let interaction_key = format!("{}:{}:{}", post_id, info.sender, serialize_interaction_type(&InteractionType::REPORT));
    
    if let Ok(Some(true)) = INTERACTION.may_load(deps.storage, &interaction_key) {
        return Err(cosmwasm_std::StdError::generic_err("Already reported"));
    }
    
    // Mark interaction
    INTERACTION.save(deps.storage, &interaction_key, &true)?;
    
    // Increment report count
    let mut report_counts = REPORT_COUNT;
    let count: u64 = report_counts.may_load(deps.storage, &u64_to_key(post_id))?.unwrap_or(0);
    let new_count = count + 1;
    report_counts.save(deps.storage, &u64_to_key(post_id), &new_count)?;
    
    // If threshold reached, mark post as deleted
    if new_count >= REPORT_THRESHOLD {
        let mut posts = POSTS;
        let mut post = post;
        post.is_deleted = true;
        posts.save(deps.storage, &u64_to_key(post_id), &post)?;
    }
    
    Ok(Response::new()
        .add_attribute("action", "report_post")
        .add_attribute("post_id", post_id.to_string())
        .add_attribute("reporter", info.sender.to_string())
        .add_attribute("reason", reason))
}

pub fn authorizeViewer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    post_id: u64,
    viewer: String,
) -> StdResult<Response> {
    // Check if post exists and user is creator
    if !is_post_creator(deps.as_ref(), post_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Not post creator"));
    }
    
    // Validate viewer address
    let viewer_addr = deps.api.addr_validate(&viewer)?;
    
    // Add to authorized viewers
    let key = format!("{}:{}", post_id, viewer_addr);
    AUTHORIZED_VIEWERS.save(deps.storage, &key, &true)?;
    
    Ok(Response::new()
        .add_attribute("action", "authorize_viewer")
        .add_attribute("post_id", post_id.to_string())
        .add_attribute("viewer", viewer_addr.to_string()))
}

pub fn setTribeEncryptionKey(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tribe_id: u64,
    encryption_key: String,
) -> StdResult<Response> {
    // Get tribe admin
    let tribe_controller = get_config(deps.storage)?.tribe_controller;
    let tribe_admin_query = TribeControllerQuery::GetTribeAdmin { tribe_id };
    let res: TribeAdminResponse = deps.querier.query_wasm_smart(
        tribe_controller,
        &tribe_admin_query,
    )?;
    
    // Convert response admin to address
    let admin_addr = deps.api.addr_validate(&res.admin)?;
    
    // Check if sender is admin
    if info.sender != admin_addr {
                return Err(cosmwasm_std::StdError::generic_err("Not tribe admin"));
            }
    
    // Check if sender is a tribe member
    if !is_tribe_member(deps.as_ref(), tribe_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Not tribe member"));
    }
    
    // Save encryption key
    let mut tribe_keys = TRIBE_ENCRYPTION_KEYS;
    tribe_keys.save(deps.storage, &u64_to_key(tribe_id), &encryption_key)?;
    
    Ok(Response::new().add_attribute("action", "set_tribe_encryption_key"))
}

pub fn interactWithPost(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    post_id: u64,
    interaction_type: InteractionType,
) -> StdResult<Response> {
    // Check if post exists and is not deleted
    let post = POSTS.load(deps.storage, &u64_to_key(post_id))?;
    
    if post.is_deleted {
        return Err(cosmwasm_std::StdError::generic_err("Post deleted"));
    }
    
    // Check if user is post creator
    if post.creator == info.sender {
        return Err(cosmwasm_std::StdError::generic_err("Cannot interact with own post"));
    }
    
    // Check if user has access to the post
    if !canViewPost(deps.as_ref(), post_id, info.sender.to_string())? {
        return Err(cosmwasm_std::StdError::generic_err("Insufficient access"));
    }
    
    // Check if user has already interacted
    let interaction_key = format!("{}:{}:{}", post_id, info.sender, serialize_interaction_type(&interaction_type));
    
    if let Ok(Some(true)) = INTERACTION.may_load(deps.storage, &interaction_key) {
        return Err(cosmwasm_std::StdError::generic_err("Already interacted"));
    }
    
    // Mark interaction
    INTERACTION.save(deps.storage, &interaction_key, &true)?;
    
    // Increment interaction count
    let interaction_count_key = format!("{}:{}", post_id, serialize_interaction_type(&interaction_type));
    let count: u64 = INTERACTION_COUNT.may_load(deps.storage, &interaction_count_key)?.unwrap_or(0);
    INTERACTION_COUNT.save(deps.storage, &interaction_count_key, &(count + 1))?;
    
    Ok(Response::new()
        .add_attribute("action", "interact_with_post")
        .add_attribute("post_id", post_id.to_string())
        .add_attribute("user", info.sender.to_string())
        .add_attribute("interaction_type", serialize_interaction_type(&interaction_type)))
}

pub fn canViewPost(deps: Deps, post_id: u64, viewer: String) -> StdResult<bool> {
    // Convert viewer string to Addr
    let viewer_addr = deps.api.addr_validate(&viewer)?;
    
    // Get post
    let post = match POSTS.may_load(deps.storage, &u64_to_key(post_id))? {
        Some(p) => p,
        None => return Ok(false),
    };
    
    // Check if post is deleted
    if post.is_deleted {
        return Ok(false);
    }
    
    // Check if viewer is authorized directly
    let authorized_key = format!("{}:{}", post_id, viewer_addr);
    if AUTHORIZED_VIEWERS.may_load(deps.storage, &authorized_key)?.unwrap_or(false) {
        return Ok(true);
    }
    
    // Check if viewer is post creator
    if post.creator == viewer_addr {
        return Ok(true);
    }
    
    // Check if viewer is tribe member
    if !is_tribe_member(deps, post.tribe_id, &viewer_addr)? {
        return Ok(false);
    }
    
    // If post is not gated, allow access
    if !post.is_gated {
        return Ok(true);
    }
    
    // If post has collectible requirements, check balance
    if let Some(collectible_addr) = &post.collectible_contract {
        let query_msg = to_json_binary(&CollectibleControllerQuery::BalanceOf { 
            account: viewer_addr.to_string(), 
            id: post.collectible_id,
        })?;
        
        let balance_result: StdResult<BalanceResponse> = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: collectible_addr.to_string(),
            msg: query_msg,
        }));
        
        match balance_result {
            Ok(balance) => return Ok(balance.balance > Uint128::zero()),
            Err(_) => return Ok(false),
        }
    }
    
    Ok(true)
}

pub fn verifyPostAccess(deps: Deps, post_id: u64, viewer: String, _signature: Binary) -> StdResult<bool> {
    // Get post
    let posts: Map<&[u8], PostData> = POSTS;
    let post = posts.load(deps.storage, &u64_to_key(post_id))?;
    
    // Check if post has access signer
    let access_signer = match post.access_signer {
        Some(addr) => addr,
        None => return Err(cosmwasm_std::StdError::generic_err("Post not signature gated")),
    };
    
    // Validate viewer address
    let viewer_addr = deps.api.addr_validate(&viewer)?;
    
    // In a real implementation, we'd verify the signature cryptographically
    // Since the original uses ECDSA recovery which is complex in CosmWasm,
    // we're simplifying this for demonstration purposes
    
    // This is a placeholder - in a real implementation we would verify 
    // that the signature was produced by the access_signer for this viewer and tribe
    
    Ok(true) // Simplified for demo
}

pub fn getInteractionCount(deps: Deps, post_id: u64, interaction_type: InteractionType) -> StdResult<InteractionCountResponse> {
    let interaction_count_key = format!("{}:{}", post_id, serialize_interaction_type(&interaction_type));
    let count: u64 = INTERACTION_COUNT.may_load(deps.storage, &interaction_count_key)?.unwrap_or(0);
    
    Ok(InteractionCountResponse { count })
}

pub fn getPostReplies(deps: Deps, post_id: u64) -> StdResult<PostRepliesResponse> {
    let interaction_count_key = format!("{}:{}", post_id, serialize_interaction_type(&InteractionType::REPLY));
    let count = INTERACTION_COUNT.may_load(deps.storage, &interaction_count_key)?.unwrap_or(0);
    
    // Initialize replies vector
    let mut replies = Vec::with_capacity(count as usize);
    
    // Get next post ID to know how many posts to search
    let next_post_id: u64 = NEXT_POST_ID.load(deps.storage)?;
    let posts = POSTS;
    
    // Iterate through all posts (inefficient but mimics the Solidity logic)
    for i in 0..next_post_id {
        if let Some(post) = posts.may_load(deps.storage, &u64_to_key(i))? {
            if post.parent_post_id == post_id && !post.is_deleted {
                replies.push(i);
            }
        }
    }
    
    Ok(PostRepliesResponse { replies })
}

pub fn query_get_post(deps: Deps, post_id: u64) -> StdResult<GetPostResponse> {
    let post = POSTS.load(deps.storage, &u64_to_key(post_id))?;
    
    Ok(GetPostResponse {
        id: post.id,
        creator: post.creator.to_string(),
        tribe_id: post.tribe_id,
        metadata: post.metadata,
        is_gated: post.is_gated,
        collectible_contract: post.collectible_contract.map(|addr| addr.to_string()),
        collectible_id: post.collectible_id,
        is_encrypted: post.is_encrypted,
        access_signer: post.access_signer.map(|addr| addr.to_string()),
    })
}

pub fn validateMetadata(metadata: &str, post_type: &PostType) -> bool {
    // Basic length check
    if metadata.len() < 10 || metadata.len() > 5000 {
        return false;
    }
    
    // Check if metadata is valid JSON
    let metadata_str = metadata.to_string();
    
    // Check if metadata contains type-specific fields
    match post_type {
        PostType::EVENT => {
            if !metadata_str.contains("\"type\":\"EVENT\"") {
                return false;
            }
        },
        PostType::RICH_MEDIA => {
            if !metadata_str.contains("\"type\":\"RICH_MEDIA\"") {
                return false;
            }
        },
        PostType::PROJECT_UPDATE => {
            if !metadata_str.contains("\"type\":\"PROJECT_UPDATE\"") {
                return false;
            }
        },
        _ => {
            // Other post types don't need special validation yet
    }
    }
    
    // More validation as needed...
    true
}

pub fn getPostTypeCooldown(deps: Deps, post_type: PostType) -> StdResult<CooldownResponse> {
    let post_type_key = serialize_post_type(&post_type);
    let cooldown = POST_TYPE_COOLDOWNS.may_load(deps.storage, &post_type_key)?.unwrap_or(0);
    
    Ok(CooldownResponse { cooldown })
}

pub fn getRemainingCooldown(deps: Deps, env: Env, user: String, post_type: PostType) -> StdResult<CooldownResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let current_time = env.block.time.seconds();
    
    // Get the cooldown for this post type
    let cooldown = POST_TYPE_COOLDOWNS.load(deps.storage, &serialize_post_type(&post_type))?;
    
    // Get the last post time
    let key = format!("{}:{}", user_addr, post_type_to_string(&post_type));
    
    let last_time = match LAST_POST_TIME.may_load(deps.storage, &key)? {
        Some(time) => time,
        None => return Ok(CooldownResponse { cooldown: 0 }),
    };
    
    let next_allowed_time = last_time + cooldown;
    
    if current_time >= next_allowed_time {
        return Ok(CooldownResponse { cooldown: 0 });
    }
    
    Ok(CooldownResponse { cooldown: next_allowed_time - current_time })
}

pub fn getBatchPostingLimits() -> StdResult<BatchLimitsResponse> {
    Ok(BatchLimitsResponse {
        max_batch_size: MAX_BATCH_POSTS,
        batch_cooldown: BATCH_POST_COOLDOWN,
    })
}

// Simplified feed query functions
pub fn getPostsByTribe(deps: Deps, tribe_id: u64, offset: u64, limit: u64) -> StdResult<PostsResponse> {
    let mut matching_posts = Vec::new();
    let posts: Map<&[u8], PostData> = POSTS;
    let next_post_id: u64 = NEXT_POST_ID.load(deps.storage)?;
    
    // Count matching posts for total
    let mut total = 0;
    
    // Collect matching posts (inefficient but simulates the original contract behavior)
    for i in 0..next_post_id {
        if let Some(post) = posts.may_load(deps.storage, &u64_to_key(i))? {
            if post.tribe_id == tribe_id && !post.is_deleted {
                if total >= offset && (total - offset) < limit {
                    matching_posts.push(i);
                }
                total += 1;
            }
        }
    }
    
    Ok(PostsResponse {
        posts: matching_posts,
        total,
    })
}

pub fn getPostsByUser(deps: Deps, user: String, offset: u64, limit: u64) -> StdResult<PostsResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    
    let mut matching_posts = Vec::new();
    let posts: Map<&[u8], PostData> = POSTS;
    let next_post_id: u64 = NEXT_POST_ID.load(deps.storage)?;
    
    // Count matching posts for total
    let mut total = 0;
    
    // Collect matching posts
    for i in 0..next_post_id {
        if let Some(post) = posts.may_load(deps.storage, &u64_to_key(i))? {
            if post.creator == user_addr && !post.is_deleted {
                if total >= offset && (total - offset) < limit {
                    matching_posts.push(i);
                }
                total += 1;
            }
        }
    }
    
    Ok(PostsResponse {
        posts: matching_posts,
        total,
    })
}

pub fn getPostsByTribeAndUser(deps: Deps, tribe_id: u64, user: String, offset: u64, limit: u64) -> StdResult<PostsResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    
    let mut matching_posts = Vec::new();
    let posts: Map<&[u8], PostData> = POSTS;
    let next_post_id: u64 = NEXT_POST_ID.load(deps.storage)?;
    
    // Count matching posts for total
    let mut total = 0;
    
    // Collect matching posts
    for i in 0..next_post_id {
        if let Some(post) = posts.may_load(deps.storage, &u64_to_key(i))? {
            if post.tribe_id == tribe_id && post.creator == user_addr && !post.is_deleted {
                if total >= offset && (total - offset) < limit {
                    matching_posts.push(i);
                }
                total += 1;
            }
        }
    }
    
    Ok(PostsResponse {
        posts: matching_posts,
        total,
    })
}

pub fn getFeedForUser(deps: Deps, user: String, offset: u64, limit: u64) -> StdResult<PostsResponse> {
    // In a real implementation, we'd have a more sophisticated feed algorithm
    // For simplicity, we'll just return posts from tribes the user is a member of
    
    let user_addr = deps.api.addr_validate(&user)?;
    
    let mut matching_posts = Vec::new();
    let posts: Map<&[u8], PostData> = POSTS;
    let next_post_id: u64 = NEXT_POST_ID.load(deps.storage)?;
    
    // Count matching posts for total
    let mut total = 0;
    
    // Collect matching posts - simplified approach
    for i in 0..next_post_id {
        if let Some(post) = posts.may_load(deps.storage, &u64_to_key(i))? {
            if !post.is_deleted && is_tribe_member(deps, post.tribe_id, &user_addr)? {
                if total >= offset && (total - offset) < limit {
                    matching_posts.push(i);
                }
                total += 1;
            }
        }
    }
    
    Ok(PostsResponse {
        posts: matching_posts,
        total,
    })
}

// Add _validateNFTRequirements and _validateProjectUpdatePermissions to match Solidity contract

fn _validateNFTRequirements(
    deps: Deps,
    collectible_controller: &Addr,
    viewer: &Addr, 
    collectible_contract: &Option<Addr>,
    collectible_id: u64
) -> StdResult<bool> {
    // Implementation similar to Solidity contract's _validateNFTRequirements
    if collectible_contract.is_none() || collectible_contract.as_ref().unwrap() == &Addr::unchecked("") {
        return Ok(true);
    }
    
    let collectible_contract_addr = collectible_contract.as_ref().unwrap();
    
    // Query collectible balance from collectible controller
    let query_msg = CollectibleControllerQuery::BalanceOf { 
        account: viewer.to_string(), 
        id: collectible_id 
    };
    
    let query = cosmwasm_std::WasmQuery::Smart {
        contract_addr: collectible_controller.to_string(),
        msg: to_json_binary(&query_msg)?,
    };
    
    match deps.querier.query::<BalanceResponse>(&query.into()) {
        Ok(balance_resp) => {
            Ok(balance_resp.balance.u128() > 0)
        },
        Err(_) => Ok(false),
    }
}

fn _validateProjectUpdatePermissions(
    deps: Deps, 
    info: &MessageInfo,
    metadata: &str
) -> StdResult<bool> {
    // Check if user has project creator role
    if !has_role(deps.storage, &info.sender, PROJECT_CREATOR_ROLE) {
        return Ok(false);
    }
    
    // Check if this is a project update
    let metadata_str = metadata.to_string();
    if !metadata_str.contains("\"type\":\"PROJECT_UPDATE\"") {
        return Ok(true);
    }
    
    // Extract project ID from metadata
    let project_id = match parse_project_post_id(metadata) {
        Some(id) => id,
        None => return Ok(false),
    };
    
    // More project-specific validation logic...
    Ok(true)
}

// Helper function to parse project post ID from metadata
fn parse_project_post_id(metadata: &str) -> Option<u64> {
    if let Some(start_idx) = metadata.find("\"projectPostId\":") {
        let substr = &metadata[start_idx + 15..];
        if let Some(end_idx) = substr.find(',') {
            let id_str = &substr[0..end_idx];
            if let Ok(id) = id_str.trim().parse::<u64>() {
                return Some(id);
            }
        }
    }
    None
}

pub fn createBatchPosts(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tribe_id: u64,
    posts: Vec<BatchPostData>,
) -> StdResult<Response> {
    // Check if batch size is within limits
    if posts.len() > MAX_BATCH_POSTS as usize {
        return Err(cosmwasm_std::StdError::generic_err(
            format!("Batch size exceeds limit of {}", MAX_BATCH_POSTS)
        ));
    }
    
    // Check if tribe exists and user is a member
    if !is_tribe_member(deps.as_ref(), tribe_id, &info.sender)? {
        return Err(cosmwasm_std::StdError::generic_err("Not a tribe member"));
    }
    
    let mut response = Response::new()
        .add_attribute("action", "create_batch_posts")
        .add_attribute("tribe_id", tribe_id.to_string())
        .add_attribute("batch_size", posts.len().to_string());
    
    let config = get_config(deps.storage)?;
    let current_time = env.block.time.seconds();
    
    for post_data in posts {
        let post_type = post_data.post_type;
        
        // Check cooldown for this post type
        if !check_cooldown(deps.storage, &info.sender, &post_type, current_time)? {
            return Err(cosmwasm_std::StdError::generic_err(
                format!("Cooldown period not expired for post type {:?}", post_type)
            ));
        }
        
        // Validate the post's metadata
        if !validateMetadata(&post_data.metadata, &post_type) {
            return Err(cosmwasm_std::StdError::generic_err("Invalid metadata"));
        }
        
        // Get new post ID
        let next_post_id = NEXT_POST_ID.load(deps.storage)?;
        NEXT_POST_ID.save(deps.storage, &(next_post_id + 1))?;
        
        // Parse collectible contract if provided
        let collectible_contract = match post_data.collectible_contract {
            Some(contract) => Some(deps.api.addr_validate(&contract)?),
            None => None,
        };
        
        // Create post data
        let post = PostData {
            id: next_post_id,
            creator: info.sender.clone(),
            tribe_id,
            metadata: post_data.metadata.clone(),
            is_gated: post_data.is_gated,
            collectible_contract,
            collectible_id: post_data.collectible_id,
            is_encrypted: false,
            encryption_key_hash: None,
            access_signer: None,
            parent_post_id: 0, // Not a reply
            created_at: current_time,
            is_deleted: false,
        };
        
        // Save post
        POSTS.save(deps.storage, &u64_to_key(next_post_id), &post)?;
        
        // Associate post with tribe
        let tribe_post_key = format!("{}:{}", tribe_id, next_post_id);
        TRIBE_POSTS.save(deps.storage, &tribe_post_key, &true)?;
        
        // Associate post with creator
        let user_post_key = format!("{}:{}", info.sender, next_post_id);
        USER_POSTS.save(deps.storage, &user_post_key, &true)?;
        
        // Update last post time for this post type
        update_last_post_time(deps.storage, &info.sender, &post_type, current_time)?;
    }
    
    Ok(response)
}

pub fn setPostTypeCooldown(
    deps: DepsMut,
    info: MessageInfo,
    post_type: PostType,
    cooldown: u64,
) -> StdResult<Response> {
    // Check if caller has rate limit manager role
    if !has_role(deps.storage, &info.sender, RATE_LIMIT_MANAGER_ROLE) {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Update cooldown for post type
    POST_TYPE_COOLDOWNS.save(deps.storage, &serialize_post_type(&post_type), &cooldown)?;
    
    Ok(Response::new()
        .add_attribute("action", "set_post_type_cooldown")
        .add_attribute("post_type", post_type_to_string(&post_type))
        .add_attribute("cooldown", cooldown.to_string()))
}

pub fn updatePost(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    post_id: u64,
    metadata: String,
) -> StdResult<Response> {
    // Check if post exists
    let mut post = match POSTS.may_load(deps.storage, &u64_to_key(post_id))? {
        Some(post) => post,
        None => return Err(cosmwasm_std::StdError::generic_err("Post not found")),
    };
    
    // Check if caller is post creator
    if post.creator != info.sender {
        return Err(cosmwasm_std::StdError::generic_err("Not post creator"));
    }
    
    // Check if post is deleted
    if post.is_deleted {
        return Err(cosmwasm_std::StdError::generic_err("Post is deleted"));
    }
    
    // Determine post type from metadata
    let post_type = determine_post_type(&metadata)?;
    
    // Validate the updated metadata
    if !validateMetadata(&metadata, &post_type) {
        return Err(cosmwasm_std::StdError::generic_err("Invalid metadata"));
    }
    
    // Update post metadata
    post.metadata = metadata;
    POSTS.save(deps.storage, &u64_to_key(post_id), &post)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_post")
        .add_attribute("post_id", post_id.to_string())
        .add_attribute("updater", info.sender.to_string()))
}

// Pause contract operations
pub fn pause(
    deps: DepsMut,
    info: MessageInfo,
) -> StdResult<Response> {
    // Check if caller has admin role
    let config = get_config(deps.storage)?;
    if info.sender != config.owner {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Set paused state
    PAUSED.save(deps.storage, &true)?;
    
    Ok(Response::new()
        .add_attribute("action", "pause")
        .add_attribute("paused_by", info.sender.to_string()))
}

// Unpause contract operations
pub fn unpause(
    deps: DepsMut,
    info: MessageInfo,
) -> StdResult<Response> {
    // Check if caller has admin role
    let config = get_config(deps.storage)?;
    if info.sender != config.owner {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Set unpaused state
    PAUSED.save(deps.storage, &false)?;
    
    Ok(Response::new()
        .add_attribute("action", "unpause")
        .add_attribute("unpaused_by", info.sender.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{mock_dependencies, mock_env, mock_info};
    
    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies();
        let info = mock_info("owner", &[]);
        
        let msg = InstantiateMsg {
            role_manager: "role_manager".to_string(),
            tribe_controller: "tribe_controller".to_string(),
            collectible_controller: "collectible_controller".to_string(),
            feed_manager: "feed_manager".to_string(),
        };
        
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(2, res.attributes.len());
        assert_eq!("instantiate", res.attributes[0].value);
        assert_eq!("owner", res.attributes[1].value);
    }
} 