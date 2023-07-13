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
// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod tests;
// pub mod weights;

use scale_codec::{Decode, Encode};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::MaxEncodedLen,
	traits::{Currency, ExistenceRequirement, Get, Randomness, ReservableCurrency},
	PalletId,
};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, Saturating, Zero},
	SaturatedConversion,
};
use sp_std::prelude::*;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, BoundedBTreeSet};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The pallet's config trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The Lottery's pallet id
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The manager origin.
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

		#[pallet::constant]
		type PotDeposit: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxSet: Get<u32>;

		#[pallet::constant]
		type MaxUserRewardPerRound: Get<u32>;
	}

	#[derive(Encode, Decode, Default, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct LotteryConfig<BlockNumber, Balance> {
		/// Min Price per entry.
		min_price: Balance,
		/// Starting block of the lottery.
		start: BlockNumber,
		/// Length of the lottery (start + length = end).
		length: BlockNumber,
		/// Delay for choosing the winner of the lottery. (start + length + delay = payout).
		/// Randomness in the "payout" block will be used to determine the winner.
		delay: BlockNumber,
		rate: u8,
		/// Whether this lottery will repeat after it completes.
		repeat: bool,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A lottery has been started!
		RoundStarted {
			round: u32,
		},

		/// A ticket has been bought!
		TicketBought {
			round: u32,
			who: T::AccountId,
			amount: BalanceOf<T>,
			number: u8,
		},

		RandomNumberGenerated {
			round: u32,
			number: u8,
		},

		RewardClaimed {
			round: u32,
			who: T::AccountId,
			amount: BalanceOf<T>,
		},

		RewardClaimedFailed {
			round: u32,
			who: T::AccountId,
			amount: BalanceOf<T>,
			error: sp_runtime::DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A lottery has not been configured.
		NotConfigured,
		/// A lottery is already in progress.
		InProgress,
		/// A lottery has already ended.
		CannotSetRate,
		/// A lottery has already ended.
		AlreadyEnded,
		/// The call is not valid for an open lottery.
		InvalidCall,
		/// You are already participating in the lottery with this call.
		InvalidNumber,
		TooManyParticipants,
	}

	#[pallet::storage]
	pub(crate) type Round<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The configuration for the current lottery.
	#[pallet::storage]
	pub(crate) type Lottery<T: Config> =
		StorageMap<_, Twox64Concat, u32, LotteryConfig<T::BlockNumber, BalanceOf<T>>>;

	#[pallet::storage]
	pub(crate) type Participants<T: Config> = StorageMap<
		_,
		Twox64Concat,
		(u32, u8),
		BoundedBTreeSet<T::AccountId, T::MaxSet>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub(crate) type Winners<T: Config> = StorageMap<
		_,
		Twox64Concat,
		u32,
		BoundedBTreeSet<T::AccountId, T::MaxSet>,
		ValueQuery,
	>;

	/// Total number of tickets sold.
	#[pallet::storage]
	pub(crate) type UserPredictionValue<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		u32,
		Twox64Concat,
		(T::AccountId, u8),
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let round = Round::<T>::get();
			let lottery = Lottery::<T>::get(round);
			if let Some(config) = lottery {
				let payout_block = config
					.start
					.saturating_add(config.length)
					.saturating_add(config.delay);
				if payout_block <= n {
					let number = Self::random_number(round);
					Self::deposit_event(Event::<T>::RandomNumberGenerated { round, number });
					let winners_from_participants = Participants::<T>::get((round, number));
					Winners::<T>::insert(round, winners_from_participants);
					Participants::<T>::remove((round, number));
					let next_round = round.saturating_add(1);
					Round::<T>::put(next_round);
					if config.repeat {
						Lottery::<T>::insert(
							next_round,
							LotteryConfig {
								min_price: config.min_price,
								start: n,
								length: config.length,
								delay: config.delay,
								rate: config.rate,
								repeat: config.repeat,
							},
						);
						Self::deposit_event(Event::<T>::RoundStarted { round: next_round });
					}
				}
			}
			T::DbWeight::get().reads(1)
		}
	}

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
			number: u8,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let caller = ensure_signed(origin.clone())?;
			ensure!(number < 100, Error::<T>::InvalidNumber);
			let round = Round::<T>::get();
			let config = Lottery::<T>::get(round).ok_or(Error::<T>::NotConfigured)?;
			let block_number = frame_system::Pallet::<T>::block_number();
			ensure!(
				block_number <= config.start.saturating_add(config.length),
				Error::<T>::AlreadyEnded
			);
			Participants::<T>::try_mutate((round, number), |participants| -> DispatchResult {
				let check = participants.contains(&caller);
				match check {
					false => {
						participants
							.try_insert(caller.clone())
							.map_err(|_| Error::<T>::TooManyParticipants)?;
						UserPredictionValue::<T>::insert(round, (&caller, number), amount)
					}
					true => UserPredictionValue::<T>::mutate(round, (&caller, number), |v| {
						*v = v.saturating_add(amount)
					}),
				}
				T::Currency::transfer(
					&caller,
					&Self::account_id(),
					amount,
					ExistenceRequirement::KeepAlive,
				)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::TicketBought {
				round,
				who: caller.clone(),
				amount,
				number,
			});
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
			min_price: BalanceOf<T>,
			length: T::BlockNumber,
			delay: T::BlockNumber,
			rate: u8,
			repeat: bool,
		) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			// Get the current index for the given kind of lottery
			let round = Round::<T>::get();
			// Attempt to update the lottery with the given kind
			Lottery::<T>::try_mutate(round, |lottery| -> DispatchResult {
				ensure!(lottery.is_none(), Error::<T>::InProgress);
				ensure!(rate < 99, Error::<T>::CannotSetRate);
				let start = frame_system::Pallet::<T>::block_number();
				// Use new_index to more easily track everything with the current state.
				*lottery = Some(LotteryConfig {
					min_price,
					start,
					length,
					delay,
					repeat,
					rate,
				});
				Ok(())
			})?;
			// Get the account for the lottery pot
			let lottery_account = Self::account_id();
			// If the lottery pot has no balance, deposit the minimum balance
			if T::Currency::total_balance(&lottery_account).is_zero() {
				T::Currency::deposit_creating(&lottery_account, T::PotDeposit::get());
			}
			// Deposit an event to indicate that the lottery has started
			Self::deposit_event(Event::<T>::RoundStarted { round });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight((10_100, DispatchClass::Normal, Pays::No))]
		pub fn claim_reward(
			origin: OriginFor<T>,
			who: T::AccountId,
			round: u32,
			number: u8,
		) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			<Winners<T>>::try_mutate(round, |winners| -> DispatchResult {
				ensure!(winners.contains(&who), Error::<T>::InvalidCall);
				let amount = <UserPredictionValue<T>>::get(round, (&who, number));
				let lottery_config = <Lottery<T>>::get(round).ok_or(Error::<T>::NotConfigured)?;
				let reward = amount.saturating_mul(lottery_config.rate.saturated_into());
				match T::Currency::transfer(
					&Self::account_id(),
					&who,
					reward,
					ExistenceRequirement::KeepAlive,
				) {
					Ok(_) => {
						winners.remove(&who);
						Self::deposit_event(Event::<T>::RewardClaimed {
							round,
							who,
							amount: reward,
						});
					}
					Err(error) => {
						Self::deposit_event(Event::<T>::RewardClaimedFailed {
							round,
							who,
							amount: reward,
							error,
						});
					}
				};
				Ok(())
			})?;
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
	// fn pot() -> (T::AccountId, BalanceOf<T>) {
	// 	let account_id = Self::account_id();
	// 	let balance =
	// 		T::Currency::free_balance(&account_id).saturating_sub(T::Currency::minimum_balance());

	// 	(account_id, balance)
	// }

	/// Randomly choose a winning ticket and return the account that purchased it.
	/// The more tickets an account bought, the higher are its chances of winning.
	/// Returns `None` if there is no winner.
	fn random_number(index: u32) -> u8 {
		// Get the current block's random seed
		let random_number = Self::generate_random_number(index);
		let random_number = (random_number % 100) as u8;
		random_number
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
