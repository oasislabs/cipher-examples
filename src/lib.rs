//! A dead-person's switch that reveals data if the cell stops being refreshed.
#![deny(rust_2018_idioms, unreachable_pub)]
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]
#![forbid(unsafe_code)]

use oasis_contract_sdk::{
    self as sdk,
    env::Env,
    types::{
        address::Address,
        env::{QueryRequest, QueryResponse},
        message::Reply,
    },
};
use oasis_contract_sdk_storage::map::Map as StorageMap;

use vigil_types::{Error, Request, Response, RevelationSet};

pub struct Vigil;

type SecretId<'a> = (Address /* owner */, &'a str /* secret name */);
const REVELATION_TSTAMPS: StorageMap<'static, SecretId<'_>, u64> = StorageMap::new(b"t");
const REVELATION_SETS: StorageMap<'static, SecretId<'_>, RevelationSet> = StorageMap::new(b"s");
const SECRET_VALUES: StorageMap<'static, SecretId<'_>, Vec<u8>> = StorageMap::new(b"v");

/// A convenient way to access the contract store that makes it harder to use the wrong one.
macro_rules! store {
    ($ctx:ident) => {
        $ctx.confidential_store()
    };
}

impl Vigil {
    /// Increment the counter and return the previous value.
    fn create_secret<C: sdk::Context>(
        ctx: &mut C,
        name: &str,
        value: Vec<u8>,
        revelation_set: RevelationSet,
        revelation_timestamp: u64,
    ) -> Result<(), Error> {
        let secret_id = (*ctx.caller_address(), name);
        if Self::secret_exists(ctx, secret_id) {
            return Err(Error::SecretAlreadyExists);
        }
        let store = store!(ctx);
        REVELATION_TSTAMPS.insert(store, secret_id, revelation_timestamp);
        REVELATION_SETS.insert(store, secret_id, revelation_set);
        SECRET_VALUES.insert(store, secret_id, value);
        Ok(())
    }

    fn secret_revelation_timestamp<C: sdk::Context>(
        ctx: &mut C,
        owner: Address,
        name: &str,
    ) -> Result<u64, Error> {
        let secret_id = (owner, name);
        let caller = *ctx.caller_address();
        if caller == owner {
            return REVELATION_TSTAMPS
                .get(store!(ctx), secret_id)
                .ok_or(Error::SecretDoesntExist);
        }
        if !Self::secret_is_revealable_to(ctx, secret_id, &caller) {
            return Err(Error::PermissionDenied);
        }
        REVELATION_TSTAMPS
            .get(store!(ctx), secret_id)
            .ok_or(Error::SecretDoesntExist)
    }

    fn secret_revelation_set<C: sdk::Context>(
        ctx: &mut C,
        name: &str,
    ) -> Result<RevelationSet, Error> {
        let secret_id = (*ctx.caller_address(), name);
        REVELATION_SETS
            .get(store!(ctx), secret_id)
            .ok_or(Error::SecretDoesntExist)
    }

    fn reset_revelation_timestamp<C: sdk::Context>(
        ctx: &mut C,
        name: &str,
        new_revelation_timestamp: u64,
    ) -> Result<(), Error> {
        let secret_id = (*ctx.caller_address(), name);
        if !Self::secret_exists(ctx, secret_id) {
            return Err(Error::SecretDoesntExist);
        }
        REVELATION_TSTAMPS.insert(store!(ctx), secret_id, new_revelation_timestamp);
        Ok(())
    }

    fn secret_value<C: sdk::Context>(
        ctx: &mut C,
        owner: Address,
        name: &str,
    ) -> Result<Vec<u8>, Error> {
        let caller = *ctx.caller_address();
        let secret_id = (owner, name);
        if caller == owner {
            return SECRET_VALUES
                .get(store!(ctx), secret_id)
                .ok_or(Error::SecretDoesntExist);
        }
        if !Self::secret_is_revealable_to(ctx, secret_id, &caller) {
            return Err(Error::SecretDoesntExist);
        }
        let block_tstamp = match ctx.env().query(QueryRequest::BlockInfo) {
            QueryResponse::BlockInfo { timestamp, .. } => timestamp,
            resp => panic!(
                "received unexpected response to block info request: {:?}",
                resp
            ),
        };
        let rev_tstamp = REVELATION_TSTAMPS
            .get(store!(ctx), secret_id)
            .ok_or(Error::PermissionDenied)?;
        if rev_tstamp > block_tstamp {
            return Err(Error::PermissionDenied);
        }
        SECRET_VALUES
            .get(store!(ctx), secret_id)
            .ok_or(Error::SecretDoesntExist)
    }

