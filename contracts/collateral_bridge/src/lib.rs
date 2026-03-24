#![no_std]
use soroban_sdk::{
    contract,
    contractimpl,
    contracttype,
    token,
    Address,
    Env,
    IntoVal,
    Map,
    Symbol,
    Val,
    Vec,
    String,
};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Lien {
    pub vault_id: u64,
    pub lender: Address,
    pub locked_amount: i128,
    pub loan_amount: i128,
    pub interest_rate: u32, // basis points (10000 = 100%)
    pub maturity_time: u64,
    pub is_active: bool,
    pub created_time: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum CollateralDataKey {
    Admin,
    VestingContract,
    LienCount,
    Lien(u64),
    VaultLiens(u64), // vault_id -> Vec<lien_id>
    LenderLiens(Address), // lender -> Vec<lien_id>
    IsPaused,
}

#[contracttype]
pub struct LienCreated {
    pub lien_id: u64,
    pub vault_id: u64,
    pub lender: Address,
    pub locked_amount: i128,
    pub loan_amount: i128,
}

#[contracttype]
pub struct LienClaimed {
    pub lien_id: u64,
    pub vault_id: u64,
    pub lender: Address,
    pub claimed_amount: i128,
}

#[contract]
pub struct CollateralBridge;

#[contractimpl]
impl CollateralBridge {
    pub fn initialize(env: Env, admin: Address, vesting_contract: Address) {
        if env.storage().instance().has(&CollateralDataKey::Admin) {
            panic!("Already initialized");
        }

        admin.require_auth();
        env.storage().instance().set(&CollateralDataKey::Admin, &admin);
        env.storage().instance().set(&CollateralDataKey::VestingContract, &vesting_contract);
        env.storage().instance().set(&CollateralDataKey::LienCount, &0u64);
        env.storage().instance().set(&CollateralDataKey::IsPaused, &false);
    }

    pub fn create_lien(
        env: Env,
        vault_id: u64,
        lender: Address,
        locked_amount: i128,
        loan_amount: i128,
        interest_rate: u32,
        maturity_time: u64
    ) -> u64 {
        Self::require_not_paused(&env);

        // Get vault owner for authorization
        let vesting_contract = Self::get_vesting_contract(&env);
        let vault = Self::get_vault_info(&env, &vesting_contract, vault_id);

        // Vault owner must authorize the lien creation
        vault.owner.require_auth();

        // Validate inputs
        if locked_amount <= 0 || loan_amount <= 0 {
            panic!("Amounts must be positive");
        }
        if interest_rate > 50000 {
            // Max 500% interest rate
            panic!("Interest rate too high");
        }
        if maturity_time <= env.ledger().timestamp() {
            panic!("Invalid maturity time");
        }

        // Check if vault has enough unvested tokens to lock
        let total_unvested = vault.total_amount - vault.released_amount;
        let currently_locked = Self::get_total_locked_for_vault(&env, vault_id);
        let available_to_lock = total_unvested - currently_locked;

        if locked_amount > available_to_lock {
            panic!("Insufficient unvested tokens to lock");
        }

        // Create lien
        let lien_id = Self::increment_lien_count(&env);
        let lien = Lien {
            vault_id,
            lender: lender.clone(),
            locked_amount,
            loan_amount,
            interest_rate,
            maturity_time,
            is_active: true,
            created_time: env.ledger().timestamp(),
        };

        // Store lien
        env.storage().instance().set(&CollateralDataKey::Lien(lien_id), &lien);

        // Update indexes
        Self::add_vault_lien(&env, vault_id, lien_id);
        Self::add_lender_lien(&env, &lender, lien_id);

        // Emit event
        env.events().publish(("lien_created", lien_id), LienCreated {
            lien_id,
            vault_id,
            lender: lender.clone(),
            locked_amount,
            loan_amount,
        });

        lien_id
    }

    pub fn claim_collateral(env: Env, lien_id: u64) -> i128 {
        Self::require_not_paused(&env);

        let mut lien = Self::get_lien(&env, lien_id);

        if !lien.is_active {
            panic!("Lien not active");
        }

        let now = env.ledger().timestamp();
        if now < lien.maturity_time {
            panic!("Lien not matured");
        }

        // Lender must authorize the claim
        lien.lender.require_auth();

        // Get vault info
        let vesting_contract = Self::get_vesting_contract(&env);
        let vault = Self::get_vault_info(&env, &vesting_contract, lien.vault_id);

        // Calculate claimable amount (vested tokens that are not already released)
        let vested_amount = Self::calculate_vested_amount(&env, &vesting_contract, lien.vault_id);
        let already_released = vault.released_amount;
        let claimable_from_vault = vested_amount - already_released;

        // The lender can claim up to the locked amount or the available vested amount, whichever is less
        let claim_amount = claimable_from_vault.min(lien.locked_amount);

        if claim_amount <= 0 {
            panic!("No tokens available to claim");
        }

        // Mark lien as inactive
        lien.is_active = false;
        env.storage().instance().set(&CollateralDataKey::Lien(lien_id), &lien);

        // Transfer tokens from vesting contract to lender
        // This requires the vesting contract to support a "claim_by_lender" function
        let token_client = token::Client::new(&env, &Self::get_token(&env, &vesting_contract));

        // For now, we'll simulate this by having the bridge contract claim the tokens
        // and then transfer to the lender. In a real implementation, the vesting contract
        // would need to be modified to support lender claims.

        // Emit event
        env.events().publish(("lien_claimed", lien_id), LienClaimed {
            lien_id,
            vault_id: lien.vault_id,
            lender: lien.lender.clone(),
            claimed_amount: claim_amount,
        });

        claim_amount
    }

    pub fn release_lien(env: Env, lien_id: u64) {
        Self::require_not_paused(&env);

        let mut lien = Self::get_lien(&env, lien_id);

        if !lien.is_active {
            panic!("Lien not active");
        }

        // Get vault info
        let vesting_contract = Self::get_vesting_contract(&env);
        let vault = Self::get_vault_info(&env, &vesting_contract, lien.vault_id);

        // Only vault owner can release the lien
        vault.owner.require_auth();

        // Mark lien as inactive
        lien.is_active = false;
        env.storage().instance().set(&CollateralDataKey::Lien(lien_id), &lien);
    }

    pub fn get_lien(env: Env, lien_id: u64) -> Lien {
        env.storage().instance().get(&CollateralDataKey::Lien(lien_id)).expect("Lien not found")
    }

    pub fn get_vault_liens(env: Env, vault_id: u64) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&CollateralDataKey::VaultLiens(vault_id))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_lender_liens(env: Env, lender: Address) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&CollateralDataKey::LenderLiens(lender))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_available_to_lock(env: Env, vault_id: u64) -> i128 {
        let vesting_contract = Self::get_vesting_contract(&env);
        let vault = Self::get_vault_info(&env, &vesting_contract, vault_id);

        let total_unvested = vault.total_amount - vault.released_amount;
        let currently_locked = Self::get_total_locked_for_vault(&env, vault_id);

        total_unvested - currently_locked
    }

    pub fn toggle_pause(env: Env) {
        Self::require_admin(&env);
        let paused: bool = env
            .storage()
            .instance()
            .get(&CollateralDataKey::IsPaused)
            .unwrap_or(false);
        env.storage().instance().set(&CollateralDataKey::IsPaused, &!paused);
    }

    // --- Internal Helpers ---

    fn require_admin(env: &Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&CollateralDataKey::Admin)
            .expect("Admin not set");
        admin.require_auth();
    }

    fn require_not_paused(env: &Env) {
        if env.storage().instance().get(&CollateralDataKey::IsPaused).unwrap_or(false) {
            panic!("Contract paused");
        }
    }

    fn get_vesting_contract(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&CollateralDataKey::VestingContract)
            .expect("Vesting contract not set")
    }

    fn increment_lien_count(env: &Env) -> u64 {
        let count: u64 = env.storage().instance().get(&CollateralDataKey::LienCount).unwrap_or(0);
        let new_count = count + 1;
        env.storage().instance().set(&CollateralDataKey::LienCount, &new_count);
        new_count
    }

    fn add_vault_lien(env: &Env, vault_id: u64, lien_id: u64) {
        let mut liens: Vec<u64> = env
            .storage()
            .instance()
            .get(&CollateralDataKey::VaultLiens(vault_id))
            .unwrap_or(Vec::new(env));
        liens.push_back(lien_id);
        env.storage().instance().set(&CollateralDataKey::VaultLiens(vault_id), &liens);
    }

    fn add_lender_lien(env: &Env, lender: &Address, lien_id: u64) {
        let mut liens: Vec<u64> = env
            .storage()
            .instance()
            .get(&CollateralDataKey::LenderLiens(lender.clone()))
            .unwrap_or(Vec::new(env));
        liens.push_back(lien_id);
        env.storage().instance().set(&CollateralDataKey::LenderLiens(lender.clone()), &liens);
    }

    fn get_total_locked_for_vault(env: &Env, vault_id: u64) -> i128 {
        let lien_ids: Vec<u64> = env
            .storage()
            .instance()
            .get(&CollateralDataKey::VaultLiens(vault_id))
            .unwrap_or(Vec::new(env));

        let mut total_locked = 0i128;
        for lien_id in lien_ids.iter() {
            if
                let Ok(lien) = env
                    .storage()
                    .instance()
                    .get::<CollateralDataKey, Lien>(&CollateralDataKey::Lien(*lien_id))
            {
                if lien.is_active {
                    total_locked += lien.locked_amount;
                }
            }
        }

        total_locked
    }

    fn get_lien(env: &Env, lien_id: u64) -> Lien {
        env.storage().instance().get(&CollateralDataKey::Lien(lien_id)).expect("Lien not found")
    }

    // These functions integrate with the actual vesting contract
    fn get_vault_info(env: &Env, vesting_contract: &Address, vault_id: u64) -> VaultInfo {
        // Call the vesting contract to get vault info
        let vesting_client = VestingContractClient::new(env, vesting_contract);
        let vault = vesting_client.get_vault(&vault_id);

        VaultInfo {
            total_amount: vault.total_amount,
            released_amount: vault.released_amount,
            owner: vault.owner,
        }
    }

    fn calculate_vested_amount(env: &Env, vesting_contract: &Address, vault_id: u64) -> i128 {
        // Call the vesting contract to get vested amount
        let vesting_client = VestingContractClient::new(env, vesting_contract);
        vesting_client.get_claimable_amount(&vault_id) +
            ({
                // Get the vault to add back the released amount
                let vault = vesting_client.get_vault(&vault_id);
                vault.released_amount
            })
    }

    fn get_token(env: &Env, vesting_contract: &Address) -> Address {
        // This would need to be implemented based on the vesting contract
        // For now, we'll assume there's a way to get the token address
        panic!("Token retrieval needs to be implemented based on vesting contract interface");
    }
}

// Placeholder structures - these would need to match the actual vesting contract
#[contracttype]
#[derive(Clone, Debug)]
pub struct VaultInfo {
    pub total_amount: i128,
    pub released_amount: i128,
    pub owner: Address,
}

#[cfg(test)]
mod test;
