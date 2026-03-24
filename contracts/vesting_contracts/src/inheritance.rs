use soroban_sdk::{contracttype, Address, Env, Symbol};
use crate::DataKey;

pub const MIN_SWITCH_DURATION: u64 = 30 * 24 * 60 * 60;
pub const MAX_SWITCH_DURATION: u64 = 730 * 24 * 60 * 60;
pub const MIN_CHALLENGE_WINDOW: u64 = 1 * 24 * 60 * 60;
pub const MAX_CHALLENGE_WINDOW: u64 = 30 * 24 * 60 * 60;

#[contracttype]
#[derive(Clone, PartialEq)]
pub struct NominatedData {
    pub backup: Address,
    pub switch_duration: u64,
    pub challenge_window: u64,
    pub last_activity: u64,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub struct ClaimPendingData {
    pub backup: Address,
    pub claimed_at: u64,
    pub challenge_window: u64,
    pub switch_duration: u64,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub struct SucceededData {
    pub new_owner: Address,
    pub succeeded_at: u64,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum SuccessionState {
    None,
    Nominated(NominatedData),
    ClaimPending(ClaimPendingData),
    Succeeded(SucceededData),
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub struct SuccessionView {
    pub primary: Address,
    pub backup: Option<Address>,
    pub switch_duration: u64,
    pub last_activity: u64,
    pub time_remaining: Option<u64>,
    pub state: SuccessionState,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum InheritanceError {
    BackupEqualsPrimary,
    BackupIsZeroAddress,
    SwitchDurationBelowMinimum,
    SwitchDurationAboveMaximum,
    ChallengeWindowOutOfRange,
    NoPlanNominated,
    AlreadySucceeded,
    ClaimAlreadyPending,
    SwitchTimerNotElapsed,
    ChallengeWindowNotElapsed,
    CallerIsNotBackup,
    CallerIsNotPrimary,
    RevocationBlockedDuringClaim,
}

fn succession_key(vault_id: u64) -> DataKey {
    DataKey::VaultSuccession(vault_id)
}

pub fn get_succession_state(env: &Env, vault_id: u64) -> SuccessionState {
    env.storage()
        .instance()
        .get(&succession_key(vault_id))
        .unwrap_or(SuccessionState::None)
}

pub fn nominate_backup(
    env: &Env,
    vault_id: u64,
    primary: &Address,
    backup: Address,
    switch_duration: u64,
    challenge_window: u64,
) {
    primary.require_auth();
    let current = get_succession_state(env, vault_id);
    if matches!(current, SuccessionState::Succeeded(_)) {
        panic!("AlreadySucceeded");
    }
    if matches!(current, SuccessionState::ClaimPending(_)) {
        panic!("ClaimAlreadyPending");
    }
    if &backup == primary {
        panic!("BackupEqualsPrimary");
    }
    if switch_duration < MIN_SWITCH_DURATION {
        panic!("SwitchDurationBelowMinimum");
    }
    if switch_duration > MAX_SWITCH_DURATION {
        panic!("SwitchDurationAboveMaximum");
    }
    if challenge_window < MIN_CHALLENGE_WINDOW || challenge_window > MAX_CHALLENGE_WINDOW {
        panic!("ChallengeWindowOutOfRange");
    }
    let now = env.ledger().timestamp();
    let state = SuccessionState::Nominated(NominatedData {
        backup: backup.clone(),
        switch_duration,
        challenge_window,
        last_activity: now,
    });
    env.storage().instance().set(&succession_key(vault_id), &state);
    env.events().publish(
        (Symbol::new(env, "backup_nominated"), vault_id),
        (primary.clone(), backup, switch_duration, challenge_window),
    );
}

pub fn revoke_backup(env: &Env, vault_id: u64, primary: &Address) {
    primary.require_auth();
    let current = get_succession_state(env, vault_id);
    match current {
        SuccessionState::None => panic!("NoPlanNominated"),
        SuccessionState::ClaimPending(_) => panic!("RevocationBlockedDuringClaim"),
        SuccessionState::Succeeded(_) => panic!("AlreadySucceeded"),
        SuccessionState::Nominated(_) => {}
    }
    env.storage()
        .instance()
        .set(&succession_key(vault_id), &SuccessionState::None);
    env.events().publish(
        (Symbol::new(env, "backup_revoked"), vault_id),
        primary.clone(),
    );
}

pub fn update_activity(env: &Env, vault_id: u64) {
    let current = get_succession_state(env, vault_id);
    let now = env.ledger().timestamp();
    match current {
        SuccessionState::Nominated(data) => {
            let updated = SuccessionState::Nominated(NominatedData {
                last_activity: now,
                ..data
            });
            env.storage().instance().set(&succession_key(vault_id), &updated);
        }
        SuccessionState::ClaimPending(data) => {
            let backup = data.backup.clone();
            let updated = SuccessionState::Nominated(NominatedData {
                backup: data.backup,
                switch_duration: data.switch_duration,
                challenge_window: data.challenge_window,
                last_activity: now,
            });
            env.storage().instance().set(&succession_key(vault_id), &updated);
            env.events().publish(
                (Symbol::new(env, "claim_cancelled"), vault_id),
                backup,
            );
        }
        SuccessionState::None | SuccessionState::Succeeded(_) => {}
    }
}

pub fn initiate_succession_claim(env: &Env, vault_id: u64, caller: &Address) {
    caller.require_auth();
    let current = get_succession_state(env, vault_id);
    match current {
        SuccessionState::None => panic!("NoPlanNominated"),
        SuccessionState::ClaimPending(_) => panic!("ClaimAlreadyPending"),
        SuccessionState::Succeeded(_) => panic!("AlreadySucceeded"),
        SuccessionState::Nominated(data) => {
            if caller != &data.backup {
                panic!("CallerIsNotBackup");
            }
            let now = env.ledger().timestamp();
            let elapsed = now.saturating_sub(data.last_activity);
            if elapsed < data.switch_duration {
                panic!("SwitchTimerNotElapsed");
            }
            let backup = data.backup.clone();
            let state = SuccessionState::ClaimPending(ClaimPendingData {
                backup: data.backup,
                claimed_at: now,
                challenge_window: data.challenge_window,
                switch_duration: data.switch_duration,
            });
            env.storage().instance().set(&succession_key(vault_id), &state);
            env.events().publish(
                (Symbol::new(env, "succession_claimed"), vault_id),
                (backup, now),
            );
        }
    }
}

pub fn finalise_succession(env: &Env, vault_id: u64, caller: &Address) -> Address {
    caller.require_auth();
    let current = get_succession_state(env, vault_id);
    match current {
        SuccessionState::None => panic!("NoPlanNominated"),
        SuccessionState::Nominated(_) => panic!("NoPlanNominated"),
        SuccessionState::Succeeded(_) => panic!("AlreadySucceeded"),
        SuccessionState::ClaimPending(data) => {
            if caller != &data.backup {
                panic!("CallerIsNotBackup");
            }
            let now = env.ledger().timestamp();
            let elapsed = now.saturating_sub(data.claimed_at);
            if elapsed < data.challenge_window {
                panic!("ChallengeWindowNotElapsed");
            }
            let new_owner = data.backup.clone();
            let state = SuccessionState::Succeeded(SucceededData {
                new_owner: data.backup,
                succeeded_at: now,
            });
            env.storage().instance().set(&succession_key(vault_id), &state);
            env.events().publish(
                (Symbol::new(env, "succession_finalised"), vault_id),
                (new_owner.clone(), now),
            );
            new_owner
        }
    }
}

pub fn cancel_succession_claim(env: &Env, vault_id: u64, primary: &Address) {
    primary.require_auth();
    let current = get_succession_state(env, vault_id);
    match current {
        SuccessionState::None => panic!("NoPlanNominated"),
        SuccessionState::Nominated(_) => panic!("NoPlanNominated"),
        SuccessionState::Succeeded(_) => panic!("AlreadySucceeded"),
        SuccessionState::ClaimPending(data) => {
            let now = env.ledger().timestamp();
            let backup = data.backup.clone();
            let state = SuccessionState::Nominated(NominatedData {
                backup: data.backup,
                switch_duration: data.switch_duration,
                challenge_window: data.challenge_window,
                last_activity: now,
            });
            env.storage().instance().set(&succession_key(vault_id), &state);
            env.events().publish(
                (Symbol::new(env, "claim_cancelled"), vault_id),
                (primary.clone(), backup),
            );
        }
    }
}

pub fn get_succession_status(env: &Env, vault_id: u64, primary: Address) -> SuccessionView {
    let state = get_succession_state(env, vault_id);
    let now = env.ledger().timestamp();
    match &state {
        SuccessionState::Nominated(data) => {
            let elapsed = now.saturating_sub(data.last_activity);
            let time_remaining = if elapsed >= data.switch_duration {
                Some(0)
            } else {
                Some(data.switch_duration - elapsed)
            };
            SuccessionView {
                primary,
                backup: Some(data.backup.clone()),
                switch_duration: data.switch_duration,
                last_activity: data.last_activity,
                time_remaining,
                state,
            }
        }
        SuccessionState::ClaimPending(data) => SuccessionView {
            primary,
            backup: Some(data.backup.clone()),
            switch_duration: data.switch_duration,
            last_activity: 0,
            time_remaining: Some(0),
            state,
        },
        SuccessionState::Succeeded(_) | SuccessionState::None => SuccessionView {
            primary,
            backup: Option::None,
            switch_duration: 0,
            last_activity: 0,
            time_remaining: Option::None,
            state,
        },
    }
}
