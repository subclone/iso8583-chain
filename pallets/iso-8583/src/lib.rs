#![cfg_attr(not(feature = "std"), no_std)]

/// This pallet serves as a synchronization point for PCIDSS compliant oracle gateway.
///
/// The oracle gateway is a trusted third party that will submit approved and applied ISO-8583
/// messages to this pallet. This pallet will then perform the necessary actions to sync the
/// offchain ledger with the onchain ledger.
mod types;

pub use pallet::*;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::{ValueQuery, *},
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
		/// Maximum transaction size in bytes
		#[pallet::constant]
		type MaxTransactionSize: Get<u32>;
	}

	/// Stored transactions
	#[pallet::storage]
	#[pallet::getter(fn transactions)]
	pub type Transactions<T> = StorageMap<_, Blake2_128Concat, Hash, TransactionOf<T>, ValueQuery>;

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
	pub enum Error<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}
	}
}
