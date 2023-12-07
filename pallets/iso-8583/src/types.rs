//! Types used in this pallet.

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::Currency;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;

use crate::Config;

/// Hash used for transaction ID.
pub type Hash = sp_core::H256;

/// Explicit `AccountId`
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// Explicit `Currency` impl of
pub type CurrencyOf<T> = <T as Config>::Currency;

/// Explicit `Balance`
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

/// Explicit `Transaction`
pub type TransactionOf<T> = Transaction<AccountIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>;

/// Basic transaction type
///
/// Block number and event index serve as a unique identifier of a transaction. They highlight
/// the block and event index where this transaction was triggered by the oracle gateway.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Transaction<AccountId, Balance, BlockNumber> {
	/// Transaction ID
	pub id: Hash,
	/// Sender
	pub from: AccountId,
	/// Receiver
	pub to: AccountId,
	/// Amount
	pub amount: Balance,
	/// Block number
	pub block_number: BlockNumber,
	/// Event index
	pub event_index: u32,
	/// Status of the transaction
	pub status: ISO8583Status,
}

/// ISO-8583 transaction status
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum ISO8583Status {
	/// Transaction is finalised
	Approved,
	/// Transaction is reverted
	Reverted,
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
