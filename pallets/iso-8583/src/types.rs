//! Types used in this pallet.

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{sp_runtime::BoundedVec, traits::Currency};
use scale_info::TypeInfo;
use sp_core::{Get, RuntimeDebug};

use crate::Config;

/// Hash used for transaction ID.
pub type Hash = sp_core::H256;

/// Batch of transactions
pub type TransactionBatch<T> = BoundedVec<TransactionOf<T>, <T as Config>::MaxBatchSize>;

/// Explicit `AccountId`
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// Explicit `Currency` impl of
pub type CurrencyOf<T> = <T as Config>::Currency;

/// Explicit `Balance`
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

/// Explicit `Transaction`
pub type TransactionOf<T> = Transaction<AccountIdOf<T>, BalanceOf<T>>;

/// Basic transaction type
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Transaction<AccountId, Balance> {
	/// Transaction ID
	pub id: Hash,
	/// Sender
	pub from: AccountId,
	/// Receiver
	pub to: AccountId,
	/// Amount
	pub amount: Balance,
}

/// Bank account type
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BankAccount<MaxStringSize: Get<u32>> {
	/// Card number
	pub card_number: BoundedVec<u8, MaxStringSize>,
}
