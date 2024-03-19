//! Types used in this pallet.

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::Currency;
use frame_system::offchain::{SignedPayload, SigningTypes};
use scale_info::TypeInfo;
use sp_core::{ConstU32, RuntimeDebug};
use sp_runtime::BoundedVec;

use crate::{AccountsOf, Config, StorageKey};

/// Hash used for transaction ID.
pub type Hash = sp_core::H256;

/// Explicit `AccountId`
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// Explicit `Currency` impl of
pub type CurrencyOf<T> = <T as Config>::Currency;

/// Explicit `Balance`
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

/// Explicit `Transaction`
pub type FinalisedTransactionOf<T> = FinalisedTransaction<AccountIdOf<T>, BalanceOf<T>>;

/// Event ID: `block_number` - `event_index`
pub type EventId = BoundedVec<u8, ConstU32<16>>;

/// Basic transaction type
///
/// Block number and event index serve as a unique identifier of a transaction. They highlight
/// the block and event index where this transaction was triggered by the oracle gateway.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FinalisedTransaction<AccountId, Balance> {
	/// Transaction ID
	pub hash: Hash,
	/// Sender
	pub from: AccountId,
	/// Receiver
	pub to: AccountId,
	/// Amount
	pub amount: Balance,
	/// Event ID
	pub event_id: EventId,
	/// Status of the transaction
	pub status: ISO8583Status,
}

/// ISO-8583 transaction status
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum ISO8583Status {
	/// Transaction is finalised
	Approved,
	/// Failed
	Failed(ISO8583FailureReason),
}

/// Reason for failure of ISO-8583 transaction
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum ISO8583FailureReason {
	/// Insufficient funds
	InsufficientFunds,
	/// Invalid transaction
	InvalidTransaction,
	/// Invalid PAN
	InvalidCardNumber,
	/// Expired card
	ExpiredCard,
	/// Do not honor
	DoNotHonor,
	/// Other
	Other,
}

/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
pub struct UpdateAccountsPayload<Public, Accounts, StorageKey> {
	/// Public key of the off-chain worker
	pub public: Public,
	/// Updated accounts
	pub accounts: Accounts,
	/// Last iterated storage key
	pub last_key: StorageKey,
}

impl<T: SigningTypes + crate::Config> SignedPayload<T>
	for UpdateAccountsPayload<T::Public, AccountsOf<T>, StorageKey>
{
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}
