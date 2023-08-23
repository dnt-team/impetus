#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::MaxEncodedLen,
	traits::{
		tokens::{
			fungible::Mutate as MutateFungible,
			fungibles::{Create, Inspect, Mutate},
			nonfungibles_v2::{Inspect as NonFungiblesInspect, Transfer},
			AssetId, Balance as AssetBalance,
		},
		Currency, ExistenceRequirement, Get, Randomness, ReservableCurrency,
	},
	PalletId,
};
use sp_core::{crypto::KeyTypeId, U256};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ga!!");
use pallet_did::Provider;
pub mod crypto {
	use super::KEY_TYPE;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		MultiSignature, MultiSigner,
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

use frame_system::offchain::{AppCrypto, CreateSignedTransaction, Signer};
pub use pallet::*;
use scale_codec::{Decode, Encode};
use sp_runtime::traits::{AccountIdConversion, Saturating, Zero};
use sp_std::vec::Vec;
type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::{OptionQuery, *},
		traits::DefensiveTruncateFrom,
		BoundedBTreeSet,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::{fmt::Display, prelude::*};
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub type GiveawayName = BoundedVec<u8, ConstU32<128>>;
	pub type RequestId = BoundedVec<u8, ConstU32<64>>;
	pub type Results = BoundedVec<U256, ConstU32<32>>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<Call<Self>> + frame_system::Config + pallet_did::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The manager origin.
		type GiveawayOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

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

		/// The type used to describe the amount of fractions converted into assets.
		type AssetBalance: AssetBalance;

		/// The type used to identify the assets created during fractionalization.
		type AssetId: AssetId;
		/// Registry for the minted assets.
		type Assets: Create<Self::AccountId, AssetId = Self::AssetId, Balance = Self::AssetBalance>
			+ Mutate<Self::AccountId, AssetId = Self::AssetId, Balance = Self::AssetBalance>
			+ Inspect<Self::AccountId>;
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub enum KYCStatus {
		Tier0,
		Tier1,
	}

	impl Default for KYCStatus {
		fn default() -> Self {
			KYCStatus::Tier0
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub enum RandomType {
		// LocalChain,
		// Babe,
		Chainlink,
	}

	impl Default for RandomType {
		fn default() -> Self {
			RandomType::Chainlink
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub enum AssetType {
		FungibleToken,
		// NonFungibleToken,
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
	pub struct NftInfo<NftCollectionId, NftId> {
		collection_id: NftCollectionId,
		nft_id: NftId,
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
		pub asset_id: u32,
		pub amount: Balance,
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
	pub struct GiveawayConfig<BlockNumber, Balance, AccountId> {
		name: GiveawayName,
		start: BlockNumber,
		end: BlockNumber,
		kyc: KYCStatus,
		random_type: RandomType,
		// pay_fee: bool,
		// fee: Balance,
		creator: AccountId,
		asset_type: AssetType,
		token: Option<TokenInfo<Balance>>,
		// nft: Option<NftInfo<NftCollectonId, NftId>>,
		max_join: u32,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A lottery has not been configured.
		TooMany,
		TooManyParticipants,
		StartBlockInvalid,
		EndBlockInvalid,
		AlreadyJoined,
		CannotSetResultAgain,
		InvalidResult,
		InvalidRound,
		GiveawayEnded,
		GiveawayNotStarted,
		UserIsNotVerified,
	}

	#[pallet::storage]
	pub type PalletManager<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::storage]
	pub type RoundWinner<T: Config> = StorageMap<_, Twox64Concat, u32, T::AccountId>;

	#[pallet::storage]
	pub type GiveawayIndex<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn give_away)]
	pub type Giveaway<T: Config> = StorageMap<
		_,
		Twox64Concat,
		u32,
		GiveawayConfig<T::BlockNumber, BalanceOf<T>, T::AccountId>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn participants)]
	pub type Participants<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u32, Twox64Concat, u32, T::AccountId>;

	#[pallet::storage]
	pub type TotalParticipantByGiveaway<T: Config> = StorageMap<_, Twox64Concat, u32, u32, ValueQuery>;

