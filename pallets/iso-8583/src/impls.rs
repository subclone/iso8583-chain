//! Implementations for the pallet.
use super::*;
use crate::traits::{ERC20R, ISO8583};
use frame_support::{
	ensure,
	pallet_prelude::DispatchResult,
	sp_runtime::traits::AccountIdConversion,
	traits::tokens::{currency::Currency, ExistenceRequirement},
	PalletId,
};

impl<T: Config> ERC20R<AccountIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn transfer(from: &AccountIdOf<T>, to: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
		<CurrencyOf<T>>::transfer(from, to, value, ExistenceRequirement::KeepAlive)
	}

	fn transfer_from(
		spender: &AccountIdOf<T>,
		from: &AccountIdOf<T>,
		to: &AccountIdOf<T>,
		value: BalanceOf<T>,
	) -> DispatchResult {
		if PalletId(*b"oracleee").into_account_truncating() == spender {
			// Transfer tokens
			<CurrencyOf<T>>::transfer(&from, &to, value, ExistenceRequirement::KeepAlive)?;

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
					<CurrencyOf<T>>::transfer(from, to, value, ExistenceRequirement::KeepAlive)?;

					// Update allowances
					let updated_allowance = allowance - value;
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

impl<T: Config> ISO8583<AccountIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn apply() -> DispatchResult {
		Ok(())
	}
}
