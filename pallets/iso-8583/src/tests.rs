//! Tests for the ISO-8583 pallet.

use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;
use sp_runtime::DispatchError;

use crate::{mock::*, types::FinalisedTransaction, Error};

mod extrinsics {
	use super::*;

	#[test]
	fn test_register() {
		ExtBuilder::default().with_oracle_accounts(vec![1]).build().execute_with(|| {
			// only oracle can register
			assert_noop!(
				ISO8583::register(RuntimeOrigin::signed(account(255)), account(1), 100),
				DispatchError::BadOrigin
			);

			// register oracle
			assert_ok!(ISO8583::register(RuntimeOrigin::signed(account(1)), account(1), 100));
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
					ISO8583::initiate_transfer(
						RuntimeOrigin::signed(account(255)),
						account(255),
						account(112),
						100
					),
					Error::<Test>::SourceNotRegistered,
				);

				// transfer is not allowed if user does not have enough balance
				assert_noop!(
					ISO8583::initiate_transfer(
						RuntimeOrigin::signed(account(4)),
						account(4),
						account(12),
						INITIAL_BALANCE + 1
					),
					Error::<Test>::InsufficientAllowance,
				);

				// initiate transfer
				assert_ok!(ISO8583::initiate_transfer(
					RuntimeOrigin::signed(account(3)),
					account(3),
					account(10),
					100
				));

				// amount is reserved
				assert_eq!(Balances::reserved_balance(account(3)), 100);

				// event is emitted
				System::assert_has_event(RuntimeEvent::ISO8583(
					crate::Event::<Test>::InitiateTransfer {
						from: account(3),
						to: account(10),
						amount: 100,
					},
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
				assert_ok!(ISO8583::initiate_transfer(
					RuntimeOrigin::signed(account(3)),
					account(3),
					account(4),
					20
				));

				// give allowance from 3 to 10
				assert_ok!(ISO8583::approve(RuntimeOrigin::signed(account(3)), account(10), 50));

				// event is emitted
				System::assert_has_event(RuntimeEvent::ISO8583(crate::Event::<Test>::Allowance {
					from: account(3),
					to: account(10),
					amount: 50,
				}));

				// 10 can now spend 25 from 3
				assert_ok!(ISO8583::initiate_transfer(
					RuntimeOrigin::signed(account(10)),
					account(3),
					account(6),
					25
				));

				// 10 can not transfer more than allowed
				assert_noop!(
					ISO8583::initiate_transfer(
						RuntimeOrigin::signed(account(10)),
						account(3),
						account(10),
						56
					),
					Error::<Test>::InsufficientAllowance,
				);

				assert_ok!(ISO8583::initiate_transfer(
					RuntimeOrigin::signed(account(4)),
					account(4),
					account(5),
					10
				));
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
				assert_ok!(ISO8583::initiate_transfer(
					RuntimeOrigin::signed(account(3)),
					account(3),
					account(4),
					20
				));

				let dummy_hash = H256::from([0; 32]);

				// initiate reversal
				assert_ok!(ISO8583::initiate_revert(RuntimeOrigin::signed(account(1)), dummy_hash));

				// event is emitted
				System::assert_has_event(RuntimeEvent::ISO8583(
					crate::Event::<Test>::InitiateRevert { who: account(1), hash: dummy_hash },
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

				// non-oracle cannot submit finalities
				assert_noop!(
					ISO8583::submit_finality(
						RuntimeOrigin::signed(account(255)),
						FinalisedTransaction {
							from: account(3),
							to: account(4),
							amount: 20,
							hash: H256::from([0; 32]),
							event_id: (1_u32, 0_u32).encode().try_into().unwrap(),
							status: crate::types::ISO8583Status::Approved,
						}
					),
					DispatchError::BadOrigin,
				);

				// finalised transaction that comes from an account that is not registered
				let finalised_transaction_mint = FinalisedTransaction {
					from: <Test as crate::Config>::PalletAccount::get(),
					to: account(4),
					amount: 20,
					hash: H256::from([0; 32]),
					event_id: (1_u32, 0_u32).encode().try_into().unwrap(),
					status: crate::types::ISO8583Status::Approved,
				};

				// to has initial balance
				assert_eq!(Balances::free_balance(account(4)), INITIAL_BALANCE);

				// submit finalities
				assert_ok!(ISO8583::submit_finality(
					RuntimeOrigin::signed(account(1)),
					finalised_transaction_mint.clone()
				));

				// event is emitted
				System::assert_has_event(RuntimeEvent::ISO8583(
					crate::Event::<Test>::ProcessedTransaction {
						event_id: finalised_transaction_mint.event_id,
						status: finalised_transaction_mint.status,
					},
				));

				// to has +20 balance
				assert_eq!(Balances::free_balance(account(4)), INITIAL_BALANCE + 20);

				// mint event is emitted
				System::assert_has_event(RuntimeEvent::Balances(
					pallet_balances::Event::<Test>::Deposit { who: account(4), amount: 20 },
				));

				// Advance one block
				System::set_block_number(2);

				// finalised transaction that comes from an account that is registered
				let finalised_transaction_transfer = FinalisedTransaction {
					from: account(3),
					to: account(5),
					amount: 23,
					hash: H256::from([0; 32]),
					event_id: (2_u32, 0_u32).encode().try_into().unwrap(),
					status: crate::types::ISO8583Status::Approved,
				};

				// to has 0 balance
				assert_eq!(Balances::free_balance(account(5)), INITIAL_BALANCE);

				// submit finalities
				assert_ok!(ISO8583::submit_finality(
					RuntimeOrigin::signed(account(1)),
					finalised_transaction_transfer.clone()
				));

				// event is emitted
				System::assert_has_event(RuntimeEvent::ISO8583(
					crate::Event::<Test>::ProcessedTransaction {
						event_id: finalised_transaction_transfer.event_id.clone(),
						status: finalised_transaction_transfer.status.clone(),
					},
				));

				// to has 123 balance
				assert_eq!(Balances::free_balance(account(5)), INITIAL_BALANCE + 23);

				// transfer event is emitted
				System::assert_has_event(RuntimeEvent::Balances(
					pallet_balances::Event::<Test>::Transfer {
						from: account(3),
						to: account(5),
						amount: 23,
					},
				));
			});
	}

	#[test]
	fn test_remove_works() {
		ExtBuilder::default().with_oracle_accounts(vec![1]).build().execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// only oracle can remove
			assert_noop!(
				ISO8583::remove(RuntimeOrigin::signed(account(255)), account(1)),
				DispatchError::BadOrigin
			);

			// remove oracle
			assert_ok!(ISO8583::remove(RuntimeOrigin::signed(account(1)), account(1)));
		});
	}
}

mod trait_tests {
	use sp_runtime::TokenError;

