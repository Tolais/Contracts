# On-Chain Vesting Certificate Registry

## Overview

The On-Chain Vesting Certificate Registry transforms the Vesting-Vault from a simple payment tool into a comprehensive "Career Infrastructure" layer for the Stellar developer ecosystem. This registry serves as a central "Source of Truth" for all completed vests, enabling Web3 Job Boards to verify a candidate's "Proof of Work" and "Loyalty."

## Features

### Core Functionality

1. **Automatic Certificate Registration**: When a user completes their 4-year vesting journey, their data is automatically moved to the registry
2. **Proof of Work Verification**: Authorized verifiers can attest to the quality and type of work performed
3. **Loyalty Scoring**: Sophisticated algorithm calculates loyalty based on vesting behavior
4. **Rich Query Interface**: Web3 Job Boards can query certificates by multiple criteria

### Data Structures

#### CompletedVestCertificate
```rust
pub struct CompletedVestCertificate {
    pub vault_id: u64,
    pub beneficiary: Address,
    pub original_vault: Vault,
    pub completion_timestamp: u64,
    pub total_claimed: i128,
    pub total_assets: i128,
    pub asset_types: Vec<Address>,
    pub loyalty_score: u32,        // 0-1000 (1000 = perfect loyalty)
    pub proof_of_work_verified: bool,
    pub certificate_id: U256,
    pub metadata_uri: Option<String>,
}
```

#### LoyaltyMetrics
```rust
pub struct LoyaltyMetrics {
    pub total_vesting_duration: u64,
    pub actual_completion_time: u64,
    pub early_claims_count: u32,
    pub missed_milestones: u32,
    pub performance_cliffs_passed: u32,
    pub stake_duration: u64,
}
```

#### WorkVerification
```rust
pub struct WorkVerification {
    pub verified_by: Address,
    pub verification_timestamp: u64,
    pub work_type: String,        // "development", "research", "community", etc.
    pub impact_score: u32,        // 0-100
    pub verification_data: String, // IPFS hash or similar
}
```

## Loyalty Score Algorithm

The loyalty score is calculated based on multiple factors:

1. **Timing Bonus**: Perfect timing (completing exactly at end_time) gets maximum points
2. **Early Completion Penalty**: Completing significantly early reduces score
3. **Early Claims Penalty**: Claiming before full vesting reduces score
4. **Milestone Adherence**: Missing milestones reduces score
5. **Staking Bonus**: Longer staking duration increases score

Score ranges from 0-1000, where 1000 represents perfect loyalty.

## Query Interface

### CertificateQuery
```rust
pub struct CertificateQuery {
    pub beneficiary: Option<Address>,
    pub work_type: Option<String>,
    pub min_loyalty_score: Option<u32>,
    pub time_range_start: Option<u64>,
    pub time_range_end: Option<u64>,
    pub verified_only: Option<bool>,
}
```

### Query Examples

1. **Get all verified certificates for a developer**:
   ```rust
   let query = CertificateQuery {
       beneficiary: Some(developer_address),
       verified_only: Some(true),
       work_type: None,
       min_loyalty_score: None,
       time_range_start: None,
       time_range_end: None,
   };
   ```

2. **Find high-loyalty developers with specific expertise**:
   ```rust
   let query = CertificateQuery {
       beneficiary: None,
       work_type: Some("development".to_string()),
       min_loyalty_score: Some(900),
       verified_only: Some(true),
       time_range_start: None,
       time_range_end: None,
   };
   ```

## Integration Points

### Automatic Registration
The registry automatically registers certificates when:
- A vault's end_time has passed
- All tokens have been claimed
- The certificate hasn't been registered before

This happens in the `check_and_register_certificate` function called after each claim.

### Verification System
Authorized verifiers can attest to work quality:
- Job boards can verify developer contributions
- Project sponsors can verify work completion
- Community members can verify impact

### Indexing System
The registry maintains efficient indexes for:
- Loyalty score ranges (grouped by 100s)
- Completion time (by year)
- Work types
- Beneficiary addresses

## Security Considerations

1. **Authorization**: Only authorized verifiers can attest to work quality
2. **Immutable Records**: Once issued, certificates cannot be modified
3. **Duplicate Prevention**: Each vault can only generate one certificate
4. **Verification Integrity**: Work verification data is stored immutably

## Gas Optimization

1. **Efficient Indexing**: Uses bucket-based indexing for common queries
2. **Lazy Registration**: Certificates are only created when needed
3. **Pagination**: Query results support pagination for large datasets
4. **Selective Verification**: Only verified certificates show in job board queries

## Use Cases for Web3 Job Boards

1. **Developer Reputation**: Query certificates to build reputation profiles
2. **Skill Verification**: Filter by verified work types and impact scores
3. **Loyalty Assessment**: Use loyalty scores to assess commitment
4. **Experience Verification**: Verify completion of long-term projects
5. **Community Impact**: Assess contributions through verification data

## Future Enhancements

1. **Cross-Chain Portability**: Export certificates to other chains
2. **Dynamic Scoring**: Machine learning-based loyalty scoring
3. **Social Features**: Developer endorsements and recommendations
4. **Integration APIs**: REST/GraphQL endpoints for external services
5. **Analytics Dashboard**: Insights into developer ecosystem

## Implementation Details

The certificate registry is implemented as a module within the main vesting contract, sharing storage and authentication mechanisms. It leverages the existing vault infrastructure while adding new functionality for career infrastructure.

All certificate operations emit events for off-chain indexing and real-time updates by web services and job boards.
