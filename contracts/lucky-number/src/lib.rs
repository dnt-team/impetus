//! Precompile to call parachain-staking runtime methods via the EVM

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::Currency,
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{ConstU32, U256};
use sp_std::{convert::TryInto, marker::PhantomData};
use sp_std::vec::Vec;

type BalanceOf<Runtime> = <<Runtime as pallet_lucky_number::Config>::Currency as Currency<
	<Runtime as frame_system::Config>::AccountId,
>>::Balance;

pub const ARRAY_LIMIT: u32 = 2u32.pow(9);

type GetArrayLimit = ConstU32<ARRAY_LIMIT>;

pub struct LuckyNumberPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> LuckyNumberPrecompile<Runtime>
where
	Runtime: pallet_lucky_number::Config + pallet_evm::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime::RuntimeCall: From<pallet_lucky_number::Call<Runtime>>,
	BalanceOf<Runtime>: TryFrom<U256> + Into<U256>,
{
	#[precompile::public("buyTicket(uint8[],uint256[])")]
	fn buy_ticket(
		handle: &mut impl PrecompileHandle,
		numbers: BoundedVec<u8, GetArrayLimit>,
		amounts: BoundedVec<U256, GetArrayLimit>,
	) -> EvmResult {
		let numbers = Vec::from(numbers);

		let amounts = Vec::from(amounts);
		let parsed_amounts: Vec<_> = amounts.iter().map(|&amount|{
			Self::u256_to_amount(amount).in_field("amount").unwrap()
		}).collect(); 
		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let selections = numbers.into_iter().zip(parsed_amounts.into_iter()).collect();
		let call = pallet_lucky_number::Call::<Runtime>::buy_ticket { selections };
		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("claimReward(address,uint32,uint8)")]
	fn claim_reward(
		handle: &mut impl PrecompileHandle,
		who: Address,
		round: u32,
		number: u8,
	) -> EvmResult {
		let who = Runtime::AddressMapping::into_account_id(who.0);
		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let call = pallet_lucky_number::Call::<Runtime>::claim_reward { who, round, number };
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
