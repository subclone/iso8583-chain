//! Tests for the ISO-8583 pallet.

use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;
use sp_runtime::DispatchError;

use crate::{mock::*, types::FinalisedTransaction, Error};

const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
/// Mocked signature
pub(crate) const MOCKED_SIGNATURE: [u8; 64] = [
	192, 93, 98, 222, 3, 215, 244, 47, 53, 196, 78, 14, 232, 48, 38, 87, 243, 210, 18, 249, 38,
	135, 182, 239, 29, 12, 204, 246, 126, 242, 148, 113, 155, 92, 146, 117, 165, 156, 244, 91, 46,
	62, 224, 153, 45, 78, 121, 173, 214, 20, 54, 72, 187, 41, 77, 29, 103, 241, 44, 5, 238, 171, 5,
	138,
];

mod extrinsics {
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sp_runtime::RuntimeAppPublic;

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
	#[test]
	fn test_update_accounts() {
		const PHRASE: &str =
			"news slush supreme milk chapter athlete soap sausage put clutch what kitten";

		let keystore = MemoryKeystore::new();
		keystore
			.sr25519_generate_new(crate::crypto::Public::ID, Some(&format!("{}/iso8583", PHRASE)))
			.unwrap();

		let mut t = ExtBuilder::default().with_accounts(vec![123, 125]).build();

		t.register_extension(KeystoreExt::new(keystore));

		t.execute_with(|| {
			// set block to 1, to read events
			System::set_block_number(1);

			// only ocw account can update accounts
			assert_noop!(
				ISO8583::update_accounts_unsigned(
					RuntimeOrigin::signed(account(255)),
					vec![(account(123), 100_110_000), (account(125), 125_250_000)]
						.try_into()
						.unwrap(),
					None,
				),
				DispatchError::BadOrigin
			);

			// update accounts
			assert_ok!(ISO8583::update_accounts_unsigned(
				RuntimeOrigin::none(),
				vec![(account(123), 100_110_000), (account(125), 125_250_000)]
					.try_into()
					.unwrap(),
				None,
			));
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

mod offchain_worker {
	use super::*;
	use crate::{AccountsOf, Config};
	use codec::Decode;
	use frame_support::traits::{Get, OffchainWorker};
	use frame_system::{offchain::Signer, pallet_prelude::BlockNumberFor};
	use lite_json::{JsonValue, NumberValue, Serialize};
	use sp_core::offchain::{testing, OffchainWorkerExt, TransactionPoolExt};
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sp_runtime::RuntimeAppPublic;

	/// Indent the body of the request.
	///
	/// simply adds opening and closing brackets and newlines to the body.
	fn mock_request(accounts: Vec<u8>) -> Vec<u8> {
		let mut full = Vec::new();

		full.push('[' as u8);

		for index in accounts {
			full.push('"' as u8);
			full.extend_from_slice(&account(index).encode()[..]);
			full.push('"' as u8);
			full.push(',' as u8);
		}

		// remove the last comma
		full.pop();
		full.push(']' as u8);

		full
	}

	fn mock_response(accounts: Vec<(u8, f64)>) -> Vec<u8> {
		JsonValue::Array(
			accounts
				.iter()
				.map(|(id, balance)| {
					JsonValue::Object(vec![
						(
							"account_id".to_string().chars().into_iter().collect(),
							JsonValue::String(
								account(*id).encode().into_iter().map(|v| v as char).collect(),
							),
						),
						(
							"balance".to_string().chars().into_iter().collect(),
							JsonValue::Number(NumberValue {
								integer: balance.trunc() as u64,
								fraction: (balance.fract() * 100.0).ceil() as u64,
								fraction_length: 2,
								exponent: 4,
								negative: false,
							}),
						),
					])
				})
				.collect::<Vec<_>>()
				.into(),
		)
		.serialize()
	}

	#[test]
	fn fetch_balances_works() {
		let (offchain, state) = testing::TestOffchainExt::new();
		let mut t = ExtBuilder::default().with_accounts(vec![]).build();
		t.register_extension(OffchainWorkerExt::new(offchain));

		let keystore = MemoryKeystore::new();
		keystore
			.sr25519_generate_new(crate::crypto::Public::ID, Some(&format!("{}/iso8583", PHRASE)))
			.unwrap();
		t.register_extension(KeystoreExt::new(keystore));

		let interval: BlockNumberFor<Test> = <Test as Config>::OffchainWorkerInterval::get();
		let signer = Signer::<Test, <Test as crate::Config>::AuthorityId>::all_accounts();

		// we are not expecting any request
		t.execute_with(|| {
			ISO8583::offchain_worker(interval - 1);
		});

		{
			let mut state = state.write();
			assert_eq!(state.requests.len(), 0);

			let body = mock_request(vec![123]);
			let mut signature = MOCKED_SIGNATURE.to_vec();
			signature.extend(body);

			let response = mock_response(vec![(123, 100.11)]);

			// prepare expectation for the request
			state.expect_request(testing::PendingRequest {
				method: "POST".into(),
				uri: "http://localhost:3001/balances".into(),
				body: signature,
				response: Some(response),
				sent: true,
				headers: vec![
					("Content-Type".to_string(), "application/json".to_string()),
					("accept".to_string(), "*/*".to_string()),
				],
				..Default::default()
			});
		}

		// skip to block `OffchainWorkerInterval`
		t.execute_with(|| {
			let parsed_accounts: AccountsOf<Test> =
				vec![(account(123), 100_110_000)].try_into().unwrap();
			assert_eq!(
				ISO8583::fetch_balances(&signer, vec![account(123)]).unwrap(),
				parsed_accounts
			);
		});
	}

	#[test]
	fn fetch_and_submit_updated_balances_works() {
		let (offchain, state) = testing::TestOffchainExt::new();
		let (pool, pool_state) = testing::TestTransactionPoolExt::new();
		let keystore = MemoryKeystore::new();
		keystore
			.sr25519_generate_new(crate::crypto::Public::ID, Some(&format!("{}/iso8583", PHRASE)))
			.unwrap();

		let mut t = ExtBuilder::default().with_accounts(vec![123, 125]).build();
		t.register_extension(OffchainWorkerExt::new(offchain));
		t.register_extension(TransactionPoolExt::new(pool));
		t.register_extension(KeystoreExt::new(keystore));

		{
			let mut state = state.write();
			assert_eq!(state.requests.len(), 0);

			let body = mock_request(vec![123, 125]);
			let mut payload = MOCKED_SIGNATURE.to_vec();
			payload.extend(body);

			let response = mock_response(vec![(123, 100.11), (125, 125.25)]);

			// prepare expectation for the request
			state.expect_request(testing::PendingRequest {
				method: "POST".into(),
				uri: "http://localhost:3001/balances".into(),
				body: payload,
				response: Some(response),
				sent: true,
				headers: vec![
					("Content-Type".to_string(), "application/json".to_string()),
					("accept".to_string(), "*/*".to_string()),
				],
				..Default::default()
			});
		}

		// we are not expecting any request
		t.execute_with(|| {
			ISO8583::fetch_and_submit_updated_balances(vec![account(123), account(125)], vec![])
				.unwrap();

			let tx = pool_state.write().transactions.pop().unwrap();
			assert!(pool_state.read().transactions.is_empty());
			let tx = crate::mock::Extrinsic::decode(&mut &tx[..]).unwrap();
			// assert_eq!(tx.signature.unwrap().0, account(123));
			assert_eq!(
				tx.call,
				RuntimeCall::ISO8583(crate::Call::update_accounts_unsigned {
					updated_accounts: vec![
						(account(123), 100_110_000),
						(account(125), 125_250_000)
					]
					.try_into()
					.unwrap(),
					last_iterated_storage_key: Some(vec![].try_into().unwrap())
				})
			);
		});
	}

	#[test]
	fn offchain_worker_works() {
		const PHRASE: &str =
			"news slush supreme milk chapter athlete soap sausage put clutch what kitten";

		let (offchain, state) = testing::TestOffchainExt::new();
		let (pool, pool_state) = testing::TestTransactionPoolExt::new();
		let keystore = MemoryKeystore::new();
		keystore
			.sr25519_generate_new(crate::crypto::Public::ID, Some(&format!("{}/iso8583", PHRASE)))
			.unwrap();

		let mut t = ExtBuilder::default().with_accounts(vec![123, 125]).build();

		t.register_extension(OffchainWorkerExt::new(offchain));
		t.register_extension(TransactionPoolExt::new(pool));
		t.register_extension(KeystoreExt::new(keystore));

		let interval: BlockNumberFor<Test> = <Test as Config>::OffchainWorkerInterval::get();

		{
			let mut state = state.write();
			assert_eq!(state.requests.len(), 0);

			let body = mock_request(vec![125, 123]);
			let mut payload = MOCKED_SIGNATURE.to_vec();
			payload.extend(body);

			let response = mock_response(vec![(125, 125.25), (123, 100.11)]);

			// prepare expectation for the request
			state.expect_request(testing::PendingRequest {
				method: "POST".into(),
				uri: "http://localhost:3001/balances".into(),
				body: payload,
				response: Some(response),
				sent: true,
				headers: vec![
					("Content-Type".to_string(), "application/json".to_string()),
					("accept".to_string(), "*/*".to_string()),
				],
				..Default::default()
			});
		}

		t.execute_with(|| {
			ISO8583::offchain_worker(interval);

			let tx = pool_state.write().transactions.pop().unwrap();
			assert!(pool_state.read().transactions.is_empty());
			let tx = crate::mock::Extrinsic::decode(&mut &tx[..]).unwrap();
			assert_eq!(
				tx.call,
				RuntimeCall::ISO8583(crate::Call::update_accounts_unsigned {
					updated_accounts: vec![
						(account(125), 125_250_000),
						(account(123), 100_110_000)
					]
					.try_into()
					.unwrap(),
					last_iterated_storage_key: Some(
						vec![
							154, 237, 128, 107, 236, 245, 7, 140, 126, 24, 139, 0, 248, 2, 43, 16,
							142, 231, 65, 138, 101, 49, 23, 61, 96, 209, 246, 168, 45, 143, 77, 81,
							255, 208, 58, 29, 34, 171, 170, 249, 207, 18, 242, 36, 206, 63, 124,
							149, 123, 123, 123, 123, 123, 123, 123, 123, 123, 123, 123, 123, 123,
							123, 123, 123, 123, 123, 123, 123, 123, 123, 123, 123, 123, 123, 123,
							123, 123, 123, 123, 123
						]
						.try_into()
						.unwrap()
					)
				})
			);
		});

		{
			let mut state = state.write();
			assert_eq!(state.requests.len(), 0);

			let body = mock_request(vec![125, 123]);
			let mut payload = MOCKED_SIGNATURE.to_vec();
			payload.extend(body);

			let response = mock_response(vec![]);

			// prepare expectation for the request
			state.expect_request(testing::PendingRequest {
				method: "POST".into(),
				uri: "http://localhost:3001/balances".into(),
				body: payload,
				response: Some(response),
				sent: true,
				headers: vec![
					("Content-Type".to_string(), "application/json".to_string()),
					("accept".to_string(), "*/*".to_string()),
				],
				..Default::default()
			});
		}

		t.execute_with(|| {
			ISO8583::offchain_worker(interval * 2);
			// no transaction is submitted, since response is empty
			assert!(pool_state.read().transactions.is_empty());
		});
	}
}