    fn delete_secret<C: sdk::Context>(ctx: &mut C, name: &str) -> Result<(), Error> {
        let secret_id = (*ctx.caller_address(), name);
        if !Self::secret_exists(ctx, secret_id) {
            return Ok(());
        }
        let store = store!(ctx);
        REVELATION_TSTAMPS.remove(store, secret_id);
        REVELATION_SETS.remove(store, secret_id);
        SECRET_VALUES.remove(store, secret_id);
        Ok(())
    }

    fn secret_exists<C: sdk::Context>(ctx: &mut C, id: SecretId<'_>) -> bool {
        REVELATION_TSTAMPS.get(store!(ctx), id).is_some()
    }

    fn secret_is_revealable_to<C: sdk::Context>(
        ctx: &mut C,
        id: SecretId<'_>,
        entity: &Address,
    ) -> bool {
        match REVELATION_SETS.get(store!(ctx), id) {
            Some(rev_set) => rev_set.contains(entity),
            None => false,
        }
    }
}

impl sdk::Contract for Vigil {
    type Request = Request;
    type Response = Response;
    type Error = Error;

    fn instantiate<C: sdk::Context>(_ctx: &mut C, request: Request) -> Result<(), Error> {
        if !matches!(request, Request::Instantiate) {
            return Err(Error::BadRequest); // The instantiate request must be `Instantiate`.
        }
        Ok(())
    }

    fn call<C: sdk::Context>(ctx: &mut C, request: Request) -> Result<Response, Error> {
        Ok(match request {
            Request::CreateSecret {
                name,
                value,
                revelation_set,
                revelation_timestamp,
            } => Vigil::create_secret(ctx, &name, value, revelation_set, revelation_timestamp)?
                .into(),
            Request::ResetRevelationTimestamp {
                name,
                revelation_timestamp,
            } => Vigil::reset_revelation_timestamp(ctx, &name, revelation_timestamp)?.into(),
            Request::DeleteSecret { name } => Vigil::delete_secret(ctx, &name)?.into(),
            Request::GetRevelationTimestamp { owner, name } => Response::RevelationTimestamp(
                Vigil::secret_revelation_timestamp(ctx, owner, &name)?,
            ),
            Request::GetRevelationSet { name } => {
                Response::RevelationSet(Vigil::secret_revelation_set(ctx, &name)?)
            }
            Request::GetSecretValue { owner, name } => {
                let value = Vigil::secret_value(ctx, owner, &name)?;
                Response::SecretValue(value)
            }
            _ => return Err(Error::BadRequest),
        })
    }

    fn query<C: sdk::Context>(_ctx: &mut C, _request: Request) -> Result<Response, Error> {
        Err(Error::BadRequest)
    }

    fn handle_reply<C: sdk::Context>(
        _ctx: &mut C,
        _reply: Reply,
    ) -> Result<Option<Self::Response>, Error> {
        // This contract does not call other contracts, so receiving a reply is erroneous.
        Err(Error::BadRequest)
    }

    fn pre_upgrade<C: sdk::Context>(_ctx: &mut C, _request: Self::Request) -> Result<(), Error> {
        Err(Error::UpgradeNotAllowed)
    }

    fn post_upgrade<C: sdk::Context>(_ctx: &mut C, _request: Self::Request) -> Result<(), Error> {
        Err(Error::UpgradeNotAllowed)
    }
}

sdk::create_contract!(Vigil);

#[cfg(test)]
mod test {
    use oasis_contract_sdk::{testing::MockContext, types::ExecutionContext, Contract};

    use super::*;

    fn make_address(num: u8) -> Address {
        Address::from_bytes(vec![num; Address::SIZE].as_slice()).unwrap()
    }

    const BLOCK_TIMESTAMP: u64 = 100_000; // The MockEnv timestamp is set to 100_000;

