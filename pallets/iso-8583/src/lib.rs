#![cfg_attr(not(feature = "std"), no_std)]
/// This pallet serves as a synchronization point for PCIDSS compliant oracle gateway.
///
/// The oracle gateway is a trusted third party that will submit approved and applied ISO-8583
/// messages to this pallet. This pallet will then perform the necessary actions to sync the
/// offchain ledger with the onchain ledger.
mod impls;
mod traits;
mod types;

use frame_support::{
	pallet_prelude::{ValueQuery, *},
	sp_runtime::BoundedVec,
	traits::{Currency, ReservableCurrency},
	Blake2_128Concat,
};
pub use pallet::*;
use traits::*;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

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

	/// Accounts registered in the oracle
	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub type Accounts<T> = StorageMap<_, Blake2_128Concat, AccountIdOf<T>, ()>;

	/// Allowances for accounts
	///
	/// `(From, Spender) => Allowance`
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
		InitiateIso8583 { from: T::AccountId, to: T::AccountId, amount: BalanceOf<T> },
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
		/// Settle a batch of transactions
		///
		/// This function is used by the oracle gateway to submit a batch of transactions to be
		/// settled on-chain. The oracle gateway will submit the finality of the transactions
		/// after they have been applied.
		///
		/// It uses `transfer_from` of ERC20-R interface to transfer tokens from the source
		/// account to the destination account.
		///
		/// # Errors
		#[pallet::weight(T::DbWeight::get().writes(0))]
		#[pallet::call_index(0)]
		pub fn submit_finalities(
			origin: OriginFor<T>,
			transactions: BoundedVec<TransactionOf<T>, T::MaxBatchSize>,
		) -> DispatchResult {
			let oracle = T::OracleGatewayOrigin::ensure_origin(origin)?;

			for transaction in transactions {
				let from = Self::ensure_registered(&transaction.from);
				let to = Self::ensure_registered(&transaction.to);

				Self::transfer_from(&origin, &from, &to, transaction.amount)?;
			}

			Ok(())
		}

		/// Initiate a transaction
		///
		/// This function is used by the bank account owners to initiate a transaction with
		/// their registered on-chain `AccountId`.
		///
		/// # Errors
		///
		/// Transfer will fail if source and destination accounts are not registered in the oracle.
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(1)]
		pub fn initiate_transfer(
			origin: OriginFor<T>,
			to: AccountIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			ensure!(T::Currency::free_balance(&from) >= amount, Error::<T>::InsufficientAllowance);

			// if account is already registered, both in the oracle and on-chain, then we can
			// settle the transaction immediately.
			if Accounts::<T>::contains_key(&to) && Accounts::<T>::contains_key(&to) {
				Self::transfer(&from, &to, amount)?;
			}

			Self::deposit_event(Event::<T>::InitiateIso8583 {
				from: from.clone(),
				to: to.clone(),
				amount: amount.clone(),
			});

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

impl<T: Config> Pallet<T> {
	fn ensure_registered(account: &AccountIdOf<T>) -> &AccountIdOf<T> {
		if Accounts::<T>::contains_key(account) {
			account
		} else {
			Accounts::<T>::insert(account, ());
			account
		}
	}
}
