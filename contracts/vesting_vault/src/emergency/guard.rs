use soroban_sdk::{Env, Address, Vec};
use crate::errors::codes::Error;
use crate::shared::emergency::config::get_dao_members;

/// Validates that all DAO members have signed
pub fn validate_signatures(
    _e: &Env,
    dao_members: Vec<Address>,
    provided_sigs: Vec<Address>,
) -> Result<(), Error> {
    if dao_members.len() != provided_sigs.len() {
        return Err(Error::CriticalNuclearTriggered);
    }

    for member in dao_members.iter() {
        if !provided_sigs.iter().any(|s| s == member) {
            return Err(Error::CriticalNuclearTriggered);
        }
    }

    Ok(())
}