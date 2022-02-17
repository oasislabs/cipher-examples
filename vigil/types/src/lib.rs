//! Types used by the Vigil contract, separated as a crate for
//! inclusion by a future Rust client before ABI codegen exists.

use oasis_contract_sdk::{self as sdk, types::address::Address};

#[derive(Clone, Debug, PartialEq, Eq, cbor::Encode, cbor::Decode)]
pub enum Request {
    #[cbor(rename = "instantiate")]
    Instantiate,

    /// Requests the creation of a new secret scoped to the caller.
    #[cbor(rename = "create_secret")]
    CreateSecret {
        /// The name of the secret.
        name: String,

        /// The secret.
        value: Vec<u8>,

        /// The set of callers that can retrieve the revealed secret.
        revelation_set: RevelationSet,

        /// The timestamp at which this secret should be revealed unless refreshed.
        revelation_timestamp: u64,
    },

    /// Refreshes the expiry of the secret scoped to the caller with the specified name.
    #[cbor(rename = "reset_revelation_timestamp")]
    ResetRevelationTimestamp {
        name: String,
        /// The new timestamp at which this secret should be revealed unless refreshed.
        /// If the expiry is in the past, the secret will be immediately revealable.
        revelation_timestamp: u64,
    },

    /// Deletes the secret owned by the caller with the specified name.
    #[cbor(rename = "delete_secret")]
    DeleteSecret { name: String },

    /// If the caller is either the owner of the secret or in its revelation set,
    /// returns the revelation timestamp of the secret.
    #[cbor(rename = "get_revelation_timestamp")]
    GetRevelationTimestamp { owner: Address, name: String },

    /// If the caller is the owner of the secret, returns the revelation set of the secret.
    #[cbor(rename = "get_revelation_set")]
    GetRevelationSet { name: String },

    /// If the caller is either the owner of the secret or in its revelation set,
    /// and the current time is past the revelation time, returns the secret data.
    #[cbor(rename = "get_secret_value")]
    GetSecretValue { owner: Address, name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, cbor::Encode, cbor::Decode)]
pub enum RevelationSet {
    #[cbor(rename = "anyone")]
    Anyone,
    #[cbor(rename = "entities")]
    Entities(Vec<Address>),
}

impl RevelationSet {
    pub fn contains(&self, entity: &Address) -> bool {
        match self {
            Self::Anyone => true,
            Self::Entities(entities) => entities.contains(entity),
        }
    }
}

/// All possible responses that the contract can return.
///
/// This includes both calls and queries.
#[derive(Clone, Debug, PartialEq, Eq, cbor::Encode, cbor::Decode)]
pub enum Response {
    /// Returned as the result of a `GetRevelationTimestamp` request.
    #[cbor(rename = "revelation_timestamp")]
    RevelationTimestamp(u64),

    /// Returned as the result of a `GetRevelationSet` request.
    #[cbor(rename = "revelation_set")]
    RevelationSet(RevelationSet),

    /// Returned as the result of a `GetSecretValue` request.
    #[cbor(rename = "secret_value")]
    SecretValue(Vec<u8>),

    #[cbor(rename = "empty")]
    Empty,
}

impl From<()> for Response {
    fn from(_: ()) -> Self {
        Self::Empty
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, sdk::Error)]
pub enum Error {
    #[error("the contract is not upgradeable")]
    #[sdk_error(code = 0)]
    UpgradeNotAllowed,

    #[error("bad request")]
    #[sdk_error(code = 1)]
    BadRequest,

    #[error("permission denied")]
    #[sdk_error(code = 2)]
    PermissionDenied,

    #[error("the secret doesn't exist")]
    #[sdk_error(code = 3)]
    SecretDoesntExist,

    #[error("the secret already exists")]
    #[sdk_error(code = 4)]
    SecretAlreadyExists,
}
