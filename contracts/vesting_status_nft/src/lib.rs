#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec, symbol_short};

mod vesting_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/vesting_contracts.wasm"
    );
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    VestingContract,
    UserBadge(Address),
}

#[contract]
pub struct VestingStatusNFT;

#[contractimpl]
impl VestingStatusNFT {
    pub fn initialize(env: Env, admin: Address, vesting_contract: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::VestingContract, &vesting_contract);
    }

    pub fn mint(env: Env, user: Address) {
        let vesting_addr: Address = env.storage().instance().get(&DataKey::VestingContract).expect("Not initialized");
        
        // Only the vesting contract itself should be able to trigger a mint (automated trigger)
        vesting_addr.require_auth();

        if env.storage().instance().has(&DataKey::UserBadge(user.clone())) {
            return; // Already has a badge
        }

        env.storage().instance().set(&DataKey::UserBadge(user.clone()), &true);
        
        // Emit event for minting
        env.events().publish(
            (symbol_short!("mint"), user),
            ()
        );
    }

    pub fn get_level(env: Env, user: Address) -> u32 {
        let vesting_addr: Address = env.storage().instance().get(&DataKey::VestingContract).expect("Not initialized");
        let client = vesting_contract::Client::new(&env, &vesting_addr);
        
        let vault_ids = client.get_user_vaults(&user);
        if vault_ids.is_empty() {
            return 0;
        }

        let mut total_amount: i128 = 0;
        let mut total_released: i128 = 0;

        for id in vault_ids.iter() {
            let vault = client.get_vault(&id);
            total_amount += vault.total_amount;
            total_released += vault.released_amount;
        }

        if total_amount == 0 {
            return 0;
        }

        let percentage = (total_released * 100) / total_amount;

        if percentage >= 100 {
            4
        } else if percentage >= 75 {
            3
        } else if percentage >= 50 {
            2
        } else if percentage >= 25 {
            1
        } else {
            0
        }
    }

    pub fn metadata(env: Env, user: Address) -> String {
        let level = Self::get_level(env.clone(), user.clone());
        match level {
            0 => String::from_str(&env, "Vesting Badge - Level 0: Trainee"),
            1 => String::from_str(&env, "Vesting Badge - Level 1: Steady Hand"),
            2 => String::from_str(&env, "Vesting Badge - Level 2: Loyal Contributor"),
            3 => String::from_str(&env, "Vesting Badge - Level 3: Senior Stakeholder"),
            4 => String::from_str(&env, "Vesting Badge - Level 4: Master of Loyalty"),
            _ => String::from_str(&env, "Vesting Badge - Level Unknown"),
        }
    }
}

#[cfg(test)]
mod test;
