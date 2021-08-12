use std::{
    convert::{TryFrom, TryInto},
    fmt,
};

use anyhow::{ensure, Context, Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, Executor, FromRow};

use crate::{
    ibc::core::ics24_host::identifier::{ChainId, Identifier},
    Db,
};

/// Denotes an operation on an account
#[derive(Debug)]
pub struct Operation {
    /// ID of operation
    pub id: i64,
    /// Address of the account
    pub address: String,
    /// Denom of tokens
    pub denom: Identifier,
    /// Amount of tokens
    pub amount: u32,
    /// Type of operation
    pub operation_type: OperationType,
    /// On-chain transaction hash
    pub transaction_hash: String,
    /// Time at which this operation was created
    pub created_at: DateTime<Utc>,
}

/// Denotes an operation on an account
#[derive(Debug, FromRow)]
pub struct RawOperation {
    /// ID of operation
    pub id: i64,
    /// Address of the account
    pub address: String,
    /// Denom of tokens
    pub denom: String,
    /// Amount of tokens
    pub amount: u32,
    /// Type of operation
    pub operation_type: Json<OperationType>,
    /// On-chain transaction hash
    pub transaction_hash: String,
    /// Time at which this operation was created
    pub created_at: DateTime<Utc>,
}

impl From<Operation> for RawOperation {
    fn from(op: Operation) -> Self {
        Self {
            id: op.id,
            address: op.address,
            denom: op.denom.to_string(),
            amount: op.amount,
            operation_type: Json(op.operation_type),
            transaction_hash: op.transaction_hash,
            created_at: op.created_at,
        }
    }
}

impl TryFrom<RawOperation> for Operation {
    type Error = Error;

    fn try_from(op: RawOperation) -> Result<Self, Self::Error> {
        Ok(Self {
            id: op.id,
            address: op.address,
            denom: op.denom.parse()?,
            amount: op.amount,
            operation_type: op.operation_type.0,
            transaction_hash: op.transaction_hash,
            created_at: op.created_at,
        })
    }
}

/// Different types of possible operations on an account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    /// Mint some tokens on IBC enabled chain
    Mint {
        /// Chain ID of IBC enabled chain
        chain_id: ChainId,
    },
    /// Burn some tokens on IBC enabled chain
    Burn {
        /// Chain ID of IBC enabled chain
        chain_id: ChainId,
    },
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mint { chain_id } => write!(f, "mint [{}]", chain_id),
            Self::Burn { chain_id } => write!(f, "burn [{}]", chain_id),
        }
    }
}

/// Adds an account operation to database
pub async fn add_operation<'e>(
    executor: impl Executor<'e, Database = Db>,
    address: &str,
    denom: &Identifier,
    amount: u32,
    operation_type: &OperationType,
    transaction_hash: &str,
) -> Result<()> {
    let operation_type = Json(operation_type);

    let rows_affected = sqlx::query(
        "INSERT INTO operations (address, denom, amount, operation_type, transaction_hash) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(address)
    .bind(denom.to_string())
    .bind(amount)
    .bind(operation_type)
    .bind(transaction_hash)
    .execute(executor)
    .await
    .context("unable to add new account operation to database")?
    .rows_affected();

    ensure!(
        rows_affected == 1,
        "rows_affected should be equal to 1 when adding a new account operation"
    );

    Ok(())
}

/// Fetches account operations from database
pub async fn get_operations<'e>(
    executor: impl Executor<'e, Database = Db>,
    address: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<Operation>> {
    let raw: Vec<RawOperation> = sqlx::query_as(
        "SELECT * FROM operations WHERE address = $1 ORDER BY id DESC LIMIT $2 OFFSET $3",
    )
    .bind(address)
    .bind(limit)
    .bind(offset)
    .fetch_all(executor)
    .await
    .context("unable to query account operations from database")?;

    raw.into_iter().map(TryInto::try_into).collect()
}