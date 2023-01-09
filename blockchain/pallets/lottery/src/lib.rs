// This file is part of Substrate.

// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A lottery pallet that uses participation in the network to purchase tickets.
//!
//! With this pallet, you can configure a lottery, which is a pot of money that
//! users contribute to, and that is reallocated to a single user at the end of
//! the lottery period. Just like a normal lottery system, to participate, you
//! need to "buy a ticket", which is used to fund the pot.
//!
//! The unique feature of this lottery system is that tickets can only be
//! purchased by making a "valid call" dispatched through this pallet.
//! By configuring certain calls to be valid for the lottery, you can encourage
//! users to make those calls on your network. An example of how this could be
//! used is to set validator nominations as a valid lottery call. If the lottery
//! is set to repeat every month, then users would be encouraged to re-nominate
//! validators every month. A user can only purchase one ticket per valid call
//! per lottery.
//!
//! This pallet can be configured to use dynamically set calls or statically set
//! calls. Call validation happens through the `ValidateCall` implementation.
//! This pallet provides one implementation of this using the `CallIndices`
//! storage item. You can also make your own implementation at the runtime level
//! which can contain much more complex logic, such as validation of the
//! parameters, which this pallet alone cannot do.
//!
//! This pallet uses the modulus operator to pick a random winner. It is known
//! that this might introduce a bias if the random number chosen in a range that
//! is not perfectly divisible by the total number of participants. The
//! `MaxGenerateRandom` configuration can help mitigate this by generating new
//! numbers until we hit the limit or we find a "fair" number. This is best
//! effort only.

#![cfg_attr(not(feature = "std"), no_std)]

// mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchResult, Dispatchable, GetDispatchInfo},
	ensure,
	pallet_prelude::MaxEncodedLen,
	storage::bounded_vec::BoundedVec,
	traits::{Currency, ExistenceRequirement::KeepAlive, Get, Randomness, ReservableCurrency},
	PalletId, RuntimeDebug,
};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, Saturating, Zero},
	ArithmeticError, DispatchError,
};
use sp_std::prelude::*;
pub use weights::WeightInfo;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Any runtime call can be encoded into two bytes which represent the pallet and call index.
// We use this to uniquely match someone's incoming call with the calls configured for the lottery.
type LotterySelection = [u8; 10];
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub struct LotteryConfig<BlockNumber, Balance> {
	/// Price per entry.
	price: Balance,
	/// Starting block of the lottery.
	start: BlockNumber,
	/// Length of the lottery (start + length = end).
	length: BlockNumber,
	/// Delay for choosing the winner of the lottery. (start + length + delay = payout).
	/// Randomness in the "payout" block will be used to determine the winner.
	delay: BlockNumber,
	/// Whether this lottery will repeat after it completes.
	repeat: bool,
}

