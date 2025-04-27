use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Order,
    entry_point,
};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::errors::ContractError;
use crate::testing::{mock_dependencies, mock_env, mock_info};

// Constants for role definitions
pub const FAN_ROLE: &str = "FAN_ROLE";
pub const ORGANIZER_ROLE: &str = "ORGANIZER_ROLE";
pub const ARTIST_ROLE: &str = "ARTIST_ROLE";
pub const BRAND_ROLE: &str = "BRAND_ROLE";
pub const MODERATOR_ROLE: &str = "MODERATOR_ROLE";
pub const FAN_ASSIGNER_ROLE: &str = "FAN_ASSIGNER_ROLE";
pub const DEFAULT_ADMIN_ROLE: &str = "DEFAULT_ADMIN_ROLE";

// Storage items using cw-storage-plus
const ROLES: Map<(&str, &str), bool> = Map::new("roles");

// Add storage for paused state
pub const PAUSED: Item<bool> = Item::new("paused");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    GrantRole { user: String, role: String },
    RevokeRole { user: String, role: String },
    RenounceRole { role: String },
    AuthorizeFanAssigner { assigner: String },
    AssignFanRole { user: String },
    Pause {},
    Unpause {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    HasRole { user: String, role: String },
    HasAnyRole { user: String, roles: Vec<String> },
    HasAllRoles { user: String, roles: Vec<String> },
    GetUserRoles { user: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RolesResponse {
    pub roles: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BoolResponse {
    pub result: bool,
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Grant the contract deployer the default admin role
    ROLES.save(deps.storage, (info.sender.as_str(), DEFAULT_ADMIN_ROLE), &true)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::GrantRole { user, role } => grantRole(deps, info, user, role),
        ExecuteMsg::RevokeRole { user, role } => revokeRole(deps, info, user, role),
        ExecuteMsg::RenounceRole { role } => renounceRole(deps, info, role),
        ExecuteMsg::AuthorizeFanAssigner { assigner } => authorizeFanAssigner(deps, info, assigner),
        ExecuteMsg::AssignFanRole { user } => assignFanRole(deps, info, user),
        ExecuteMsg::Pause {} => pause(deps, info),
        ExecuteMsg::Unpause {} => unpause(deps, info),
    }
}

pub fn grantRole(
    deps: DepsMut,
    info: MessageInfo,
    user: String,
    role: String,
) -> Result<Response, ContractError> {
    // Check if sender has admin role
    if !has_role(deps.storage, &info.sender, DEFAULT_ADMIN_ROLE)? {
        return Err(ContractError::Unauthorized {});
    }

    let user_addr = deps.api.addr_validate(&user)?;
    
    // Grant role to user
    ROLES.save(deps.storage, (user_addr.as_str(), &role), &true)?;

    Ok(Response::new()
        .add_attribute("action", "grant_role")
        .add_attribute("user", user)
        .add_attribute("role", role))
}

pub fn revokeRole(
    deps: DepsMut,
    info: MessageInfo,
    user: String,
    role: String,
) -> Result<Response, ContractError> {
    // Check if sender has admin role
    if !has_role(deps.storage, &info.sender, DEFAULT_ADMIN_ROLE)? {
        return Err(ContractError::Unauthorized {});
    }

    let user_addr = deps.api.addr_validate(&user)?;
    
    // Revoke role from user
    ROLES.remove(deps.storage, (user_addr.as_str(), &role));

    Ok(Response::new()
        .add_attribute("action", "revoke_role")
        .add_attribute("user", user)
        .add_attribute("role", role))
}

pub fn renounceRole(
    deps: DepsMut,
    info: MessageInfo,
    role: String,
) -> Result<Response, ContractError> {
    // User can only renounce their own roles
    // Remove the role from the sender
    ROLES.remove(deps.storage, (info.sender.as_str(), &role));

    Ok(Response::new()
        .add_attribute("action", "renounce_role")
        .add_attribute("user", info.sender)
        .add_attribute("role", role))
}

pub fn authorizeFanAssigner(
    deps: DepsMut,
    info: MessageInfo,
    assigner: String,
) -> Result<Response, ContractError> {
    // Check if sender has admin role
    if !has_role(deps.storage, &info.sender, DEFAULT_ADMIN_ROLE)? {
        return Err(ContractError::Unauthorized {});
    }

    let assigner_addr = deps.api.addr_validate(&assigner)?;
    
    // Grant FAN_ASSIGNER_ROLE to assigner
    ROLES.save(deps.storage, (assigner_addr.as_str(), FAN_ASSIGNER_ROLE), &true)?;

    Ok(Response::new()
        .add_attribute("action", "authorize_fan_assigner")
        .add_attribute("assigner", assigner))
}

pub fn assignFanRole(
    deps: DepsMut,
    info: MessageInfo,
    user: String,
) -> Result<Response, ContractError> {
    // Check if sender has admin role or FAN_ASSIGNER_ROLE
    if !has_role(deps.storage, &info.sender, DEFAULT_ADMIN_ROLE)? && 
       !has_role(deps.storage, &info.sender, FAN_ASSIGNER_ROLE)? {
        return Err(ContractError::Unauthorized {});
    }

    let user_addr = deps.api.addr_validate(&user)?;
    
    // Grant FAN_ROLE to user
    ROLES.save(deps.storage, (user_addr.as_str(), FAN_ROLE), &true)?;

    Ok(Response::new()
        .add_attribute("action", "assign_fan_role")
        .add_attribute("user", user))
}

pub fn pause(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Check if sender has admin role
    if !has_role(deps.storage, &info.sender, DEFAULT_ADMIN_ROLE)? {
        return Err(ContractError::Unauthorized {});
    }

    // Set PAUSED state to true
    PAUSED.save(deps.storage, &true)?;
    
    Ok(Response::new()
        .add_attribute("action", "pause")
        .add_attribute("sender", info.sender))
}

pub fn unpause(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Check if sender has admin role
    if !has_role(deps.storage, &info.sender, DEFAULT_ADMIN_ROLE)? {
        return Err(ContractError::Unauthorized {});
    }

    // Set PAUSED state to false
    PAUSED.save(deps.storage, &false)?;
    
    Ok(Response::new()
        .add_attribute("action", "unpause")
        .add_attribute("sender", info.sender))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::HasRole { user, role } => to_json_binary(&query_has_role(deps, user, role)?),
        QueryMsg::HasAnyRole { user, roles } => to_json_binary(&has_any_role(deps, user, roles)?),
        QueryMsg::HasAllRoles { user, roles } => to_json_binary(&has_all_roles(deps, user, roles)?),
        QueryMsg::GetUserRoles { user } => to_json_binary(&get_user_roles(deps, user)?),
    }
}

fn query_has_role(deps: Deps, user: String, role: String) -> StdResult<BoolResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let result = has_role(deps.storage, &user_addr, &role)?;
    Ok(BoolResponse { result })
}

