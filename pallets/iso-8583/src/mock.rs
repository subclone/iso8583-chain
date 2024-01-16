//! Mock runtime for tests

use frame_support::{parameter_types, traits::Everything, weights::IdentityFee, PalletId};
use pallet_balances::AccountData;
use sp_core::{ConstU128, ConstU32, ConstU64, H256};
use sp_runtime::{
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;
type Balance = u128;

/// Initial balance of an account.
pub(crate) const INITIAL_BALANCE: Balance = 100;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances::{Pallet, Storage, Event<T>, Config<T>},
		Timestamp: pallet_timestamp::{Pallet, Storage},
		ISO8583: crate::{Pallet, Storage, Event<T>, Call, Config<T>},
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<0>;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type MaxHolds = ();
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<3>;
	type WeightInfo = ();
}

parameter_types! {
	pub PalletAccount: AccountId = PalletId(*b"py/iso85").into_account_truncating();
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type PalletAccount = PalletAccount;
	type MaxStringSize = ConstU32<1024>;
	type WeightToFee = IdentityFee<Balance>;
}

/// Helper struct to create new test externalities
#[derive(Default)]
pub(crate) struct ExtBuilder {
	oracle_accounts: Vec<AccountId>,
	accounts: Vec<AccountId>,
}

impl ExtBuilder {
	pub(crate) fn with_oracle_accounts(mut self, oracle_accounts: Vec<AccountId>) -> Self {
		self.oracle_accounts = oracle_accounts;
		self
	}

	pub(crate) fn with_accounts(mut self, accounts: Vec<AccountId>) -> Self {
		self.accounts = accounts;
		self
	}

	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		crate::GenesisConfig::<Test> {
			oracle_accounts: self.oracle_accounts.clone(),
			accounts: self.accounts.clone(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut endowed_accounts = self.accounts.clone();

		endowed_accounts.append(&mut self.oracle_accounts.clone());

		pallet_balances::GenesisConfig::<Test> {
			balances: endowed_accounts.iter().map(|x| (*x, INITIAL_BALANCE)).collect(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
