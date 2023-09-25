//! Types used in this pallet.

use frame_support::{sp_runtime::BoundedVec, traits::Currency};

use crate::Config;

/// Hash used for transaction ID.
pub type Hash = sp_core::H256;

/// Opaque transaction type
pub type TransactionOf<T> = BoundedVec<u8, <T as Config>::MaxTransactionSize>;

/// Explicit `AccountId`
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// Explicit `Currency` impl of
pub type CurrencyOf<T> = <T as Config>::Currency;

/// Explicit `Balance`
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
