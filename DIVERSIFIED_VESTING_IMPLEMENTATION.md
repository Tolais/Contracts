# Diversified Vesting Implementation

## Overview

This implementation adds **Diversified Vesting** functionality to the existing vesting contract system. Instead of vesting a single token, beneficiaries can now receive a basket of multiple assets (e.g., 50% Project Token, 25% XLM, 25% USDC) that vest simultaneously according to the same schedule.

## Key Benefits

### 1. Risk Reduction
- **Problem**: Single-token vesting exposes beneficiaries to high volatility
- **Solution**: Diversified baskets reduce exposure to any single asset's price movements
- **Example**: If project token drops 80%, beneficiary still has stable value from XLM/USDC portions

### 2. Attractive Compensation Packages
- **Problem**: Senior developers require stable financial planning
- **Solution**: Mix of project tokens (upside potential) + stable assets (predictable value)
- **Result**: More competitive offers for top talent

### 3. Flexible Asset Allocation
- **Junior Developer**: 30% ProjectToken, 35% XLM, 35% USDC (stability-focused)
- **Senior Developer**: 50% ProjectToken, 25% XLM, 25% USDC (balanced)
- **Executive**: 70% ProjectToken, 15% XLM, 15% USDC (upside-focused)
- **Advisor**: 40% ProjectToken, 30% XLM, 30% USDC (conservative)

## Technical Implementation

### Core Data Structures

#### AssetAllocation
```rust
#[contracttype]
#[derive(Clone)]
pub struct AssetAllocation {
    pub asset_id: Address,        // Token contract address
    pub total_amount: i128,       // Total tokens allocated
    pub released_amount: i128,    // Tokens already claimed
    pub locked_amount: i128,      // Tokens locked for collateral
    pub percentage: u32,          // Percentage in basis points (10000 = 100%)
}
```

#### Updated Vault Structure
```rust
#[contracttype]
#[derive(Clone)]
pub struct Vault {
    pub allocations: Vec<AssetAllocation>, // Basket of assets (NEW)
    pub keeper_fee: i128,
    pub staked_amount: i128,
    pub owner: Address,
    pub delegate: Option<Address>,
    pub title: String,
    pub start_time: u64,
    pub end_time: u64,
    pub creation_time: u64,
    pub step_duration: u64,
    pub is_initialized: bool,
    pub is_irrevocable: bool,
    pub is_transferable: bool,
    pub is_frozen: bool,
}
```

### Key Functions

#### 1. Vault Creation
```rust
pub fn create_vault_diversified_full(
    env: Env,
    owner: Address,
    asset_basket: Vec<AssetAllocation>,
    start_time: u64,
    end_time: u64,
    keeper_fee: i128,
    is_revocable: bool,
    is_transferable: bool,
    step_duration: u64,
    title: String,
) -> u64
```

**Features:**
- Validates asset basket percentages sum to 10000 (100%)
- Transfers all assets from admin to contract
- Creates vault with multiple asset allocations

#### 2. Diversified Claiming
```rust
pub fn claim_tokens_diversified(env: Env, vault_id: u64) -> Vec<(Address, i128)>
```

**Process:**
1. Calculate vested amount for each asset based on time elapsed
2. Determine claimable amount (vested - already released)
3. Transfer each asset to beneficiary
4. Update vault state with new released amounts
5. Return list of (asset_id, claimed_amount) pairs

#### 3. Asset Basket Validation
```rust
fn validate_asset_basket(basket: &Vec<AssetAllocation>) -> bool
```

**Validation Rules:**
- Percentages must sum to exactly 10000 (100%)
- Basket cannot be empty
- Each asset amount must be positive
- Each percentage must be between 1 and 10000

### Vesting Calculation

The system supports multiple vesting schedules, all applied uniformly across the asset basket:

#### Linear Vesting (Default)
```rust
vested_amount = (allocation.total_amount * elapsed_time) / total_duration
```

#### Step-Based Vesting
```rust
completed_steps = elapsed_time / step_duration
vested_amount = (allocation.total_amount * completed_steps) / total_steps
```

#### Milestone-Based Vesting
```rust
unlocked_percentage = sum(milestone.percentage for milestone in unlocked_milestones)
vested_amount = (allocation.total_amount * unlocked_percentage) / 100
```

#### Performance Cliff
- Oracle-based conditions must be met before any vesting begins
- Once cliff is passed, normal vesting schedule applies to all assets

## Usage Examples

### Example 1: Senior Developer Package
```rust
// Create asset basket: 50% ProjectToken, 25% XLM, 25% USDC
let mut asset_basket = vec![&env];

asset_basket.push_back(AssetAllocation {
    asset_id: project_token_address,
    total_amount: 10_000_0000000, // 10,000 tokens
    released_amount: 0,
    locked_amount: 0,
    percentage: 5000, // 50%
});

asset_basket.push_back(AssetAllocation {
    asset_id: xlm_address,
    total_amount: 2_500_0000000, // 2,500 XLM
    released_amount: 0,
    locked_amount: 0,
    percentage: 2500, // 25%
});

asset_basket.push_back(AssetAllocation {
    asset_id: usdc_address,
    total_amount: 2_500_0000000, // 2,500 USDC
    released_amount: 0,
    locked_amount: 0,
    percentage: 2500, // 25%
});

// Create 4-year vesting vault
let vault_id = client.create_vault_diversified_full(
    &beneficiary,
    &asset_basket,
    &start_time,
    &(start_time + 4 * 365 * 24 * 60 * 60), // 4 years
    &0, // no keeper fee
    &true, // revocable
    &true, // transferable
    &0, // linear vesting
    &String::from_str(&env, "Senior Dev Package"),
);
```

