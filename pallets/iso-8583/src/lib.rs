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
	dispatch::Vec,
	pallet_prelude::{ValueQuery, *},
	traits::{BuildGenesisConfig, Currency, PalletInfoAccess, ReservableCurrency},
};
use frame_system::{
	ensure_signed,
	offchain::{ForAll, SendUnsignedTransaction, SignMessage, SignedPayload, Signer},
	pallet_prelude::OriginFor,
};
use sp_runtime::{
	offchain::http,
	traits::{TryConvert, Zero},
	KeyTypeId, Saturating,
};

use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};
use lite_json::{parse_json, JsonValue, Serialize};
use sp_std::vec;

pub use pallet::*;
use traits::*;
use types::*;

#[cfg(test)]
use crate::tests::MOCKED_SIGNATURE;

use crate::impls::{AccountIdDecoder, BalanceDecoder};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"iso8");

/// Max number of accounts to query in offchain worker
pub const MAX_ACCOUNTS: u32 = 20;

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct Iso8583AuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for Iso8583AuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for Iso8583AuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

/// List of accounts, bound is arbitrary, enough for our use case.
type AccountsOf<T> = BoundedVec<(AccountIdOf<T>, BalanceOf<T>), ConstU32<30>>;

/// Storage key as a bounded vector. The bound is arbitrary, enough for our use case.
type StorageKey = BoundedVec<u8, ConstU32<128>>;

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Pallet configuration
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The identifier type for an offchain worker.
		type AuthorityId: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>;
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
		/// Interval between offchain worker runs
		#[pallet::constant]
		type OffchainWorkerInterval: Get<BlockNumberFor<Self>>;
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

	/// Last queried storage key for offchain worker
	/// Offchain worker iterates through all the registered accounts, queries their balances
	/// and updates updates the on-chain balances if they are out of sync.
	///
	/// Since we can not do unlimited iterations in offchain worker, we need to keep track of
	/// the last iterated storage key.
	#[pallet::storage]
	#[pallet::getter(fn last_storage_key)]
	pub type LastIteratedStorageKey<T: Config> = StorageValue<_, StorageKey, OptionQuery>;

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

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call
		///
		/// This function is used to validate unsigned calls.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// check if the call is `update_accounts_unsigned`
			if let Call::update_accounts_unsigned { payload, signature } = call {
				// valid signature
				let valid_signature =
					SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());

				if !valid_signature {
					return InvalidTransaction::BadProof.into();
				}

				let UpdateAccountsPayload { public: _public, accounts, last_key: _ } = payload;

				if accounts.is_empty() {
					return InvalidTransaction::Call.into();
				} else if accounts.len() > MAX_ACCOUNTS as usize {
					return InvalidTransaction::ExhaustsResources.into();
				}

				ValidTransaction::with_tag_prefix("ISO8583")
					.priority(TransactionPriority::max_value())
					.and_provides(
						frame_system::Pallet::<T>::block_number() +
							T::OffchainWorkerInterval::get(),
					)
					.longevity(5)
					.propagate(false)
					.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
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

		/// Submit updated balances
		///
		/// This function is used by the offchain worker to submit updated balances to the chain.
		#[pallet::weight(T::DbWeight::get().writes(payload.accounts.len() as u64 + 1))]
		#[pallet::call_index(6)]
		pub fn update_accounts_unsigned(
			origin: OriginFor<T>,
			payload: UpdateAccountsPayload<T::Public, AccountsOf<T>, StorageKey>,
			_signature: T::Signature,
		) -> DispatchResult {
			// it is an unsigned transaction
			ensure_none(origin)?;

			let UpdateAccountsPayload { public: _public, accounts, last_key } = payload;

			for (account, balance) in accounts {
				// do basic check if account is registered
				if Accounts::<T>::contains_key(&account) {
					T::Currency::make_free_balance_be(&account, balance);
				}
			}

			LastIteratedStorageKey::<T>::put(last_key);

			Ok(())
		}

		/// Register an oracle account
		///
		/// This function is used to register an oracle account.
		///
		/// # Errors
		///
		/// Origin must be signed by the root account.
		///
		/// # Weight
		///
		/// - `O(1)`
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(7)]
		pub fn register_oracle(origin: OriginFor<T>, account: AccountIdOf<T>) -> DispatchResult {
			ensure_root(origin)?;

			OracleAccounts::<T>::insert(&account, ());

			Ok(())
		}

		/// Remove an oracle account
		///
		/// This function is used to remove an oracle account.
		///
		/// # Errors
		///
		/// Origin must be signed by the root account.
		///
		/// # Weight
		///
		/// - `O(1)`
		#[pallet::weight(T::DbWeight::get().writes(1))]
		#[pallet::call_index(8)]
		pub fn remove_oracle(origin: OriginFor<T>, account: AccountIdOf<T>) -> DispatchResult {
			ensure_root(origin)?;

			OracleAccounts::<T>::remove(&account);

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain worker
		///
		/// Queries balances of all registered accounts and makes sure they are in sync with the
		/// offchain ledger.
		fn offchain_worker(now: BlockNumberFor<T>) {
			// respect interval between offchain worker runs
			if now % T::OffchainWorkerInterval::get() != Zero::zero() {
				return;
			}

			// get last iterated storage key
			let prefix = storage::storage_prefix(
				<Pallet<T> as PalletInfoAccess>::name().as_bytes(),
				b"Accounts",
			);

			let mut previous_key = if let Some(key) = LastIteratedStorageKey::<T>::get() {
				key.into_inner()
			} else {
				prefix.to_vec()
			};

			let mut count = 0;

			let mut accounts = Vec::new();
			while let Some(next) = sp_io::storage::next_key(&previous_key) {
				// Ensure we are iterating through the correct storage prefix
				if !next.starts_with(&prefix) {
					previous_key = prefix.to_vec();
					break;
				}

				previous_key = next;
				count += 1;

				// decode from last 32 bytes of the key
				if let Ok(account) =
					AccountIdOf::<T>::decode(&mut &previous_key[previous_key.len() - 32..])
				{
					accounts.push(account);
				}

				if count >= MAX_ACCOUNTS {
					break;
				}
			}

			// if there are no accounts, early return
			if accounts.is_empty() {
				return;
			}

			// fetch and submit updated balances
			match Self::fetch_and_submit_updated_balances(accounts.clone(), previous_key) {
				Ok(_) => log::info!(
					target: "offchain-worker",
					"Submitted updated balances for {} accounts",
					accounts.len(),
				),
				Err(e) => log::error!(target: "offchain-worker", "Failed: {:?}", e),
			}
		}
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
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

		// we don't distinguish between transfer and reverse transactions
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
			_ => (),
		}

		Ok(())
	}
}