// Struct for holding kitty information
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct TicketInfo<T: Config> {
	pub account_id: T::AccountId,
	pub selection: LotterySelection,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The pallet's config trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The Lottery's pallet id
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The type used as a unique asset id,
		/// The type used as a unique asset id,
		type LotteryKind: Copy + Parameter + Member + Default + TypeInfo + MaxEncodedLen;

		/// A dispatchable call.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>;

		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The manager origin.
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Number of time we should try to generate a random number that has no modulo bias.
		/// The larger this number, the more potential computation is used for picking the winner,
		/// but also the more likely that the chosen winner is done fairly.
		#[pallet::constant]
		type MaxGenerateRandom: Get<u32>;

		/// The maximum number of members per member role.
		#[pallet::constant]
		type MaxParticipants: Get<u32>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A lottery has been started!
		LotteryStarted {
			kind: T::LotteryKind,
			index: u32,
		},
		/// A new set of calls have been set!
		CallsUpdated,
		/// A winner has been chosen!
		Winner {
			winner: T::AccountId,
			lottery_balance: BalanceOf<T>,
		},
		/// A ticket has been bought!
		TicketBought {
			kind: T::LotteryKind,
			index: u32,
			user: T::AccountId,
			selection: LotterySelection,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A lottery has not been configured.
		NotConfigured,
		/// A lottery is already in progress.
		InProgress,
		/// A lottery has already ended.
		AlreadyEnded,
		/// The call is not valid for an open lottery.
		InvalidCall,
		/// You are already participating in the lottery with this call.
		AlreadyParticipating,
		/// Too many calls for a single lottery.
		TooManyCalls,
		/// Failed to encode calls
		EncodingFailed,
		TooManyParticipants,
	}

	#[pallet::storage]
	pub(crate) type LotteryIndex<T: Config> =
		StorageMap<_, Twox64Concat, T::LotteryKind, u32, ValueQuery>;

	/// The configuration for the current lottery.
	#[pallet::storage]
	pub(crate) type Lottery<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::LotteryKind,
		Twox64Concat,
		u32,
		LotteryConfig<T::BlockNumber, BalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub(crate) type Participants<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, T::LotteryKind>,
			NMapKey<Twox64Concat, u32>,
			NMapKey<Twox64Concat, LotterySelection>,
		),
		BoundedVec<T::AccountId, T::MaxParticipants>,
		ValueQuery,
	>;

	/// Total number of tickets sold.
	#[pallet::storage]
	pub(crate) type TicketsCount<T> = StorageValue<_, u32, ValueQuery>;

	/// Each ticket's owner.
	///
	/// May have residual storage from previous lotteries. Use `TicketsCount` to see which ones
	/// are actually valid ticket mappings.
	#[pallet::storage]
	pub(crate) type Tickets<T: Config> = StorageMap<_, Twox64Concat, u32, T::AccountId>;

	// #[pallet::hooks]
	// impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
	// 	fn on_initialize(n: T::BlockNumber) -> Weight {
	// 		Lottery::<T>::mutate(|mut lottery| -> Weight {
	// 			if let Some(config) = &mut lottery {
	// 				let payout_block =
	// 					config.start.saturating_add(config.length).saturating_add(config.delay);
	// 				if payout_block <= n {
	// 					let (lottery_account, lottery_balance) = Self::pot();

	// 					let winner = Self::choose_account().unwrap_or(lottery_account);
	// 					// Not much we can do if this fails...
	// 					let res = T::Currency::transfer(
	// 						&Self::account_id(),
	// 						&winner,
	// 						lottery_balance,
	// 						KeepAlive,
	// 					);
	// 					debug_assert!(res.is_ok());

	// 					Self::deposit_event(Event::<T>::Winner { winner, lottery_balance });

	// 					TicketsCount::<T>::kill();

	// 					if config.repeat {
	// 						// If lottery should repeat, increment index by 1.
	// 						LotteryIndex::<T>::mutate(|index| *index = index.saturating_add(1));
	// 						// Set a new start with the current block.
	// 						config.start = n;
	// 						return T::WeightInfo::on_initialize_repeat()
	// 					} else {
	// 						// Else, kill the lottery storage.
	// 						*lottery = None;
	// 						return T::WeightInfo::on_initialize_end()
	// 					}
	// 					// We choose not need to kill Participants and Tickets to avoid a large
	// 					// number of writes at one time. Instead, data persists between lotteries,
	// 					// but is not used if it is not relevant.
	// 				}
	// 			}
	// 			T::DbWeight::get().reads(1)
	// 		})
	// 	}
	// }

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Buy a ticket to enter the lottery.
		///
		/// This extrinsic acts as a passthrough function for `call`. In all
		/// situations where `call` alone would succeed, this extrinsic should
		/// succeed.
		///
		/// If `call` is successful, then we will attempt to purchase a ticket,
		/// which may fail silently. To detect success of a ticket purchase, you
		/// should listen for the `TicketBought` event.
		///
		/// This extrinsic must be called by a signed origin.
		#[pallet::call_index(0)]
		#[pallet::weight((10_100, DispatchClass::Normal, Pays::No))]
		pub fn buy_ticket(
			origin: OriginFor<T>,
			kind: T::LotteryKind,
			index: u32,
			selection: LotterySelection,
		) -> DispatchResult {
			let caller = ensure_signed(origin.clone())?;
			let _ = Self::do_buy_ticket(caller, kind, index, selection);
			Ok(())
		}

		/// Start a lottery using the provided configuration.
		///
		/// This extrinsic must be called by the `ManagerOrigin`.
		///
		/// Parameters:
		///
		/// * `price`: The cost of a single ticket.
		/// * `length`: How long the lottery should run for starting at the current block.
		/// * `delay`: How long after the lottery end we should wait before picking a winner.
		/// * `repeat`: If the lottery should repeat when completed.
		#[pallet::call_index(1)]
		#[pallet::weight((10_100, DispatchClass::Normal, Pays::No))]
		pub fn start_lottery(
			origin: OriginFor<T>,
			kind: T::LotteryKind,
			price: BalanceOf<T>,
			length: T::BlockNumber,
			delay: T::BlockNumber,
			repeat: bool,
		) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			// Get the current index for the given kind of lottery
			let index = LotteryIndex::<T>::get(kind);
			// Attempt to update the lottery with the given kind
			Lottery::<T>::try_mutate(kind, index, |lottery| -> DispatchResult {
				// Ensure that the lottery is not already in progress
				ensure!(lottery.is_none(), Error::<T>::InProgress);
				// Check for arithmetic overflow when incrementing the index
				let new_index = index.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				// Get the current block number
				let start = frame_system::Pallet::<T>::block_number();
				// Set the lottery to the new configuration
				*lottery = Some(LotteryConfig {
					price,
					start,
					length,
					delay,
					repeat,
				});
				// Update the index for the given kind of lottery
				<LotteryIndex<T>>::mutate(kind, |v| {
					*v = new_index;
				});
				Ok(())
			})?;
			// Get the account for the lottery pot
			let lottery_account = Self::account_id();
			// If the lottery pot has no balance, deposit the minimum balance
			if T::Currency::total_balance(&lottery_account).is_zero() {
				T::Currency::deposit_creating(&lottery_account, T::Currency::minimum_balance());
			}
			// Deposit an event to indicate that the lottery has started
			Self::deposit_event(Event::<T>::LotteryStarted {kind, index});
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// The account ID of the lottery pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	/// Return the pot account and amount of money in the pot.
	/// The existential deposit is not part of the pot so lottery account never gets deleted.
	fn pot() -> (T::AccountId, BalanceOf<T>) {
		let account_id = Self::account_id();
		let balance =
			T::Currency::free_balance(&account_id).saturating_sub(T::Currency::minimum_balance());

		(account_id, balance)
	}

	/// Converts a vector of calls into a vector of call indices.
	// fn calls_to_indices(
	// 	calls: &[<T as Config>::RuntimeCall],
	// ) -> Result<BoundedVec<CallIndex, T::MaxCalls>, DispatchError> {
	// 	let mut indices = BoundedVec::<CallIndex, T::MaxCalls>::with_bounded_capacity(calls.len());
	// 	for c in calls.iter() {
	// 		let index = Self::call_to_index(c)?;
	// 		indices.try_push(index).map_err(|_| Error::<T>::TooManyCalls)?;
	// 	}
	// 	Ok(indices)
	// }

	/// Convert a call to it's call index by encoding the call and taking the first two bytes.
	// fn call_to_index(call: &<T as Config>::RuntimeCall) -> Result<CallIndex, DispatchError> {
	// 	let encoded_call = call.encode();
	// 	if encoded_call.len() < 2 {
	// 		return Err(Error::<T>::EncodingFailed.into())
	// 	}
	// 	Ok((encoded_call[0], encoded_call[1]))
	// }

	/// Logic for buying a ticket.
	fn do_buy_ticket(
		caller: T::AccountId,
		kind: T::LotteryKind,
		index: u32,
		selection: LotterySelection,
	) -> DispatchResult {
		// Check the call is valid lottery
		let config = Lottery::<T>::get(kind, index).ok_or(Error::<T>::NotConfigured)?;
		let block_number = frame_system::Pallet::<T>::block_number();
		ensure!(
			block_number < config.start.saturating_add(config.length),
			Error::<T>::AlreadyEnded
		);

		// Try to update the participant status
		Participants::<T>::try_mutate(
			(kind, index, selection),
			|v| -> Result<(), DispatchError> {
				ensure!(!v.is_full(), Error::<T>::TooManyParticipants);
				T::Currency::transfer(&caller, &Self::account_id(), config.price, KeepAlive)?;
				v.try_push(caller.clone())
					.map_err(|_| Error::<T>::TooManyParticipants)?;
				Ok(())
			},
		)?;

		Self::deposit_event(Event::<T>::TicketBought {
			kind,
			index,
			user: caller.clone(),
			selection,
		});
		Ok(())
	}

	/// Randomly choose a winning ticket and return the account that purchased it.
	/// The more tickets an account bought, the higher are its chances of winning.
	/// Returns `None` if there is no winner.
	// fn choose_account() -> Option<T::AccountId> {
	// 	match Self::choose_ticket(TicketsCount::<T>::get()) {
	// 		None => None,
	// 		Some(ticket) => Tickets::<T>::get(ticket),
	// 	}
	// }

	/// Randomly choose a winning ticket from among the total number of tickets.
	/// Returns `None` if there are no tickets.
	fn choose_ticket(total: u32) -> Option<u32> {
		if total == 0 {
			return None;
		}
		let mut random_number = Self::generate_random_number(0);

		// Best effort attempt to remove bias from modulus operator.
		for i in 1..T::MaxGenerateRandom::get() {
			if random_number < u32::MAX - u32::MAX % total {
				break;
			}

			random_number = Self::generate_random_number(i);
		}

		Some(random_number % total)
	}

	/// Generate a random number from a given seed.
	/// Note that there is potential bias introduced by using modulus operator.
	/// You should call this function with different seed values until the random
	/// number lies within `u32::MAX - u32::MAX % n`.
	/// TODO: deal with randomness freshness
	/// https://github.com/paritytech/substrate/issues/8311
	fn generate_random_number(seed: u32) -> u32 {
		let (random_seed, _) = T::Randomness::random(&(T::PalletId::get(), seed).encode());
		let random_number = <u32>::decode(&mut random_seed.as_ref())
			.expect("secure hashes should always be bigger than u32; qed");
		random_number
	}
}
