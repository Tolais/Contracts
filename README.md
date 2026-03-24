## Deployed Contract
- **Network:** Stellar Testnet
- **Contract ID:** CD6OGC46OFCV52IJQKEDVKLX5ASA3ZMSTHAAZQIPDSJV6VZ3KUJDEP4D

## Gas Costs

| Operation | Estimated Cost (XLM) |
|-----------|---------------------|
| Create Vault | ~0.05 XLM |
| Claim | ~0.01 XLM |
| Propose Governance Action | ~0.02 XLM |
| Vote on Proposal | ~0.01 XLM |
| Execute Proposal | ~0.02 XLM |

*Note: These are estimated gas costs based on contract complexity. Actual costs may vary depending on network conditions and specific operation parameters.*

## Defensive Governance System

This contract implements a **Defensive Governance** system with **Consent Logic** to protect beneficiaries from malicious admin actions. The system shifts power from a "Dictatorial Admin" to a "Collaborative Ecosystem."

### Key Features

#### 72-Hour Challenge Period
- All major admin actions require a 72-hour challenge period before execution
- During this period, beneficiaries can vote to veto the proposal
- Proposals can only be executed after the challenge period ends

#### 51% Veto Threshold
- If more than 51% of the total locked token value votes "No" on a proposal, it is automatically cancelled
- Voting power is proportional to the amount of tokens locked in vaults
- This ensures beneficiaries with significant stakes have meaningful influence

#### Governable Actions
The following admin actions now require governance approval:

1. **Admin Rotation** - Changing the contract administrator
2. **Contract Upgrade** - Migrating to a new contract version
3. **Emergency Pause** - Pausing contract operations

### How It Works

1. **Proposal Creation**: Admin proposes an action using `propose_*` functions
2. **Challenge Period**: 72-hour window for beneficiaries to review and vote
3. **Voting**: Beneficiaries vote using their locked token value as voting power
4. **Execution**: If veto threshold isn't reached, the action executes automatically

### Voting Power Calculation

- **Voting Power** = Total tokens in vaults - Already claimed tokens
- Only beneficiaries with active vaults can vote
- Voting power decreases as tokens are claimed from vaults

### API Functions

#### Governance Functions
- `propose_admin_rotation(new_admin: Address) -> u64` - Propose changing admin
- `propose_contract_upgrade(new_contract: Address) -> u64` - Propose contract upgrade
- `propose_emergency_pause(pause_state: bool) -> u64` - Propose pause/resume
- `vote_on_proposal(proposal_id: u64, is_yes: bool)` - Vote on a proposal
- `execute_proposal(proposal_id: u64)` - Execute a successful proposal

#### Query Functions
- `get_proposal_info(proposal_id: u64) -> GovernanceProposal` - Get proposal details
- `get_voter_power(voter: Address) -> i128` - Get voting power of an address
- `get_total_locked() -> i128` - Get total locked token value

### Security Benefits

- **Prevents malicious admin actions** through community veto power
- **Ensures transparency** with all proposals publicly visible
- **Protects investor interests** by giving token holders governance rights
- **Maintains operational flexibility** while adding security layers
- **Provides decentralized decision-making** on critical contract changes

*Note: These are estimated gas costs based on contract complexity. Actual costs may vary depending on network conditions and specific operation parameters.*


## Auto-Stake Feature

### How it works

Tokens stay locked inside the Vesting Vault at all times. When a beneficiary calls `auto_stake`, the vault makes a synchronous cross-contract call to a whitelisted staking contract, registering the vault's current locked balance as an active stake record. No token transfer occurs — the staking contract holds only the record, not the tokens.

### Staking lifecycle

```
Unstaked ──► auto_stake() ──► Staked
                                 │
                    manual_unstake() or revoke_vault()
                                 │
                              Unstaked
                                 │
                    (if revoked) treasury transfer
```

### Yield mechanics

Yield accrues on the staking contract against the beneficiary/vault pair. The beneficiary calls `claim_yield(vault_id)` on the vesting contract, which:

1. Calls `claim_yield_for(beneficiary, vault_id)` on the staking contract to get the accrued amount.
2. Transfers that amount from the staking contract's address to the beneficiary.
3. Resets `accumulated_yield` to zero.

Yield is claimable at any time while the vault is staked and has not been revoked.

### Revocation flow

1. Admin calls `revoke_vault(vault_id, treasury)`.
2. If the vault is currently staked, `do_unstake` is called first — the staking contract's record is cleared.
3. The vault is marked revoked in `RevokedVaults` storage.
4. All remaining unvested tokens are transferred to `treasury`.
5. The vault is frozen; no further claims or yield withdrawals are possible.

### Security assumptions

| What the vault trusts | What the vault verifies |
|---|---|
| Staking contract correctly records/clears stakes | Contract address is on the whitelist before any call |
| Staking contract holds yield tokens for payout | Yield transfer uses the staking contract as `from` address |
| Staking contract does not transfer vault tokens | No token transfer is initiated by the vault during staking |

### Integration guide

1. Deploy your staking contract implementing `stake_tokens`, `unstake_tokens`, `claim_yield_for`.
2. Call `add_staking_contract(staking_contract_id)` on the vesting contract (admin only).
3. Authorise the vesting contract address as a caller on the staking contract.
4. Beneficiaries can now call `auto_stake(vault_id, staking_contract_id)`.

To remove a staking contract from the whitelist: `remove_staking_contract(staking_contract_id)`.

### Error reference

