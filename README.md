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
