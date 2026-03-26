use soroban_sdk::Env;

use crate::errors::codes::Error;
use super::config::get_max_fee;

// Enforce fee ceiling
pub fn check_fee(e: &Env, provided_fee: i128) -> Result<(), Error> {
    let max_fee = get_max_fee(e);

    if provided_fee > max_fee {
        return Err(Error::GasFeeTooHigh);
    }

    Ok(())
}