/// Functions used by offchain worker
impl<T: Config> Pallet<T> {
	/// Submit updated balances
	fn fetch_and_submit_updated_balances(
		accounts: Vec<AccountIdOf<T>>,
		last_iterated_storage_key: Vec<u8>,
	) -> Result<(), &'static str> {
		// submit updated balances
		let signer = Signer::<T, T::AuthorityId>::all_accounts();

		if !signer.can_sign() {
			return Err("No local accounts available");
		}

		let mut updated_accounts =
			Self::fetch_balances(&signer, accounts).map_err(|_| "Failed to fetch balances")?;

		let last_iterated_storage_key: StorageKey =
			last_iterated_storage_key.try_into().map_err(|_| "Invalid key")?;

		// check for each balance if it is updated
		updated_accounts.retain(|(account, balance)| {
			let current_balance = T::Currency::free_balance(account);
			*balance != current_balance
		});

		// only submit if there are updated balances
		// we trust that API will only return updated balances
		if updated_accounts.len().is_zero() {
			return Ok(());
		}

		// Actually send the extrinsic to the chain
		let result = signer.send_unsigned_transaction(
			|account| UpdateAccountsPayload {
				public: account.public.clone(),
				accounts: updated_accounts.clone(),
				last_key: last_iterated_storage_key.clone(),
			},
			|payload, signature| Call::update_accounts_unsigned { payload, signature },
		);

