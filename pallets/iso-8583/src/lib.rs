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
	traits::{BuildGenesisConfig, Currency, ReservableCurrency},
};
use frame_system::{ensure_signed, pallet_prelude::OriginFor};
use sp_runtime::Saturating;

pub use pallet::*;
use traits::*;
use types::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	use frame_support::weights::WeightToFee;
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
		/// PalletAccount origin
		#[pallet::constant]
		type PalletAccount: Get<Self::AccountId>;
		/// Maximum string size
		#[pallet::constant]
		type MaxStringSize: Get<u32>;
		/// Weight to fee conversion algorithm
		type WeightToFee: WeightToFee<Balance = BalanceOf<Self>>;
	}

	/// Accounts registered in the oracle
	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub type Accounts<T> = StorageMap<_, Blake2_128Concat, AccountIdOf<T>, ()>;

	/// Allowances for accounts
	///
	/// `(From, Spender) => Allowance`
	#[pallet::storage]
	#[pallet::getter(fn allowance)]
	pub type Allowances<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		AccountIdOf<T>,
		BalanceOf<T>,
		ValueQuery,
	>;

	/// Registered oracle accounts
	#[pallet::storage]
	#[pallet::getter(fn oracle_accounts)]
	pub type OracleAccounts<T> = StorageMap<_, Blake2_128Concat, AccountIdOf<T>, ()>;

	/// Events of this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Initiate transfer of funds
		InitiateTransfer { from: T::AccountId, to: T::AccountId, amount: BalanceOf<T> },
		/// Initiate revert transaction
		InitiateRevert { who: T::AccountId, hash: T::Hash },
		/// Deduct funds from account: slashing, transaction fee, etc.
		DeductFunds { who: T::AccountId, amount: BalanceOf<T> },
		/// Processed transaction by the oracle gateway
		ProcessedTransaction { event_id: EventId, status: ISO8583Status },
		/// Account was registered
		/// This event is emitted when an account is registered by the oracle/s;
		AccountRegistered { account: T::AccountId, initial_balance: BalanceOf<T> },
		/// Allowance given
		Allowance { from: T::AccountId, to: T::AccountId, amount: BalanceOf<T> },
		/// Account removed
		AccountRemoved { account: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient allowance
		InsufficientAllowance,
		/// Allowance exceeds balance
		AllowanceExceedsBalance,
		/// Source account is not registered
		SourceNotRegistered,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Settle a batch of transactions
		///
		/// This function is used by the oracle gateway to submit a final transaction to be
		/// settled on-chain. The oracle gateway will submit the finality of the transactions
		/// after they have been applied to the offchain ledger.
		///
		/// It uses `transfer_from` of ERC20-R interface to transfer tokens from the source
		/// account to the destination account.
		///
		/// # Errors
		#[pallet::weight(T::DbWeight::get().writes(0))]
		#[pallet::call_index(0)]
		pub fn submit_finality(
			origin: OriginFor<T>,
			transaction: FinalisedTransactionOf<T>,
		) -> DispatchResult {
			Self::ensure_oracle(origin)?;

			Self::process_finalised_transaction(&transaction)?;

			Self::deposit_event(Event::<T>::ProcessedTransaction {
				event_id: transaction.event_id,
				status: transaction.status,
			});

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
			from: AccountIdOf<T>,
			to: AccountIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(Accounts::<T>::contains_key(&from), Error::<T>::SourceNotRegistered);
			ensure!(T::Currency::free_balance(&from) >= amount, Error::<T>::InsufficientAllowance);

			if who != from {
				ensure!(
					Allowances::<T>::get(&from, &who) >= amount,
					Error::<T>::InsufficientAllowance
				);
			}

			// self-transfer is no-op
			if from == to {
				return Ok(());
			}

			// lock funds
			T::Currency::reserve(&from, amount)?;

			Self::deposit_event(Event::<T>::InitiateTransfer {
				from: from.clone(),
				to: to.clone(),
				amount: amount.clone(),
			});

			Ok(())
		}

		/// Initiate a revert transaction
		///
		/// This function is used by the bank account owners to initiate a revert transaction with
		/// their registered on-chain `AccountId`.
		///
		/// # Errors
		///
		/// Extrinsic is infallible.
		#[pallet::weight(T::DbWeight::get().writes(0))]
		#[pallet::call_index(2)]
		pub fn initiate_revert(origin: OriginFor<T>, hash: T::Hash) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::deposit_event(Event::<T>::InitiateRevert { who, hash: hash.clone() });

			Ok(())
		}

		/// Give allowance to an account
		///
		/// Any account can give allowance to any other account.
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(3)]
		pub fn approve(
			origin: OriginFor<T>,
			spender: AccountIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			// ensure owner has enough balance
			ensure!(
				T::Currency::free_balance(&owner) >= value,
				Error::<T>::AllowanceExceedsBalance
			);

			<Self as ERC20R<AccountIdOf<T>, BalanceOf<T>>>::approve(&owner, &spender, value)?;

			Self::deposit_event(Event::<T>::Allowance { from: owner, to: spender, amount: value });

			Ok(())
		}

		/// Register an account
		///
		/// This function is used by the oracle gateway to register an account.
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(4)]
		pub fn register(
			origin: OriginFor<T>,
			account: AccountIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			Self::ensure_oracle(origin)?;

			// register account and mint initial balance
			Accounts::<T>::insert(&account, ());

			let _ = T::Currency::deposit_creating(&account, amount);

			Self::deposit_event(Event::<T>::AccountRegistered { account, initial_balance: amount });

			Ok(())
		}

		/// Remove an account
		///
		/// This function is used by the oracle gateway to remove an account. Oracle can remove
		/// accounts that are not honest or have been compromised.
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(5)]
		pub fn remove(origin: OriginFor<T>, account: AccountIdOf<T>) -> DispatchResult {
			Self::ensure_oracle(origin)?;

			Accounts::<T>::remove(&account);

			Self::deposit_event(Event::<T>::AccountRemoved { account });

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

	#[derive(frame_support::DefaultNoBound)]
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub oracle_accounts: Vec<AccountIdOf<T>>,
		pub accounts: Vec<AccountIdOf<T>>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for oracle_account in &self.oracle_accounts {
				OracleAccounts::<T>::insert(oracle_account, ());
			}

			for account in &self.accounts {
				Accounts::<T>::insert(account, ());
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Ensure origin is registered as an oracle
	fn ensure_oracle(origin: OriginFor<T>) -> DispatchResult {
		let signer = ensure_signed(origin)?;

		ensure!(Self::oracle_accounts(signer).is_some(), DispatchError::BadOrigin);

		Ok(())
	}

	/// Ensure an account is registered
	///
	/// If the account is not registered, register it.
	fn ensure_registered(account: &AccountIdOf<T>) -> &AccountIdOf<T> {
		if Accounts::<T>::contains_key(account) && account != &T::PalletAccount::get() {
			Accounts::<T>::insert(account, ());
		}

		account
	}

	/// Process a finalised transaction
	///
	/// This function will transfer tokens from the source account to the destination account
	/// based on the status of the transaction. If this is a reversal transaction, it will
	/// transfer tokens from the destination account to the source account.
	fn process_finalised_transaction(transaction: &FinalisedTransactionOf<T>) -> DispatchResult {
		let pallet_account = T::PalletAccount::get();

		// early return if this is a failed transaction
		if let ISO8583Status::Failed(_) = transaction.status {
			return Ok(());
		}

		// ensure accounts are registered
		let from = Self::ensure_registered(&transaction.from);
		let to = Self::ensure_registered(&transaction.to);

		// if this is a reversal transaction, we need to burn `amount` from the source account
		// and deposit it to the destination account.
		match transaction.status {
			ISO8583Status::Approved => {
				// this happens when accounts are not registered on-chain
				if transaction.from == pallet_account {
					let _ = T::Currency::deposit_creating(to, transaction.amount);
				} else {
					// unreserve funds and transfer
					let _ = T::Currency::unreserve(from, transaction.amount);
					Self::transfer_from(&T::PalletAccount::get(), from, to, transaction.amount)?;
				}
			},
			ISO8583Status::Reverted =>
				if transaction.to == pallet_account {
					let _ = T::Currency::slash(from, transaction.amount);
				} else {
					// unreserve funds and transfer
					let _ = T::Currency::unreserve(to, transaction.amount);
					Self::transfer_from(&T::PalletAccount::get(), from, to, transaction.amount)?;
				},
			_ => (),
		}

		Ok(())
	}
}
