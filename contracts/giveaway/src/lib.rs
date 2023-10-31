//! Precompile to call parachain-staking runtime methods via the EVM

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	traits::Currency,
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{ConstU32, U256};
use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};
use sp_runtime::traits::{Dispatchable, Hash, StaticLookup};

type BalanceOf<Runtime> = <<Runtime as pallet_ocw_giveaway::Config>::Currency as Currency<
	<Runtime as frame_system::Config>::AccountId,
>>::Balance;

pub const ARRAY_LIMIT: u32 = 2u32.pow(9);

type GetArrayLimit = ConstU32<ARRAY_LIMIT>;

pub struct GiveawayPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> GiveawayPrecompile<Runtime>
where
	Runtime: pallet_ocw_giveaway::Config + pallet_evm::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime::RuntimeCall: From<pallet_ocw_giveaway::Call<Runtime>>,
	BalanceOf<Runtime>: TryFrom<U256> + Into<U256>,
{
	#[precompile::public("createGiveaway(string,uint32,uint32,uint8,uint8,uint8,uint32,uint256,uint32)")]
	fn create_giveaway(
		handle: &mut impl PrecompileHandle,
		name: BoundedString<GetArrayLimit>,
		start: u32,
		end: u32,
		kyc_status: u8,
		random_type: u8,
		asset_type: u8,
		asset_id: u32,
		amount: U256,
		max_join: u32,
	) -> EvmResult {
		// Build call with origin.
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let kyc = match kyc_status {
			0 => pallet_ocw_giveaway::KYCStatus::Tier0,
			_ => pallet_ocw_giveaway::KYCStatus::Tier1,
		};

		let random_type = match random_type {
			_ => pallet_ocw_giveaway::RandomType::Chainlink,
		};

		let asset_type = match asset_type {
			_ => pallet_ocw_giveaway::AssetType::FungibleToken,
		};

		let call = pallet_ocw_giveaway::Call::<Runtime>::create_give_away {
			name: name.into(),
			start_block: start.into(),
			end_block: end.into(),
			kyc,
			random_type,
			asset_type,
			token: Some(pallet_ocw_giveaway::TokenInfo {
				asset_id,
				amount
			}),
			max_join,
		};
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("claimReward(uint32)")]
	fn claim_reward(handle: &mut impl PrecompileHandle, round: u32) -> EvmResult {
		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let call = pallet_ocw_giveaway::Call::<Runtime>::claim_reward { round };
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("participate(uint32)")]
	fn participate(handle: &mut impl PrecompileHandle, index: u32) -> EvmResult {
		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let call = pallet_ocw_giveaway::Call::<Runtime>::participate { index };
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime>> {
		value
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("balance type").into())
	}
}