### Example 2: Claiming After 1 Year
```rust
// After 1 year (25% vested), beneficiary claims:
let claimed_assets = client.claim_tokens_diversified(&vault_id);

// Result:
// - 2,500 Project Tokens (25% of 10,000)
// - 625 XLM (25% of 2,500)
// - 625 USDC (25% of 2,500)
```

## Backward Compatibility

The implementation maintains full backward compatibility:

### Legacy Single-Asset Functions
```rust
pub fn claim_tokens(env: Env, vault_id: u64, claim_amount: i128) -> i128
```
- Works with single-asset vaults (allocations.len() == 1)
- Panics if used on multi-asset vaults with helpful error message

### Automatic Migration
- Old vault creation functions create single-asset allocations internally
- Existing vaults continue to work without modification
- New diversified functions are additive, not replacing

## Advanced Features

### 1. Asset-Specific Locking
```rust
pub fn lock_tokens_for_asset(env: Env, vault_id: u64, asset_id: Address, amount: i128)
pub fn unlock_tokens_for_asset(env: Env, vault_id: u64, asset_id: Address, amount: i128)
```

### 2. Asset-Specific Claiming by Lenders
```rust
pub fn claim_by_lender_for_asset(
    env: Env,
    vault_id: u64,
    lender: Address,
    asset_id: Address,
    amount: i128,
) -> i128
```

### 3. Vault Statistics
```rust
pub fn get_vault_statistics(env: Env, vault_id: u64) -> (i128, i128, i128, u32)
// Returns: (total_value, released_value, claimable_value, asset_count)
```

### 4. Asset Basket Management
```rust
pub fn get_vault_asset_basket(env: Env, vault_id: u64) -> Vec<AssetAllocation>
pub fn update_vault_asset_basket(env: Env, vault_id: u64, new_basket: Vec<AssetAllocation>)
```

## Security Considerations

### 1. Percentage Validation
- Asset percentages must sum to exactly 10000 (100%)
- Prevents over-allocation or under-allocation errors
- Enforced at vault creation and basket updates

### 2. Asset Whitelisting
- Only whitelisted tokens can be used in asset baskets
- Prevents malicious or invalid token addresses
- Admin-controlled whitelist management

### 3. Authorization
- Only vault owner can claim tokens
- Only admin can create/modify vaults
- Only authorized bridges can lock/unlock tokens

### 4. Atomic Operations
- All assets in a basket are claimed atomically
- Either all transfers succeed or all fail
- Prevents partial claim states

## Gas Optimization

### 1. Batch Operations
- Single function call claims all assets simultaneously
- Reduces transaction costs compared to individual claims
- Efficient iteration over asset allocations

### 2. Storage Efficiency
- Asset allocations stored in single Vec
- Minimal storage overhead per additional asset
- Efficient serialization/deserialization

## Testing Strategy

### 1. Unit Tests
- Asset basket validation
- Vesting calculations for each asset
- Claiming logic with multiple assets
- Error conditions and edge cases

### 2. Integration Tests
- End-to-end vault creation and claiming
- Interaction with token contracts
- Multi-asset scenarios with real tokens

### 3. Property-Based Tests
- Random asset basket generation
- Invariant checking (percentages always sum to 100%)
- Fuzz testing with various time scenarios

## Migration Guide

### For Existing Projects
1. **No immediate changes required** - existing vaults continue working
2. **Gradual adoption** - new vaults can use diversified features
3. **Optional migration** - existing vaults can be migrated if desired

### For New Projects
1. **Design asset baskets** based on compensation strategy
2. **Use diversified creation functions** for new vaults
3. **Implement diversified claiming** in frontend applications

## Future Enhancements

### 1. Dynamic Rebalancing
- Allow periodic rebalancing of asset percentages
- Maintain target allocations as token prices change
- Admin-controlled rebalancing triggers

### 2. Yield-Bearing Assets
- Support for assets that generate yield
- Automatic reinvestment or distribution options
- Integration with DeFi protocols

### 3. Cross-Chain Assets
- Support for assets on different blockchains
- Bridge integration for cross-chain transfers
- Unified claiming across multiple chains

### 4. Advanced Vesting Schedules
- Different vesting schedules per asset
- Conditional vesting based on performance metrics
- Dynamic cliff adjustments

## Conclusion

The Diversified Vesting implementation provides a powerful solution for creating more attractive and stable compensation packages. By allowing multiple assets to vest simultaneously, it reduces risk for beneficiaries while maintaining the incentive alignment benefits of traditional token vesting.

The implementation is designed to be:
- **Backward compatible** with existing systems
- **Flexible** for various compensation strategies  
- **Secure** with comprehensive validation
- **Efficient** in terms of gas usage
- **Extensible** for future enhancements

This makes it an ideal solution for projects looking to attract and retain top talent with competitive, risk-adjusted compensation packages.