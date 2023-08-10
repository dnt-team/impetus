#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::MaxEncodedLen,
	traits::{Currency, ExistenceRequirement, Get, Randomness, ReservableCurrency},
	PalletId,
};
pub use pallet::*;
use sp_std::vec::Vec;
use sp_runtime::{
	traits::{AccountIdConversion, Saturating, Zero},
	SaturatedConversion,
};
type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, traits::DefensiveTruncateFrom};
	use frame_system::pallet_prelude::*;
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_did::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The manager origin.
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		#[pallet::constant]
		type PotDeposit: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxSet: Get<u32>;
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub enum KYCStatus {
		Tier0,
		Tier1,
		Tier2,
	}

	impl Default for KYCStatus {
		fn default() -> Self {
			KYCStatus::Tier0
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub enum RandomType {
		LocalChain,
		Babe,
		ChainLink,
	}

	impl Default for RandomType {
		fn default() -> Self {
			RandomType::LocalChain
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub enum AssetType {
		FungibleToken,
		NonFungibleToken,
	}

	impl Default for AssetType {
		fn default() -> Self {
			AssetType::FungibleToken
		}
	}

	#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct NftInfo {
		collection_id: u32,
		item_id: u32,
	}


	#[derive(
		Encode,
		Decode,
		Default,
		Eq,
		PartialEq,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen
	)]
	pub struct GiveAwayConfig<BlockNumber, Balance, AccountId> {
		start: BlockNumber,
		length: BlockNumber,
		delay: BlockNumber,
		kyc: KYCStatus,
		random_type: RandomType,
		pay_fee: bool,
		fee: Balance,
		user_must_claim: bool,
		fund_back_block: BlockNumber,
		creator: AccountId,
		asset_type: AssetType,
		amount: Balance,
		nft: NftInfo,
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::storage]
	pub type PalletManager<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool, ValueQuery>;
	
	#[pallet::storage]
	pub type GiveAwayIndex<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn give_away)]
	pub type GiveAway<T: Config> = StorageMap<
		_,
		Twox64Concat,
		u32,
		GiveAwayConfig<T::BlockNumber, BalanceOf<T>, T::AccountId>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_give_aways_by_block)]
	pub type BlockToGiveAway<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, BoundedVec<u32, T::MaxSet>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {

	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((10_100, DispatchClass::Normal, Pays::No))]
		pub fn create_give_away(
			origin: OriginFor<T>,
			length: T::BlockNumber,
			start_delay: T::BlockNumber,
			delay: T::BlockNumber,
			kyc: KYCStatus,
			random_type: RandomType,
			pay_fee: bool,
			fee: BalanceOf<T>,
			user_must_claim: bool,
		) -> DispatchResult {
			// Get user
			let who = ensure_signed(origin.clone())?;
			// Get the current index for the given kind of giveaway
			let index = GiveAwayIndex::<T>::get();
			// Attempt to update the lottery with the given kind
			let start = frame_system::Pallet::<T>::block_number();
			GiveAway::<T>::insert(index, GiveAwayConfig {
				start,
				length,
				delay,
				kyc,
				random_type,
				pay_fee,
				fee,
				user_must_claim,
				fund_back_block: start.saturating_add(delay),
				creator: who,
			});

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
}
