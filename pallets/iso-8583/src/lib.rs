#![cfg_attr(not(feature = "std"), no_std)]
/// This pallet serves as a synchronization point for PCIDSS compliant oracle gateway.
///
/// The oracle gateway is a trusted third party that will submit approved and applied ISO-8583
/// messages to this pallet. This pallet will then perform the necessary actions to sync the
/// offchain ledger with the onchain ledger.
mod impls;
mod traits;
mod types;

pub use pallet::*;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::{OptionQuery, ValueQuery, *},
		sp_runtime::BoundedVec,
		traits::ReservableCurrency,
		Blake2_128Concat,
	};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Pallet configuration
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Currency type to control the monetary system.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// Oracle gateway account
		type OracleGatewayOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Maximum transaction size in bytes
		#[pallet::constant]
		type MaxTransactionSize: Get<u32>;
		/// Maximum transaction batch size
		#[pallet::constant]
		type MaxBatchSize: Get<u32>;
		/// Maximum string size
		#[pallet::constant]
		type MaxStringSize: Get<u32>;
	}

	/// Stored transactions
	#[pallet::storage]
	#[pallet::getter(fn transactions)]
	pub type Transactions<T> = StorageMap<_, Blake2_128Concat, Hash, TransactionOf<T>, OptionQuery>;

	/// Bank account to `AccountId` mapping
	#[pallet::storage]
	#[pallet::getter(fn bank_accounts)]
	pub type BankAccounts<T> =
		StorageMap<_, Blake2_128Concat, BankAccount<T::MaxStringSize>, AccountIdOf<T>, OptionQuery>;

	/// Allowances for accounts
	#[pallet::storage]
	#[pallet::getter(fn allowances)]
	pub type Allowances<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		AccountIdOf<T>,
		BalanceOf<T>,
		ValueQuery,
	>;

	/// Events of this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New transaction batch submitted
		TransactionBatchSubmitted { count: u32 },
		/// Transaction submitted
		TransactionSubmitted {
			hash: Hash,
			from: AccountIdOf<T>,
			to: AccountIdOf<T>,
			amount: BalanceOf<T>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient allowance
		InsufficientAllowance,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Ask for authorization
		///
		/// This is similar to ISO-8583's `0200` message type. When user wants to perform

		/// Settle a batch of transactions
		///
		/// This function is used by the oracle gateway to submit a batch of transactions to be
		/// settled onchain.
		///
		/// # Errors
		///
		/// - `
		#[pallet::weight(T::DbWeight::get().writes(transactions.len() as u64))]
		#[pallet::call_index(0)]
		pub fn submit_finality(
			origin: OriginFor<T>,
			transactions: BoundedVec<TransactionOf<T>, T::MaxBatchSize>,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain worker
		///
		/// This function is executed by the offchain worker and is used to validate ISO-8583
		/// messages submitted by the oracle gateway.
		fn offchain_worker(_now: BlockNumberFor<T>) {}
	}
}
