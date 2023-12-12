//! Tests for the ISO-8583 pallet.

use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;

use crate::{mock::*, Error};

#[test]
fn test_register() {
	ExtBuilder::default().with_oracle_accounts(vec![1]).build().execute_with(|| {
		// only oracle can register
		assert_noop!(
			ISO8583::register(RuntimeOrigin::signed(1234), 1, 100),
			DispatchError::BadOrigin
		);

		// register oracle
		assert_ok!(ISO8583::register(RuntimeOrigin::signed(1), 1, 100));
	});
}

#[test]
fn test_initiate_transfer() {
	ExtBuilder::default()
		.with_oracle_accounts(vec![1, 2])
		.with_accounts(vec![3, 4, 5])
		.build()
		.execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// only registered users can initiate transfer
			assert_noop!(
				ISO8583::initiate_transfer(RuntimeOrigin::signed(1234), 1234, 112, 100),
				Error::<Test>::SourceNotRegistered,
			);

			// transfer is not allowed if user does not have enough balance
			assert_noop!(
				ISO8583::initiate_transfer(RuntimeOrigin::signed(4), 4, 12, INITIAL_BALANCE + 1),
				Error::<Test>::InsufficientAllowance,
			);

			// initiate transfer
			assert_ok!(ISO8583::initiate_transfer(RuntimeOrigin::signed(3), 3, 10, 100));

			// amount is reserved
			assert_eq!(Balances::reserved_balance(3), 100);

			// event is emitted
			System::assert_last_event(RuntimeEvent::ISO8583(
				crate::Event::<Test>::InitiateTransfer { from: 3, to: 10, amount: 100 },
			));
		});
}

#[test]
fn test_approve_transfer() {
	ExtBuilder::default()
		.with_oracle_accounts(vec![1, 2])
		.with_accounts(vec![3, 4, 5])
		.build()
		.execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// initiate transfer
			assert_ok!(ISO8583::initiate_transfer(RuntimeOrigin::signed(3), 3, 4, 20));

			// give allowance from 3 to 10
			assert_ok!(ISO8583::approve(RuntimeOrigin::signed(3), 10, 50));

			// event is emitted
			System::assert_last_event(RuntimeEvent::ISO8583(crate::Event::<Test>::Allowance {
				from: 3,
				to: 10,
				amount: 50,
			}));

			// 10 can now spend 25 from 3
			assert_ok!(ISO8583::initiate_transfer(RuntimeOrigin::signed(10), 3, 6, 25));

			// 10 can not transfer more than allowed
			assert_noop!(
				ISO8583::initiate_transfer(RuntimeOrigin::signed(10), 3, 10, 56),
				Error::<Test>::InsufficientAllowance,
			);

			assert_ok!(ISO8583::initiate_transfer(RuntimeOrigin::signed(4), 4, 5, 10));
		});
}
