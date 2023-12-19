//! Tests for the ISO-8583 pallet.

use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;
use sp_runtime::DispatchError;

use crate::{mock::*, types::FinalisedTransaction, Error};

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

#[test]
fn test_reversal() {
	ExtBuilder::default()
		.with_oracle_accounts(vec![1, 2])
		.with_accounts(vec![3, 4, 5])
		.build()
		.execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// initiate transfer
			assert_ok!(ISO8583::initiate_transfer(RuntimeOrigin::signed(3), 3, 4, 20));

			let dummy_hash = H256::from([0; 32]);

			// initiate reversal
			assert_ok!(ISO8583::initiate_revert(RuntimeOrigin::signed(1), dummy_hash));

			// event is emitted
			System::assert_last_event(RuntimeEvent::ISO8583(
				crate::Event::<Test>::InitiateRevert { who: 1, hash: dummy_hash },
			));
		});
}

#[test]
fn test_submit_finalities() {
	ExtBuilder::default()
		.with_oracle_accounts(vec![1, 2])
		.with_accounts(vec![3, 4, 5])
		.build()
		.execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// to has 0 balance
			assert_eq!(Balances::free_balance(4), INITIAL_BALANCE);

			// finalised transaction that comes from an account that is not registered
			let finalised_transaction_mint = FinalisedTransaction {
				from: <Test as crate::Config>::PalletAccount::get().into(),
				to: 4,
				amount: 20,
				hash: H256::from([0; 32]),
				event_id: (1_u32, 0_u32).encode().try_into().unwrap(),
				status: crate::types::ISO8583Status::Approved,
			};

			// submit finalities
			assert_ok!(ISO8583::submit_finality(
				RuntimeOrigin::signed(1),
				finalised_transaction_mint.clone()
			));

			// event is emitted
			System::assert_last_event(RuntimeEvent::ISO8583(
				crate::Event::<Test>::ProcessedTransaction {
					event_id: finalised_transaction_mint.event_id,
					status: finalised_transaction_mint.status,
				},
			));

			// to has 20 balance
			assert_eq!(Balances::free_balance(4), INITIAL_BALANCE + 20);

			// mint event is emitted
			assert!(System::events().iter().any(|r| {
				r.event ==
					RuntimeEvent::Balances(pallet_balances::Event::<Test>::Deposit {
						who: 4,
						amount: 20,
					})
			}));
		});
}
