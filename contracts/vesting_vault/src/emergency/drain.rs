use soroban_sdk::{Env, Address};

use crate::errors::codes::Error;
use crate::shared::emergency::{config, guard};

pub fn drain_to_safety(
    e: &Env,
    provided_sigs: Vec<Address>,
    unvested_tokens: Vec<(Address, i128)>, // (user, amount)
) -> Result<(), Error> {
    let dao_members = config::get_dao_members(e);

    // ✅ require 100% council approval
    guard::validate_signatures(e, dao_members, provided_sigs)?;

    let cold_storage = config::get_cold_storage(e);

    // move all unvested tokens to cold storage
    for (_user, amount) in unvested_tokens.iter() {
        // pseudo-transfer logic
        // in reality call transfer to cold_storage
        e.log(format!("Transferred {} tokens to cold storage", amount));
    }

    Ok(())
}