fn has_any_role(deps: Deps, user: String, roles: Vec<String>) -> StdResult<BoolResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    
    for role in roles {
        if has_role(deps.storage, &user_addr, &role)? {
            return Ok(BoolResponse { result: true });
        }
    }
    
    Ok(BoolResponse { result: false })
}

fn has_all_roles(deps: Deps, user: String, roles: Vec<String>) -> StdResult<BoolResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    
    for role in roles {
        if !has_role(deps.storage, &user_addr, &role)? {
            return Ok(BoolResponse { result: false });
        }
    }
    
    Ok(BoolResponse { result: true })
}

fn get_user_roles(deps: Deps, user: String) -> StdResult<RolesResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    
    // Define potential roles to check
    let all_roles = vec![
        DEFAULT_ADMIN_ROLE.to_string(),
        FAN_ROLE.to_string(),
        ORGANIZER_ROLE.to_string(),
        ARTIST_ROLE.to_string(),
        BRAND_ROLE.to_string(),
        MODERATOR_ROLE.to_string(),
        FAN_ASSIGNER_ROLE.to_string(),
    ];
    
    let mut user_roles = Vec::new();
    for role in all_roles {
        if has_role(deps.storage, &user_addr, &role)? {
            user_roles.push(role);
        }
    }
    
    Ok(RolesResponse { roles: user_roles })
}

// Helper to check if an address has a role
pub fn has_role(storage: &dyn cosmwasm_std::Storage, user: &Addr, role: &str) -> StdResult<bool> {
    Ok(ROLES.may_load(storage, (user.as_str(), role))?.unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        
        // Instantiate the contract
        let res = instantiate(deps.as_mut(), mock_env(), info, InstantiateMsg {}).unwrap();
        assert_eq!(0, res.messages.len());
        
        // Verify creator has admin role
        let has_role = query_has_role(
            deps.as_ref(),
            "creator".to_string(),
            DEFAULT_ADMIN_ROLE.to_string(),
        )
        .unwrap();
        assert_eq!(true, has_role.result);
    }

    #[test]
    fn test_grant_role() {
        let mut deps = mock_dependencies();
        let admin_info = mock_info("admin", &[]);
        
        // Instantiate contract with admin
        instantiate(deps.as_mut(), mock_env(), admin_info.clone(), InstantiateMsg {}).unwrap();
        
        // Grant ARTIST_ROLE to user
        let res = grantRole(
            deps.as_mut(),
            admin_info,
            "user".to_string(),
            ARTIST_ROLE.to_string(),
        )
        .unwrap();
        
        assert_eq!("grant_role", res.attributes[0].value);
        
        // Verify user has ARTIST_ROLE
        let has_role = query_has_role(
            deps.as_ref(),
            "user".to_string(),
            ARTIST_ROLE.to_string(),
        )
        .unwrap();
        assert_eq!(true, has_role.result);
    }
} 