	#[pallet::storage]
	pub type GiveawayToUser<T: Config> = StorageDoubleMap<_, Twox64Concat, u32, Twox64Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_giveaways_by_block)]
	pub type BlockToGiveaway<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, BoundedVec<u32, T::MaxSet>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_results_by_block)]
	pub type BlockToResults<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, (RequestId, Results), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		GiveawayCreated {
			index: u32,
		},
		Winner {
			index: u32,
			who: T::AccountId,
			status: bool,
		},
		Participated {
			index: u32,
			who: T::AccountId,
		},
		Results {
			block: T::BlockNumber,
			results: (RequestId, Results),
		},
		RewardClaimed {
			index: u32,
			winner: T::AccountId,
		},
	}

	// #[pallet::hooks]
	// impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
	// fn offchain_worker(block_number: T::BlockNumber) {
	// 	let signer = Signer::<T, T::AuthorityId>::all_accounts();
	// 	// The entry point of your code called by offchain worker
	// }

	// fn on_initialize(n: T::BlockNumber) -> Weight {
	// let giveaways = BlockToGiveaway::<T>::get(n);
	// for giveaway_index in giveaways.iter() {
	// 	let giveaway = Giveaway::<T>::get(giveaway_index);
	// 	let participants = Participants::<T>::get(giveaway_index);
	// 	let number: usize = Self::random_number(
	// 		giveaway_index.clone(),
	// 		participants.len().try_into().unwrap(),
	// 	)
	// 	.try_into()
	// 	.unwrap();
	// 	Self::deposit_event(Event::<T>::Winner {
	// 		index: *giveaway_index,
	// 		who: participants.into_iter().nth(number).unwrap(),
	// 	});
	// }
	// T::DbWeight::get().reads(2)
	// }
	// }

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
			asset_type: AssetType,
			token: Option<TokenInfo<BalanceOf<T>>>,
			max_join: u32,
		) -> DispatchResult {
			// Get user
			let who = ensure_signed(origin.clone())?;
			// Get the current index for the given kind of giveaway
			let block_number = frame_system::Pallet::<T>::block_number();
			ensure!(block_number < start_block, Error::<T>::StartBlockInvalid);
			ensure!(end_block > start_block, Error::<T>::EndBlockInvalid);
			let name_bounded: GiveawayName = GiveawayName::defensive_truncate_from(name.clone());
			let index = GiveawayIndex::<T>::get();
			let next_index = index.saturating_add(1);
			GiveawayIndex::<T>::put(next_index);
			// Attempt to update the lottery with the given kind
			Giveaway::<T>::insert(
				index,
				GiveawayConfig {
					name: name_bounded,
					start: start_block,
					end: end_block,
					kyc,
					random_type,
					// pay_fee,
					// fee,
					creator: who.clone(),
					asset_type: asset_type.clone(),
					token: token.clone(),
					// nft: nft.clone(),
					max_join,
				},
			);
			BlockToGiveaway::<T>::try_append(end_block, index).map_err(|_| Error::<T>::TooMany)?;
			// Get the account for the lottery pot
			let pallet_account = Self::account_id();

			T::Currency::deposit_creating(&pallet_account, T::PotDeposit::get());

			match asset_type {
				// AssetType::NonFungibleToken => {
				// 	let nft_info = nft.unwrap();
				// 	Self::transfer_nft(nft_info.collection_id, nft_info.nft_id, &pallet_account)?;
				// }
				AssetType::FungibleToken => {
					let token_info = token.unwrap();
					Self::transfer_asset(&who, &pallet_account, token_info.amount)?;
				}
			}
			// Deposit an event to indicate that the lottery has started
			Self::deposit_event(Event::<T>::GiveawayCreated { index });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn participate(origin: OriginFor<T>, index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let giveaways = Giveaway::<T>::get(index).unwrap();
			let current_block = frame_system::Pallet::<T>::block_number();
			ensure!(giveaways.end >= current_block, Error::<T>::GiveawayEnded);
			ensure!(
				giveaways.start <= current_block,
				Error::<T>::GiveawayNotStarted
			);
			if giveaways.kyc == KYCStatus::Tier1 {
				let provider: Provider =
					Provider::defensive_truncate_from("Fractal".as_bytes().to_vec());
				let value = pallet_did::ExternalIdAddress::<T>::get(&who, provider);
				ensure!(value.len() > 0, Error::<T>::UserIsNotVerified);
			}
			
				ensure!(!(GiveawayToUser::<T>::get(index, &who)) , Error::<T>::AlreadyJoined);
				let total = TotalParticipantByGiveaway::<T>::get(index);
				ensure!(
					total < giveaways.max_join,
					Error::<T>::TooManyParticipants
				);
				Participants::<T>::insert(index, total, &who);
				TotalParticipantByGiveaway::<T>::mutate(index, |value| {
					*value = value.saturating_add(1);
				});
			// if giveaways.pay_fee {
			// 	T::Currency::transfer(
			// 		&who,
			// 		&Self::account_id(),
			// 		giveaways.fee,
			// 		ExistenceRequirement::AllowDeath,
			// 	)?;
			// }
			Self::deposit_event(Event::<T>::Participated { index, who });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn set_block_result(
			origin: OriginFor<T>,
			block_number: T::BlockNumber,
			request_id: Vec<u8>,
			result: Vec<U256>,
		) -> DispatchResult {
			T::GiveawayOrigin::ensure_origin(origin)?;
			ensure!(
				!<BlockToResults<T>>::contains_key(block_number),
				Error::<T>::CannotSetResultAgain
			);
			let current_block = frame_system::Pallet::<T>::block_number();
			ensure!(block_number < current_block, Error::<T>::EndBlockInvalid);
			let giveaways = BlockToGiveaway::<T>::get(block_number);
			ensure!(giveaways.len() == result.len(), Error::<T>::InvalidResult);
			let results_bounded: Results = Results::defensive_truncate_from(result);
			let request_id_bounded: RequestId = RequestId::defensive_truncate_from(request_id);
			BlockToResults::<T>::insert(block_number, (&request_id_bounded, &results_bounded));
			for (giveaway, result_bounded) in giveaways.iter().zip(results_bounded.iter()) {
				let participants_len = TotalParticipantByGiveaway::<T>::get(giveaway);
				if participants_len != 0 {
					let mut index: u32 = (result_bounded.low_u32() % participants_len )
						.try_into()
						.unwrap();
					if index == 0 {
						index = participants_len;
					}
					let winner = Participants::<T>::get(giveaway, index.saturating_sub(1)).unwrap();
					RoundWinner::<T>::insert(giveaway, &winner);
					Self::deposit_event(Event::<T>::Winner {
						index: *giveaway,
						who: winner,
						status: true,
					});
				} else {
					let giveaway_info = Giveaway::<T>::get(giveaway).unwrap();
					RoundWinner::<T>::insert(giveaway, &giveaway_info.creator);
					Self::deposit_event(Event::<T>::Winner {
						index: *giveaway,
						who: giveaway_info.creator,
						status: false,
					});
				}
			}
			BlockToGiveaway::<T>::remove(block_number);
			Self::deposit_event(Event::<T>::Results {
				block: block_number,
				results: (request_id_bounded, results_bounded),
			});
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight((10_100, DispatchClass::Normal))]
		pub fn claim_reward(origin: OriginFor<T>, round: u32) -> DispatchResult {
			_ = ensure_signed(origin)?;
			let round_winner = RoundWinner::<T>::get(round);
			let giveaway = Giveaway::<T>::get(round);
			ensure!(
				(giveaway.is_some() && round_winner.is_some()),
				Error::<T>::InvalidRound
			);
			let giveaway = giveaway.unwrap();
			let round_winner = round_winner.unwrap();
			let pallet_account = Self::account_id();
			match giveaway.asset_type {
				// AssetType::NonFungibleToken => {
				// 	let nft_info = giveaway.nft.unwrap();
				// 	Self::transfer_nft(nft_info.collection_id, nft_info.nft_id, &round_winner)?;
				// }
				AssetType::FungibleToken => {
					let token_info = giveaway.token.unwrap();
					Self::transfer_asset(&pallet_account, &round_winner, token_info.amount)?;
				}
			}
			Self::deposit_event(Event::<T>::RewardClaimed {
				index: round,
				winner: round_winner,
			});
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

	fn transfer_nft(
		nft_collection_id: T::NftCollectionId,
		nft_id: T::NftId,
		account: &T::AccountId,
	) -> DispatchResult {
		T::Nfts::transfer(&nft_collection_id, &nft_id, account)
	}

	fn transfer_asset(
		from: &T::AccountId,
		to: &T::AccountId,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		T::Currency::transfer(from, to, amount, ExistenceRequirement::KeepAlive)
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
