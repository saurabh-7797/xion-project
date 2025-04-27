// Export module declarations
pub mod errors;
pub mod role_manager;
pub mod tribe_controller;
pub mod profile_nft_minter;
pub mod post_minter;
pub mod testing;

// Re-export errors for convenience
pub use errors::ContractError;

// Utility function for creating contract version string
pub fn create_contract_version(name: &str, version: &str) -> String {
    format!("{}/{}", name, version)
}

// Export the entry points for role_manager
pub use role_manager::{
    // Main entry points
    instantiate as role_manager_instantiate,
    execute as role_manager_execute,
    query as role_manager_query,
    // Execute functions
    grantRole, revokeRole, renounceRole, authorizeFanAssigner, assignFanRole,
    pause as role_manager_pause, unpause as role_manager_unpause,
    // Query functions
    has_role,
    // Constants
    DEFAULT_ADMIN_ROLE, FAN_ROLE, ORGANIZER_ROLE, ARTIST_ROLE, 
    BRAND_ROLE, MODERATOR_ROLE, FAN_ASSIGNER_ROLE,
    // Storage
    PAUSED as ROLE_MANAGER_PAUSED,
    // Response types
    RolesResponse, BoolResponse as RoleBoolResponse,
    // Messages
    InstantiateMsg as RoleManagerInstantiateMsg,
    ExecuteMsg as RoleManagerExecuteMsg,
    QueryMsg as RoleManagerQueryMsg,
};

// Export the entry points for profile_nft_minter
pub use profile_nft_minter::{
    // Main entry points
    instantiate as profile_nft_instantiate,
    execute as profile_nft_execute,
    query as profile_nft_query,
    // Execute functions
    createProfile, updateProfileMetadata, transferFrom,
    mintProfileNFT, updateProfileNFT, setAdmin, setPaused,
    addToWhitelist, removeFromWhitelist,
    // Query functions
    usernameExists_query, getProfileByTokenId, getTokenIdByUsername, 
    ownerOf, balanceOf, queryProfileNFT, queryToken, is_whitelisted,
    // Helper functions
    _validateUsername, _toLowerCase,
    // Storage
    CONFIG as PROFILE_CONFIG, ADMIN, PAUSED, WHITELIST,
    // Response types
    BoolResponse as ProfileBoolResponse, 
    ProfileResponse, TokenIdResponse, OwnerResponse,
    ProfileNFT, TokenInfo, ProfileNFTResponse, TokenResponse,
    // Messages
    InstantiateMsg as ProfileNFTInstantiateMsg,
    ExecuteMsg as ProfileNFTExecuteMsg,
    QueryMsg as ProfileNFTQueryMsg,
    // Structs
    Config as ProfileConfig,
};

// Export the entry points for tribe_controller
pub use tribe_controller::{
    // Main entry points
    instantiate as tribe_controller_instantiate,
    execute as tribe_controller_execute,
    query as tribe_controller_query,
    // Execute functions
    createTribe, updateTribe, updateTribeConfig, joinTribe,
    requestToJoinTribe, approveMember, rejectMember, banMember,
    joinTribeWithCode, createInviteCode, requestMerge,
    approveMerge, executeMerge, revokeInviteCode, cancelMerge,
    // Query functions
    getTribeAdmin, getTribeWhitelist, isAddressWhitelisted,
    getMemberStatus, getTribeConfigView, getMemberCount,
    getUserTribes, getInviteCodeStatus, getMergeRequest, getTribeDetails,
    is_tribe_member, is_tribe_admin_check, is_tribe_member_with_status,
    is_whitelisted as tribe_is_whitelisted,
    // Types and Enums
    JoinType, MemberStatus, NFTType, NFTRequirement,
    InviteCode, MergeRequest, TribeConfigView, Config as TribeConfig,
    TribeMeta, TribeData, TribeMember, TribeDetailsView,
    // Response types
    AdminResponse, WhitelistResponse, BoolResponse as TribeBoolResponse,
    MemberStatusResponse, TribeConfigViewResponse, MemberCountResponse,
    UserTribesResponse, InviteCodeStatusResponse, MergeRequestResponse,
    // Query enums
    RoleManagerQuery as TribeRoleManagerQuery, 
    Erc721Query, Erc1155Query,
    // Messages
    InstantiateMsg as TribeControllerInstantiateMsg,
    ExecuteMsg as TribeControllerExecuteMsg,
    QueryMsg as TribeControllerQueryMsg,
    // Other response types
    RoleResponse, BalanceResponse as TribeBalanceResponse, OwnerResponse as TribeOwnerResponse,
};

