use soroban_sdk::Env;

const MAX_FEE_KEY: &str = "MAX_FEE_LIMIT";

// Set maximum allowed fee (stroops)
pub fn set_max_fee(e: &Env, fee: i128) {
    e.storage().instance().set(&MAX_FEE_KEY, &fee);
}

// Get max fee (safe default)
pub fn get_max_fee(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&MAX_FEE_KEY)
        .unwrap_or(50_000_000) // default: 5 XLM
}