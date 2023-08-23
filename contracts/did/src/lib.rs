//! Precompile to call parachain-staking runtime methods via the EVM

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::{Currency, DefensiveTruncateFrom},
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{ConstU32, U256};
use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};


pub const ARRAY_LIMIT: u32 = 2u32.pow(9);

type GetArrayLimit = ConstU32<ARRAY_LIMIT>;

pub struct DidPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> DidPrecompile<Runtime>
where
	Runtime: pallet_did::Config + pallet_evm::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime::RuntimeCall: From<pallet_did::Call<Runtime>>,
{
	#[precompile::public("externalIdAddress(address,string)")]
	#[precompile::view]
	fn external_id_address(
		handle: &mut impl PrecompileHandle,
		user: Address,
		provider: BoundedString<GetArrayLimit>,
	) -> EvmResult<UnboundedBytes> {
		// Build call with origin.
		// AccountsPayable: Twox64Concat(8) + AccountId(20) + RewardPoint(32) 128
		handle.record_db_read::<Runtime>(188)?;
		let user = Runtime::AddressMapping::into_account_id(user.0);
		let provider: pallet_did::Provider = pallet_did::Provider::defensive_truncate_from(provider.as_bytes().to_vec());
		Ok(pallet_did::Pallet::<Runtime>::external_id(user, provider).to_vec().into())
	}


}