// Export the entry points for post_minter
pub use post_minter::{
    // Main entry points
    instantiate as post_minter_instantiate,
    execute as post_minter_execute,
    // Execute functions
    createPost, createReply, createEncryptedPost, createSignatureGatedPost,
    deletePost, reportPost, authorizeViewer, setTribeEncryptionKey,
    interactWithPost, createBatchPosts, setPostTypeCooldown, updatePost,
    pause as post_pause, unpause as post_unpause,
    // Query functions 
    canViewPost, verifyPostAccess, getInteractionCount, getPostReplies,
    query_get_post, validateMetadata, getPostTypeCooldown, getRemainingCooldown,
    getBatchPostingLimits, getPostsByTribe, getPostsByUser, getPostsByTribeAndUser,
    getFeedForUser,
    // Types and Enums
    PostType, InteractionType, PostData, BatchPostData,
    Config as PostConfig, InstantiateMsg as PostMinterInstantiateMsg,
    ExecuteMsg as PostMinterExecuteMsg, QueryMsg as PostMinterQueryMsg,
    // Response types
    BoolResponse as PostBoolResponse, GetPostResponse, 
    InteractionCountResponse, PostRepliesResponse, StringResponse,
    PostsResponse, CooldownResponse, BatchLimitsResponse,
    // Query enums
    RoleManagerQuery as PostRoleManagerQuery, 
    TribeControllerQuery, CollectibleControllerQuery, 
    MemberStatus as PostMemberStatus, MemberStatusResponse as PostMemberStatusResponse,
    TribeAdminResponse as PostTribeAdminResponse,
    CollectibleResponse, BalanceResponse as PostBalanceResponse,
    // Constants - now public
    DEFAULT_ADMIN_ROLE as POST_DEFAULT_ADMIN_ROLE, 
    RATE_LIMIT_MANAGER_ROLE, PROJECT_CREATOR_ROLE,
};

// Export testing utilities
pub use testing::{
    mock_dependencies, mock_env, mock_info,
};

// Run with: cargo test -- --nocapture
#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use crate::testing::{mock_dependencies, mock_env, mock_info};
    
    // Import required modules
    use super::post_minter::{
        instantiate as post_minter_instantiate,
        InstantiateMsg as PostMinterInstantiateMsg,
    };
    
    use super::role_manager::{
        instantiate as role_manager_instantiate,
        InstantiateMsg as RoleManagerInstantiateMsg,
        DEFAULT_ADMIN_ROLE,
        has_role,
    };
    
    #[test]
    pub fn test_post_minter_initialization() {
        println!("Running post minter initialization test...");
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        
        let msg = PostMinterInstantiateMsg {
            role_manager: "role_manager".to_string(),
            tribe_controller: "tribe_controller".to_string(),
            collectible_controller: "collectible_controller".to_string(),
            feed_manager: "feed_manager".to_string(),
        };
        
        let response = post_minter_instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(response.attributes.len(), 2);
        println!("Post minter initialization test passed!");
    }

    #[test]
    pub fn test_role_manager_initialization() {
        println!("Running role manager initialization test...");
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        
        let msg = RoleManagerInstantiateMsg {};
        
        let response = role_manager_instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(response.attributes.len(), 2);
        
        // Check that admin role was granted
        let has_admin = has_role(deps.as_ref().storage, &Addr::unchecked("creator"), DEFAULT_ADMIN_ROLE).unwrap();
        assert!(has_admin);
        println!("Role manager initialization test passed!");
    }

    #[test]
    pub fn simple_test() {
        println!("Running simple test...");
        assert_eq!(2 + 2, 4);
        println!("Simple test passed!");
    }
} 