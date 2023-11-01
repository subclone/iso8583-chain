//! Traits
use frame_support::pallet_prelude::DispatchResult;

/// ERC20-R Reversible ERC20 interface
pub trait ERC20R<AccountId, Balance> {
	/// Transfer `value` tokens from `from` to `to`
	fn transfer(&self, from: &AccountId, to: &AccountId, value: Balance) -> DispatchResult;

	/// Transfer `value` tokens from `from` to `to` on behalf of `spender`
	fn transfer_from(
		&self,
		spender: &AccountId,
		from: &AccountId,
		to: &AccountId,
		value: Balance,
	) -> DispatchResult;

	/// Approve `spender` to transfer `value` tokens from `owner`
	fn approve(&self, owner: &AccountId, spender: &AccountId, value: Balance) -> DispatchResult;
}

/// ISO-8583 interface
pub trait ISO8583<AccountId, Balance>: ERC20R<AccountId, Balance> {
	/// Apply a transaction
	fn apply(&self) -> DispatchResult;
}
