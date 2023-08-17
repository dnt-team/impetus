#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::MaxEncodedLen,
	traits::{
		tokens::nonfungibles_v2::{Inspect as NonFungiblesInspect, Transfer},
		Currency, ExistenceRequirement, Get, Randomness, ReservableCurrency,
	},
	PalletId,
};
use sp_core::{crypto::KeyTypeId};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ga!!");

pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify, MultiSignature, MultiSigner
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	// implemented for runtime
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
	type RuntimeAppPublic = Public;
	type GenericSignature = sp_core::sr25519::Signature;
	type GenericPublic = sp_core::sr25519::Public;
	}
}

pub use pallet::*;
use scale_codec::{Decode, Encode};
use sp_runtime::traits::{AccountIdConversion, Saturating, Zero};
use frame_system::offchain::{AppCrypto, CreateSignedTransaction, Signer};
use sp_std::vec::Vec;
type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, traits::DefensiveTruncateFrom, BoundedBTreeSet};
	use frame_system::pallet_prelude::*;
	use sp_std::{fmt::Display, prelude::*};
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub type GiveAwayName = BoundedVec<u8, ConstU32<128>>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config + pallet_did::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The manager origin.
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
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
		/// Identifier for the collection of NFT.
		type NftCollectionId: Member + Parameter + MaxEncodedLen + Copy + Display;

		/// The type used to identify an NFT within a collection.
		type NftId: Member + Parameter + MaxEncodedLen + Copy + Display;
		/// Registry for minted NFTs.
		type Nfts: NonFungiblesInspect<
				Self::AccountId,
				ItemId = Self::NftId,
				CollectionId = Self::NftCollectionId,
			> + Transfer<Self::AccountId>;
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
		Both,
	}

	impl Default for AssetType {
		fn default() -> Self {
			AssetType::FungibleToken
		}
	}

	#[derive(
		Encode,
		Decode,
		Default,
		Clone,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen
	)]
	pub struct NftInfo {
		collection_id: u32,
		item_id: u32,
	}

	#[derive(
		Encode,
		Decode,
		Default,
		Clone,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen
	)]
	pub struct TokenInfo<Balance> {
		asset_id: u32,
		amount: Balance,
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
		name: GiveAwayName,
		start: BlockNumber,
		end: BlockNumber,
		kyc: KYCStatus,
		random_type: RandomType,
		pay_fee: bool,
		fee: Balance,
		creator: AccountId,
		asset_type: AssetType,
		token: Option<TokenInfo<Balance>>,
		nft: Option<NftInfo>,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A lottery has not been configured.
		TooManyParticipants,
		AlreadyJoined,
	}

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
	#[pallet::getter(fn participants)]
	pub type Participants<T: Config> =
		StorageMap<_, Twox64Concat, u32, BoundedBTreeSet<T::AccountId, T::MaxSet>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_give_aways_by_block)]
	pub type BlockToGiveAway<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, BoundedVec<u32, T::MaxSet>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		GiveAwayCreated { index: u32 },
		Winner { index: u32, who: T::AccountId },
		Participated { index: u32, who: T::AccountId },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			let signer = Signer::<T, T::AuthorityId>::all_accounts();
			// The entry point of your code called by offchain worker
		}

		fn on_initialize(n: T::BlockNumber) -> Weight {
			let giveaways = BlockToGiveAway::<T>::get(n);
			for giveaway_index in giveaways.iter() {
				let giveaway = GiveAway::<T>::get(giveaway_index);
				let participants = Participants::<T>::get(giveaway_index);
				let number: usize = Self::random_number(
					giveaway_index.clone(),
					participants.len().try_into().unwrap(),
				)
				.try_into()
				.unwrap();
				Self::deposit_event(Event::<T>::Winner {
					index: *giveaway_index,
					who: participants.into_iter().nth(number).unwrap(),
				});
			}
			T::DbWeight::get().reads(2)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((10_100, DispatchClass::Normal, Pays::No))]
		pub fn create_give_away(
			origin: OriginFor<T>,
			name: Vec<u8>,
			start_block: T::BlockNumber,
			end_block: T::BlockNumber,
			kyc: KYCStatus,
			random_type: RandomType,
			pay_fee: bool,
			fee: BalanceOf<T>,
			asset_type: AssetType,
			token: Option<TokenInfo<BalanceOf<T>>>,
			nft: Option<NftInfo>,
		) -> DispatchResult {
			// Get user
			let who = ensure_signed(origin.clone())?;
			// Get the current index for the given kind of giveaway
			let name_bounded: GiveAwayName = GiveAwayName::defensive_truncate_from(name.clone());
			let index = GiveAwayIndex::<T>::get();
			// Attempt to update the lottery with the given kind
			GiveAway::<T>::insert(
				index,
				GiveAwayConfig {
					name: name_bounded,
					start: start_block,
					end: end_block,
					kyc,
					random_type,
					pay_fee,
					fee,
					creator: who.clone(),
					asset_type,
					token,
					nft,
				},
			);
			// Get the account for the lottery pot
			let lottery_account = Self::account_id();

			T::Currency::deposit_creating(&lottery_account, T::PotDeposit::get());

			// match asset_type {
			// 	AssetType::NonFungibleToken => {
			// 		T::Nfts::transfer(
			// 			&nft.unwrap().collection_id.into(),
			// 			&nft.unwrap().item_id.into(),
			// 			&Self::account_id(),
			// 		);
			// 	}
			// }

			// Deposit an event to indicate that the lottery has started
			Self::deposit_event(Event::<T>::GiveAwayCreated { index });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn participate(origin: OriginFor<T>, index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Participants::<T>::try_mutate(index, |participants| -> DispatchResult {
				ensure!(!participants.contains(&who), Error::<T>::AlreadyJoined);
				participants
					.try_insert(who.clone())
					.map_err(|_| Error::<T>::TooManyParticipants)?;
				Ok(())
			})?;
			let giveaways = GiveAway::<T>::get(index).unwrap();

			if giveaways.pay_fee {
				T::Currency::transfer(
					&who,
					&Self::account_id(),
					giveaways.fee,
					ExistenceRequirement::AllowDeath,
				)?;
			}

			Self::deposit_event(Event::<T>::Participated { index, who });
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

	fn random_number(index: u32, length: u32) -> u32 {
		// Get the current block's random seed
		let random_number = Self::generate_random_number(index);
		let random_number = random_number % length;
		random_number
	}

	fn generate_random_number(seed: u32) -> u32 {
		let (random_seed, _) = T::Randomness::random(&(T::PalletId::get(), seed).encode());
		let random_number = <u32>::decode(&mut random_seed.as_ref())
			.expect("secure hashes should always be bigger than u32; qed");
		random_number
	}
}