    #[test]
    fn firstparty_requests() {
        let mut ctx: MockContext = ExecutionContext::default().into();
        let owner = make_address(0);
        ctx.ec.caller_address = owner;

        Vigil::instantiate(&mut ctx, Request::Instantiate).unwrap();

        let secret_name = "test secret";
        let secret_value = "secret value";
        let revelation_set = RevelationSet::Entities(vec![make_address(1), make_address(2)]);
        let revelation_timestamp = BLOCK_TIMESTAMP + 1;

        let other_secret_name = "other test secret";
        let other_revelation_set = RevelationSet::Anyone;

        let create_secret_request = Request::CreateSecret {
            name: secret_name.into(),
            value: secret_value.into(),
            revelation_set: revelation_set.clone(),
            revelation_timestamp,
        };
        Vigil::call(&mut ctx, create_secret_request.clone()).unwrap();
        assert_eq!(
            Vigil::call(&mut ctx, create_secret_request.clone()),
            Err(Error::SecretAlreadyExists)
        );

        // Create a second secret to test the `Anyone` revelation set, and also
        // that deletion only removes the selected secret.
        Vigil::call(
            &mut ctx,
            Request::CreateSecret {
                name: other_secret_name.into(),
                value: secret_value.into(),
                revelation_set: other_revelation_set.clone(),
                revelation_timestamp,
            },
        )
        .unwrap();

        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetRevelationSet {
                    name: secret_name.into(),
                },
            )
            .unwrap(),
            Response::RevelationSet(revelation_set)
        );

        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetRevelationTimestamp {
                    owner,
                    name: secret_name.into(),
                },
            )
            .unwrap(),
            Response::RevelationTimestamp(revelation_timestamp)
        );

        let secret_value_request = Request::GetSecretValue {
            owner,
            name: secret_name.into(),
        };
        let secret_val = Vigil::call(&mut ctx, secret_value_request.clone()).unwrap();
        assert_eq!(secret_val, Response::SecretValue(secret_value.into()));

        let delete_secret_request = Request::DeleteSecret {
            name: secret_name.into(),
        };
        Vigil::call(&mut ctx, delete_secret_request.clone()).unwrap();
        // Secret deletion should be idempotent.:w
        Vigil::call(&mut ctx, delete_secret_request).unwrap();

        // The deleted secret shouldn't be accessible.
        assert_eq!(
            Vigil::call(&mut ctx, secret_value_request),
            Err(Error::SecretDoesntExist)
        );

        // The other secret should still exist.
        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetRevelationSet {
                    name: other_secret_name.into(),
                },
            )
            .unwrap(),
            Response::RevelationSet(other_revelation_set)
        );

        // Deleted secret should be able to be recreated.
        Vigil::call(&mut ctx, create_secret_request).unwrap();
    }

    #[test]
    fn secondparty_requests() {
        let mut ctx: MockContext = ExecutionContext::default().into();
        let owner = make_address(0);
        ctx.ec.caller_address = owner;

        Vigil::instantiate(&mut ctx, Request::Instantiate).unwrap();

        let beneficiary = make_address(1);

        let secret_name = "test secret";
        let secret_value = "secret value";
        let initial_revelation_timestamp = BLOCK_TIMESTAMP + 1; // Not yet revealed.
        let updated_revelation_timestamp = BLOCK_TIMESTAMP - 1; // Revealed.

        Vigil::call(
            &mut ctx,
            Request::CreateSecret {
                name: secret_name.into(),
                value: secret_value.into(),
                revelation_set: RevelationSet::Entities(vec![beneficiary]),
                revelation_timestamp: initial_revelation_timestamp,
            },
        )
        .unwrap();

        ctx.ec.caller_address = beneficiary;

        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetSecretValue {
                    owner,
                    name: secret_name.into(),
                },
            ),
            Err(Error::PermissionDenied)
        );

        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetRevelationTimestamp {
                    owner,
                    name: secret_name.into(),
                },
            ),
            Ok(Response::RevelationTimestamp(initial_revelation_timestamp))
        );

        ctx.ec.caller_address = owner;
        Vigil::call(
            &mut ctx,
            Request::ResetRevelationTimestamp {
                name: secret_name.into(),
                revelation_timestamp: updated_revelation_timestamp,
            },
        )
        .unwrap();

        ctx.ec.caller_address = beneficiary;
        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetSecretValue {
                    owner,
                    name: secret_name.into(),
                },
            )
            .unwrap(),
            Response::SecretValue(secret_value.into())
        );
    }

    #[test]
    fn thirdparty_requests() {
        let mut ctx: MockContext = ExecutionContext::default().into();
        let owner = make_address(0);
        ctx.ec.caller_address = owner;

        Vigil::instantiate(&mut ctx, Request::Instantiate).unwrap();

        let secret_name = "test secret";

        Vigil::call(
            &mut ctx,
            Request::CreateSecret {
                name: secret_name.into(),
                value: "secret value".into(),
                revelation_set: RevelationSet::Entities(vec![]),
                revelation_timestamp: BLOCK_TIMESTAMP - 1, // Already revealed.
            },
        )
        .unwrap();

        let thirdparty = make_address(3);
        ctx.ec.caller_address = thirdparty;

        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetRevelationTimestamp {
                    owner,
                    name: secret_name.into(),
                },
            ),
            Err(Error::PermissionDenied)
        );

        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetRevelationSet {
                    name: secret_name.into(),
                },
            ),
            Err(Error::SecretDoesntExist)
        );

        // The secret is not accessible by a third party.
        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetSecretValue {
                    owner,
                    name: secret_name.into(),
                },
            ),
            Err(Error::SecretDoesntExist)
        );

        // The contract storate is not totally broken.
        assert_eq!(
            Vigil::call(
                &mut ctx,
                Request::GetSecretValue {
                    owner: thirdparty,
                    name: secret_name.into(),
                },
            ),
            Err(Error::SecretDoesntExist)
        );
    }
}
