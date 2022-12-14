#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, ENTRY_SEQ};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:my-first-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    let owner = msg
        .owner
        .and_then(|addr_string| { deps.api.addr_validate(addr_string.as_str()).ok() })
        .unwrap_or(info.sender);

    let config = Config {
        owner: owner.clone(),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    ENTRY_SEQ.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::NewEntry {description, priority} =>
            execute::create_entry(deps, info, description, priority),
        ExecuteMsg::UpdateEntry { id, description, status, priority } =>
            execute::update_entry(deps, info, id, description, status, priority),
        ExecuteMsg::DeleteEntry {id} =>
            execute::delete_entry(deps, info, id),
    }
}

pub mod execute {
    use std::ops::Add;
    use crate::state::{Entry, LIST, Priority, Status};
    use super::*;

    pub fn create_entry(
        deps: DepsMut,
        info: MessageInfo,
        description: String,
        priority: Option<Priority>,
    ) -> Result<Response, ContractError> {

        let owner = CONFIG.load(deps.storage)?.owner;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {})
        }

        let id = ENTRY_SEQ.update::<_, cosmwasm_std::StdError>(deps.storage, |id| {
            Ok(id.add(1))
        })?;

        let new_entry = Entry {
            id,
            description,
            priority: priority.unwrap_or(Priority::None),
            status: Status::ToDo
        };
        LIST.save(deps.storage, id, &new_entry)?;

        Ok(Response::new().add_attribute("method", "create_entry")
            .add_attribute("new_entry_id", id.to_string()))
    }

    pub fn update_entry(
        deps: DepsMut,
        info: MessageInfo,
        id: u64,
        description: Option<String>,
        status: Option<Status>,
        priority: Option<Priority>,
    ) -> Result<Response, ContractError> {

        let owner = CONFIG.load(deps.storage)?.owner;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {});
        }

        let entry = LIST.load(deps.storage, id)?;

        let updated_entry = Entry {
            id,
            description: description.unwrap_or(entry.description),
            status: status.unwrap_or(entry.status),
            priority: priority.unwrap_or(entry.priority),
        };
        LIST.save(deps.storage, id, &updated_entry)?;

        Ok(Response::new().add_attribute("method", "update_entry")
            .add_attribute("updated_entry_id", id.to_string()))
    }

    pub fn delete_entry(
        deps: DepsMut,
        info: MessageInfo,
        id: u64,
    ) -> Result<Response, ContractError> {

        let owner = CONFIG.load(deps.storage)?.owner;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {});
        }

        LIST.remove(deps.storage, id);

        Ok(Response::new().add_attribute("method", "delete_entry")
            .add_attribute("deleted_entry_id", id.to_string()))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryEntry {id} =>
            to_binary(&query::query_entry(deps, id)?),
        QueryMsg::QueryList {start_after, limit} =>
            to_binary(&query::query_list(deps, start_after, limit)?),
    }
}

pub mod query {
    use cosmwasm_std::Order;
    use cw_storage_plus::Bound;
    use crate::msg::{EntryResponse, ListResponse};
    use crate::state::LIST;
    use super::*;

    pub fn query_entry(deps: Deps, id: u64) -> StdResult<EntryResponse> {
        let entry = LIST.load(deps.storage, id)?;
        Ok(EntryResponse {
            id: entry.id,
            description: entry.description,
            status: entry.status,
            priority: entry.priority,
        })
    }

    // Limits for pagination
    const MAX_LIMIT: u32 = 30;
    const DEFAULT_LIMIT: u32 = 10;

    pub fn query_list(
        deps: Deps,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> StdResult<ListResponse> {

        let start = start_after.map(Bound::exclusive);
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

        let entries: StdResult<Vec<_>> = LIST.range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .collect();

        let result = ListResponse {
          entries: entries?.into_iter().map(|l| l.1).collect(),
        };

        Ok(result)
    }
}