| Error | Description |
|---|---|
| `AlreadyStaked` | Vault is already registered as a stake — unstake first |
| `NotStaked` | Operation requires the vault to be staked |
| `InsufficientBalance` | Vault has zero locked balance; nothing to stake |
| `UnauthorizedStakingContract` | Staking contract address is not whitelisted |
| `BeneficiaryRevoked` | Vault has been revoked; yield can no longer be claimed |
| `CrossContractCallFailed` | The cross-contract call to the staking contract failed |
| `UnstakeBeforeRevocationFailed` | Auto-unstake during revocation could not complete |
| `YieldClaimFailed` | The yield claim call to the staking contract failed |
| `Vault is irrevocable` | Cannot revoke a vault marked irrevocable |

---

## Inheritance & Succession (Dead-Man's Switch)

### Overview

Locked assets in a vesting vault are at risk of permanent loss if the primary beneficiary loses their private key or passes away. The inheritance system solves this by letting the primary nominate a backup address and an inactivity timer. If the primary makes no on-chain vault interactions for the full timer duration, the backup can claim ownership — preventing assets from being locked forever.

This implements a Dead-Man's Switch: the primary must periodically "check in" by interacting with their vault. Silence for long enough triggers the succession path.

### Succession lifecycle

```
None
 │
 │  nominate_backup()
 ▼
Nominated  ◄──────────────────────────────────────────────────────────────────┐
 │                                                                             │
 │  (primary inactive for switch_duration)                                    │
 │  initiate_succession_claim()  [called by backup]                           │
 ▼                                                                             │
ClaimPending ──── primary acts (claim_tokens, auto_stake, etc.) ──────────────┘
 │                                                                             │
 │  (challenge_window elapses, primary did not cancel)                        │
 │  finalise_succession()  [called by backup]                                 │
 ▼
Succeeded  (irreversible — vault.owner = backup)
```

### Dead-Man's Switch mechanics

Every vault function the primary calls (`claim_tokens`, `auto_stake`, `manual_unstake`, `claim_yield`) invokes `update_activity()` internally. This resets the inactivity timer to the current block timestamp.

The backup can only initiate a claim when:

```
now - last_activity >= switch_duration
```

If the primary acts at any point — even during an active claim — the claim is cancelled and the timer resets.

### Challenge window

After the backup calls `initiate_succession_claim`, a challenge window opens. During this window the primary can call `cancel_succession_claim` to abort the succession and reset to `Nominated`. This protects against premature or malicious claims.

The backup can only finalise succession when:

```
now - claimed_at >= challenge_window
```

### Configuration guide

| Parameter | Minimum | Maximum | Recommended |
|---|---|---|---|
| `switch_duration` | 30 days | 730 days | 180 days |
| `challenge_window` | 1 day | 30 days | 7 days |

Choose a `switch_duration` long enough that normal inactivity (holidays, illness) does not trigger succession, but short enough to protect against permanent key loss.

### Security assumptions

| What the vault verifies | Notes |
|---|---|
| Caller is the vault owner before nominating/revoking | `require_auth()` on `vault.owner` |
| Caller is the backup before claiming/finalising | Checked against stored backup address |
| Timer has fully elapsed before claim is allowed | `elapsed >= switch_duration` (not `>`) |
| Challenge window has fully elapsed before finalise | `elapsed >= challenge_window` (not `>`) |
| Backup != primary | Validated before storing |
| Succession is irreversible once finalised | `Succeeded` state has no revert path |
| Cannot nominate after succession | `AlreadySucceeded` guard in `nominate_backup` |

### Interaction with staking and vesting

- Staking (`auto_stake`, `manual_unstake`, `claim_yield`) all trigger the activity heartbeat — they count as primary activity.
- Succession transfers `vault.owner` to the backup. All future vault interactions (claims, staking, revocation) require the new owner's signature.
- Vesting schedules are unaffected — the schedule continues on the same timeline with the new owner.
- Revocation by the admin is independent of succession state.

### Error reference

| Error | Description | Remediation |
|---|---|---|
| `BackupEqualsPrimary` | Backup address is the same as the vault owner | Choose a different backup address |
| `BackupIsZeroAddress` | Backup address is the zero address | Provide a valid account address |
| `SwitchDurationBelowMinimum` | `switch_duration` < 30 days | Use at least `MIN_SWITCH_DURATION` (2,592,000 s) |
| `SwitchDurationAboveMaximum` | `switch_duration` > 730 days | Use at most `MAX_SWITCH_DURATION` (63,072,000 s) |
| `ChallengeWindowOutOfRange` | `challenge_window` outside [1 day, 30 days] | Use a value within the allowed range |
| `NoPlanNominated` | No backup has been nominated | Call `nominate_backup` first |
| `AlreadySucceeded` | Succession has been finalised | State is permanent; no further changes possible |
| `ClaimAlreadyPending` | A claim is already in progress | Wait for the claim to be finalised or cancelled |
| `SwitchTimerNotElapsed` | Primary was active within `switch_duration` | Wait for the full inactivity period to elapse |
| `ChallengeWindowNotElapsed` | Challenge window has not closed yet | Wait for `challenge_window` seconds after the claim |
| `CallerIsNotBackup` | Caller is not the nominated backup | Only the backup address can initiate or finalise claims |
| `CallerIsNotPrimary` | Caller is not the current vault owner | Only the primary can cancel claims or revoke the backup |
| `RevocationBlockedDuringClaim` | Cannot revoke backup while a claim is pending | Cancel the claim first, then revoke |