	use crate::traits::ERC20R;

	use super::*;

	#[test]
	fn test_transfer_works() {
		ExtBuilder::default().with_accounts(vec![3, 4]).build().execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// not enough balance
			assert_noop!(
				ISO8583::transfer(&account(3), &account(4), INITIAL_BALANCE + 1),
				TokenError::FundsUnavailable,
			);

			assert_ok!(ISO8583::transfer(&account(3), &account(4), 20));

			// event is emitted
			System::assert_has_event(RuntimeEvent::Balances(
				pallet_balances::Event::<Test>::Transfer {
					from: account(3),
					to: account(4),
					amount: 20,
				},
			));
		});
	}

	#[test]
	fn test_approve_works() {
		ExtBuilder::default().with_accounts(vec![3, 4]).build().execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// give allowance from 3 to 4
			assert_ok!(ISO8583::approve(RuntimeOrigin::signed(account(3)), account(4), 50));

			// event is emitted
			System::assert_has_event(RuntimeEvent::ISO8583(crate::Event::<Test>::Allowance {
				from: account(3),
				to: account(4),
				amount: 50,
			}));

			// 4 can now spend 25 from 3
			assert_ok!(ISO8583::transfer_from(&account(4), &account(3), &account(10), 25));

			// try sending without allowance
			assert_noop!(
				ISO8583::transfer_from(&account(3), &account(4), &account(10), 26),
				Error::<Test>::InsufficientAllowance,
			);
		});
	}

	#[test]
	fn test_transfer_from_works() {
		ExtBuilder::default().with_accounts(vec![3, 4, 5]).build().execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// not enough balance
			assert_noop!(
				ISO8583::transfer_from(&account(3), &account(4), &account(5), INITIAL_BALANCE + 1),
				Error::<Test>::InsufficientAllowance,
			);

			// not enough allowance
			assert_noop!(
				ISO8583::transfer_from(&account(3), &account(4), &account(5), 20),
				Error::<Test>::InsufficientAllowance,
			);

			// give allowance from 4 to 3
			assert_ok!(ISO8583::approve(RuntimeOrigin::signed(account(4)), account(3), 50));

			// 3 can now spend 25 from 4
			assert_ok!(ISO8583::transfer_from(&account(3), &account(4), &account(10), 25));

			// 3 can not transfer more than allowed
			assert_noop!(
				ISO8583::transfer_from(&account(3), &account(4), &account(10), 56),
				Error::<Test>::InsufficientAllowance,
			);

			// event is emitted
			System::assert_has_event(RuntimeEvent::Balances(
				pallet_balances::Event::<Test>::Transfer {
					from: account(4),
					to: account(10),
					amount: 25,
				},
			));
		});
	}
}
