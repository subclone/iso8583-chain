//! Implementations for the pallet.
use super::*;
use crate::traits::ERC20R;
use frame_support::{
	ensure,
	pallet_prelude::DispatchResult,
	traits::tokens::{currency::Currency, ExistenceRequirement},
};
use sp_runtime::{traits::TryConvert, SaturatedConversion};

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

/// Converts `JsonValue` to `BalanceOf<T>`.
pub(crate) struct BalanceDecoder<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> TryConvert<&JsonValue, BalanceOf<T>> for BalanceDecoder<T> {
	fn try_convert(json: &JsonValue) -> Result<BalanceOf<T>, &JsonValue> {
		json.clone()
			.to_number()
			.map(|num| {
				let value_1 = num.integer as u128 * 10_u128.pow(num.exponent as u32 + 10);
				let value_2 = num.fraction as u128 *
					10_u128.pow(num.exponent as u32 + 10 - num.fraction_length);
				(value_1 + value_2).saturated_into()
			})
			.ok_or(json)
	}
}

/// Converts `JsonValue` to `AccountIdOf<T>`.
pub(crate) struct AccountIdDecoder<T: Config>(sp_std::marker::PhantomData<T>);

impl<T: Config> TryConvert<&JsonValue, AccountIdOf<T>> for AccountIdDecoder<T> {
	fn try_convert(json: &JsonValue) -> Result<AccountIdOf<T>, &JsonValue> {
		let raw_bytes = json
			.clone()
			.to_string()
			.map(|v| v.iter().map(|c| *c as u8).collect::<Vec<_>>())
			.ok_or(json)?;

		AccountIdOf::<T>::decode(&mut &raw_bytes[..]).map_err(|_| json)
	}
}