		for (acc, res) in &result {
			match res {
				Ok(()) =>
					log::info!(target: "offchain-worker", "Submitted updated balances by: {:?}", acc.id),
				Err(e) =>
					log::error!(target: "offchain-worker", "Account: {:?} failed: {:?}", acc.id, e),
			}
		}

		Ok(())
	}

	/// Fetch balances of batch accounts
	fn fetch_balances(
		signer: &Signer<T, T::AuthorityId, ForAll>,
		accounts: Vec<AccountIdOf<T>>,
	) -> Result<AccountsOf<T>, http::Error> {
		let deadline =
			sp_io::offchain::timestamp().add(sp_core::offchain::Duration::from_millis(2_000));

		// Body of the POST request, list of accounts
		let body = JsonValue::Array(
			accounts
				.iter()
				.map(|account| {
					JsonValue::String(hex::encode(account.encode()).chars().map(|x| x).collect())
				})
				.collect::<Vec<_>>()
				.into(),
		);

		#[cfg(not(test))]
		// sign the body of the request
		let results = signer.sign_message(&body.serialize());
		#[cfg(not(test))]
		let signature = results[0].1.encode();

		// sr25519 signatures are non-deterministic
		#[cfg(test)]
		let signature = MOCKED_SIGNATURE.to_vec();

		let body = JsonValue::Object(vec![
			("accounts".chars().into_iter().collect(), body),
			(
				"signature".chars().into_iter().collect(),
				JsonValue::String(hex::encode(&signature[..]).chars().map(|x| x).collect()),
			),
		])
		.serialize();

		// Form the request
		let request = http::Request::new("http://localhost:3001/balances")
			.method(http::Method::Post)
			.deadline(deadline)
			.body(vec![body])
			.add_header("Content-Type", "application/json")
			.add_header("accept", "*/*")
			.send()
			.map_err(|_| http::Error::IoError)?;

		// Wait until the request is done, or until the deadline is reached
		let response = request.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

		let binding = response.body().collect::<Vec<u8>>();

		let json_str: &str = match core::str::from_utf8(&binding) {
			Ok(v) => v,
			Err(_e) => "Error parsing json",
		};

		let raw_accounts = parse_json(json_str).map_err(|_| http::Error::IoError)?;

		let mut parsed_accounts = Vec::new();

		// Parse the response. Expects a list of accounts and their balances
		// Example response:
		// ```json
		// [
		//   {"account_id": "5GQ...","balance": "100.11"},
		//   {"account_id": "5FQ...","balance": "200.22"},
		//   ..
		// ]
		// ```
		match raw_accounts {
			JsonValue::Array(inner_accounts) =>
				for inner_account in inner_accounts {
					match inner_account {
						JsonValue::Object(entries) => {
							debug_assert!(
								entries.len() == 2,
								"Invalid response, expected 2 fields"
							);

							let account_id = entries[0].clone();
							let balance = entries[1].clone();

							let account_id = AccountIdDecoder::<T>::try_convert(&account_id.1)
								.map_err(|_| http::Error::IoError)?;

							let balance = BalanceDecoder::<T>::try_convert(&balance.1)
								.map_err(|_| http::Error::IoError)?;

							parsed_accounts.push((account_id, balance));
						},
						_ => return Err(http::Error::IoError),
					}
				},
			_ => return Err(http::Error::IoError),
		};

		Ok(parsed_accounts.try_into().map_err(|_| http::Error::IoError)?)
	}
}
