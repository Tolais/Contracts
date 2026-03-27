use soroban_sdk::{Env, Address, Vec};

/// Validates that all DAO members have signed
pub fn validate_signatures(
    _e: &Env,
    dao_members: Vec<Address>,
    provided_sigs: Vec<Address>,
) -> Result<(), bool> {
    if dao_members.len() != provided_sigs.len() {
        return Err(false);
    }

    for member in dao_members.iter() {
        if !provided_sigs.iter().any(|s| s == member) {
            return Err(false);
        }
    }

    Ok(())
}