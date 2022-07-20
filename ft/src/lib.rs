/*!
Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

const SVG_TOKEN_ICON: &str = "data:image/png;base64,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";
const TOTAL_SUPPLY: Balance = 100_000_000_000_000_000_000_000_000;

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new_default_meta(owner_id: ValidAccountId) -> Self {
        Self::new(
            owner_id,
            U128(TOTAL_SUPPLY),
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "DOJO".to_string(),
                symbol: "DOJO".to_string(),
                icon: Some(SVG_TOKEN_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 18
            },
        )
    }

    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(
        owner_id: ValidAccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
        };
        this.token.internal_register_account(owner_id.as_ref());
        this.token.internal_deposit(owner_id.as_ref(), total_supply.into());
        this
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

    use super::*;

    const TOTAL_SUPPLY: Balance = 100_000_000_000_000_000_000_000_000;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_paras_meta(accounts(1).into(), TOTAL_SUPPLY.into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_paras_meta(accounts(2).into(), TOTAL_SUPPLY.into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
