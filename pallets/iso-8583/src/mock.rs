//! Mock runtime for tests

use crate::crypto;
use frame_support::{parameter_types, traits::Everything, weights::IdentityFee, PalletId};
use pallet_balances::AccountData;
use sp_core::{sr25519::Signature, ConstU128, ConstU32, ConstU64, H256};
use sp_runtime::{
	testing::TestXt,
	traits::{
		AccountIdConversion, BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup,
		Verify,
	},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
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

type Extrinsic = TestXt<RuntimeCall, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

impl crate::Config for Test {
	type AuthorityId = crypto::TestAuthId;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type PalletAccount = PalletAccount;
	type MaxStringSize = ConstU32<1024>;
	type WeightToFee = IdentityFee<Balance>;
}

/// Mock account id for testing
pub(crate) fn account(value: u8) -> AccountId {
	sp_core::sr25519::Public::from_raw([value; 32])
}

/// Helper struct to create new test externalities
#[derive(Default)]
pub(crate) struct ExtBuilder {
	oracle_accounts: Vec<AccountId>,
	accounts: Vec<AccountId>,
}

impl ExtBuilder {
	pub(crate) fn with_oracle_accounts(mut self, oracle_accounts: Vec<u8>) -> Self {
		self.oracle_accounts = oracle_accounts.into_iter().map(account).collect();
		self
	}

	pub(crate) fn with_accounts(mut self, accounts: Vec<u8>) -> Self {
		self.accounts = accounts.into_iter().map(account).collect();
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
