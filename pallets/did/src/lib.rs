#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;
use sp_std::vec::Vec;
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, traits::DefensiveTruncateFrom};
	use frame_system::pallet_prelude::*;
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The manager origin.
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
	}

	pub type ExternalId = BoundedVec<u8, ConstU32<128>>;
	pub type Provider = BoundedVec<u8, ConstU32<32>>;
	pub type ListName = BoundedVec<u8, ConstU32<32>>;

	#[pallet::error]
	pub enum Error<T> {
		NotAllowedToRemove,
		NotAllowedToMutate,
		InvalidOrigin,
	}

	#[pallet::storage]
	#[pallet::getter(fn did_manager)]
	pub type PalletManager<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn external_id)]
	pub type ExternalIdAddress<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		Provider,
		ExternalId,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_list)]
	pub type UserList<T: Config> =
		StorageDoubleMap<_, Twox64Concat, ListName, Twox64Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AddedUserAddress {
			who: T::AccountId,
			provider: Provider,
		},
		RemovedUserAddress {
			who: T::AccountId,
			provider: Provider,
		},
		AddedUserToList {
			who: T::AccountId,
			list_name: ListName,
		},
		RemovedUserFromList {
			who: T::AccountId,
			list_name: ListName,
		},
		AddedManager {
			manager: T::AccountId,
		},
		RemovedManager {
			manager: T::AccountId,
		},
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub managers: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				managers: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for manager in &self.managers {
				<PalletManager<T>>::insert(manager, true);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn add_user_address(
			origin: OriginFor<T>,
			user: T::AccountId,
			provider: Vec<u8>,
			external_id: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let is_manager = <PalletManager<T>>::get(who);
			ensure!(is_manager, Error::<T>::InvalidOrigin);
			let provider_bounded: Provider = Provider::defensive_truncate_from(provider.clone());
			let external_id_bounded: ExternalId =
				ExternalId::defensive_truncate_from(external_id.clone());
			<ExternalIdAddress<T>>::insert(&user, &provider_bounded, external_id_bounded);
			Self::deposit_event(Event::AddedUserAddress {
				who: user,
				provider: provider_bounded,
			});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn remove_user_address(
			origin: OriginFor<T>,
			user: T::AccountId,
			provider: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let is_manager = <PalletManager<T>>::get(who);
			ensure!(is_manager, Error::<T>::InvalidOrigin);
			let provider_bounded: Provider = Provider::defensive_truncate_from(provider.clone());
			<ExternalIdAddress<T>>::remove(&user, &provider_bounded);
			Self::deposit_event(Event::RemovedUserAddress {
				who: user,
				provider: provider_bounded,
			});
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn add_user_to_list(
			origin: OriginFor<T>,
			list_name: Vec<u8>,
			user: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let is_manager = <PalletManager<T>>::get(who);
			ensure!(is_manager, Error::<T>::InvalidOrigin);
			let list_name_bounded: ListName = ListName::defensive_truncate_from(list_name.clone());
			<UserList<T>>::insert(&list_name_bounded, &user, true);
			Self::deposit_event(Event::AddedUserToList {
				who: user,
				list_name: list_name_bounded,
			});
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn remove_user_from_list(
			origin: OriginFor<T>,
			list_name: Vec<u8>,
			user: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let is_manager = <PalletManager<T>>::get(who);
			ensure!(is_manager, Error::<T>::InvalidOrigin);
			let list_name_bounded: ListName = ListName::defensive_truncate_from(list_name.clone());
			<UserList<T>>::remove(&list_name_bounded, &user);
			Self::deposit_event(Event::RemovedUserFromList {
				who: user,
				list_name: list_name_bounded,
			});
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn add_did_manager(origin: OriginFor<T>, manager: T::AccountId) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			<PalletManager<T>>::insert(&manager, true);
			Self::deposit_event(Event::AddedManager { manager });
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn remove_did_manager(origin: OriginFor<T>, manager: T::AccountId) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			<PalletManager<T>>::remove(&manager);
			Self::deposit_event(Event::RemovedManager { manager });
			Ok(())
		}
	}
}
