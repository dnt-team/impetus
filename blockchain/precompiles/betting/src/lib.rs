//! Precompile to call parachain-staking runtime methods via the EVM

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use core::str::from_utf8;
use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::{ConstU32, Currency},
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{H256, U256};
use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};
type BalanceOf<Runtime> = <<Runtime as pallet_betting::Config>::Currency as Currency<
	<Runtime as frame_system::Config>::AccountId,
>>::Balance;
pub struct BettingPrecompile<Runtime>(PhantomData<Runtime>);

type GetHashStringLimit = ConstU32<100>;

#[precompile_utils::precompile]
impl<Runtime> BettingPrecompile<Runtime>
where
	Runtime: pallet_betting::Config + pallet_evm::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime::RuntimeCall: From<pallet_betting::Call<Runtime>>,
	BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + EvmData,
	H256: From<<Runtime as frame_system::Config>::Hash>
		+ Into<<Runtime as frame_system::Config>::Hash>,
{
	#[precompile::public("bet(string,uint128,uint256)")]
	fn bet(
		handle: &mut impl PrecompileHandle,
		round_id: BoundedString<GetHashStringLimit>,
		bet_id: u128,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;
		let round_id: Vec<u8> = round_id.into();
		match array_bytes::hex_n_into::<_, H256, 32>(from_utf8(&round_id).unwrap()) {
			Ok(round_id) => {
				// Build call with origin.
				let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
				let call = pallet_betting::Call::<Runtime>::bet {
					round_id: round_id.into(),
					bet: bet_id,
					amount,
				};
				// Dispatch call (if enough gas).
				RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
			},
			_ => (),
		}

		Ok(())
	}

	fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime>> {
		value
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("balance type").into())
	}
}
