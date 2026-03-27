use soroban_sdk::{Env, Address, Vec};

use crate::errors::codes::Error;
use crate::fee_guard::check_fee;

pub fn execute_batch_claim(
    e: &Env,
    keeper: Address,
    users: Vec<Address>,
    provided_fee: i128,
) -> Result<(), Error> {
    keeper.require_auth();

    // 🔥 Fail-safe check
    check_fee(e, provided_fee)?;

    for user in users.iter() {
        process_claim(e, user)?;
    }

    Ok(())
}

// placeholder
fn process_claim(_e: &Env, _user: Address) -> Result<(), Error> {
    Ok(())
}