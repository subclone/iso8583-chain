//! Implementations for the pallet.
use super::*;
use crate::traits::ERC20R;
use frame_support::{
	ensure,
	pallet_prelude::DispatchResult,
	traits::tokens::{currency::Currency, ExistenceRequirement},
};

impl<T: Config> ERC20R<AccountIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn transfer(
		&self,
		from: &AccountIdOf<T>,
		to: &AccountIdOf<T>,
		value: BalanceOf<T>,
	) -> DispatchResult {
		<CurrencyOf<T>>::transfer(from, to, value, ExistenceRequirement::KeepAlive)
	}

	fn transfer_from(
		&self,
		spender: &AccountIdOf<T>,
		from: &AccountIdOf<T>,
		to: &AccountIdOf<T>,
		value: BalanceOf<T>,
	) -> DispatchResult {
		Allowances::<T>::try_mutate_exists(from, spender, |maybe_allowance| -> DispatchResult {
			let allowance = maybe_allowance.take().ok_or(Error::<T>::InsufficientAllowance)?;
			ensure!(allowance >= value, Error::<T>::InsufficientAllowance);

			// Transfer tokens
			<CurrencyOf<T>>::transfer(from, to, value, ExistenceRequirement::KeepAlive)?;

			// Update allowances
			let updated_allowance = allowance - value;
			*maybe_allowance = Some(updated_allowance);

			Ok(())
		})?;

		Ok(())
	}

	fn approve(
		&self,
		owner: &AccountIdOf<T>,
		spender: &AccountIdOf<T>,
		value: BalanceOf<T>,
	) -> DispatchResult {
		Allowances::<T>::insert(owner, spender, value);
		Ok(())
	}
}

impl<T: Config> traits::ISO8583<AccountIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn apply(&self) -> DispatchResult {}
}
