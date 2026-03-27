use soroban_sdk::{Env, Address, Vec};

const DAO_MEMBERS_KEY: &str = "DAO_MEMBERS";
const COLD_STORAGE_KEY: &str = "COLD_STORAGE";

pub fn set_dao_members(e: &Env, members: Vec<Address>) {
    e.storage().instance().set(&DAO_MEMBERS_KEY, &members);
}

pub fn get_dao_members(e: &Env) -> Vec<Address> {
    e.storage().instance().get(&DAO_MEMBERS_KEY).unwrap_or(Vec::new(e))
}

pub fn set_cold_storage(e: &Env, vault: Address) {
    e.storage().instance().set(&COLD_STORAGE_KEY, &vault);
}

pub fn get_cold_storage(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&COLD_STORAGE_KEY)
        .expect("Cold storage not set")
}