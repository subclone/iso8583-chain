//! Implementations for the pallet.
use super::*;
use crate::traits::ERC20R;
use frame_support::{
	ensure,
	pallet_prelude::DispatchResult,
	traits::tokens::{currency::Currency, ExistenceRequirement},
};

impl<T: Config> ERC20R<AccountIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn transfer(from: &AccountIdOf<T>, to: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
		CurrencyOf::<T>::transfer(from, to, value, ExistenceRequirement::KeepAlive)
	}

	fn transfer_from(
		spender: &AccountIdOf<T>,
		from: &AccountIdOf<T>,
		to: &AccountIdOf<T>,
		value: BalanceOf<T>,
	) -> DispatchResult {
		// Pallet account has unlimited allowance for all accounts and transfering from self is
		// allowed
		if &T::PalletAccount::get() == spender || from == spender {
			CurrencyOf::<T>::transfer(&from, &to, value, ExistenceRequirement::KeepAlive)?;
			Ok(())
		} else {
			Allowances::<T>::try_mutate_exists(
				from,
				spender,
				|maybe_allowance| -> DispatchResult {
					let allowance =
						maybe_allowance.take().ok_or(Error::<T>::InsufficientAllowance)?;
					ensure!(allowance >= value, Error::<T>::InsufficientAllowance);

					// Transfer tokens
					CurrencyOf::<T>::transfer(from, to, value, ExistenceRequirement::KeepAlive)?;

					// Update allowances
					let updated_allowance = allowance.saturating_sub(value);
					*maybe_allowance = Some(updated_allowance);

					Ok(())
				},
			)?;

			Ok(())
		}
	}

	fn approve(
		owner: &AccountIdOf<T>,
		spender: &AccountIdOf<T>,
		value: BalanceOf<T>,
	) -> DispatchResult {
		Allowances::<T>::insert(owner, spender, value);
		Ok(())
	}
}
