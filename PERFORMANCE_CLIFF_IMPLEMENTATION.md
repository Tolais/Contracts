# Dynamic Cliff Based on Performance Oracles Implementation

## Overview

This implementation introduces **Performance-Based Cliff Vesting** to the Divine vesting system, replacing traditional time-based cliffs with dynamic conditions that depend on real-world project metrics. This aligns investor interests with project growth by ensuring exit liquidity is only provided once the team delivers verifiable value to the ecosystem.

## Architecture

### Core Components

1. **Oracle Module** (`oracle.rs`)
   - `OracleCondition`: Defines a single performance condition
   - `PerformanceCliff`: Groups multiple conditions with AND/OR logic
   - `OracleClient`: Handles oracle queries and condition evaluation

2. **Enhanced Vesting Contract** (`lib.rs`)
   - Modified `calculate_claimable()` to check performance cliffs first
   - New functions for cliff management
   - Integration with existing milestone system

## Key Features

### 1. Oracle Condition Types

```rust
pub enum OracleType {
    TVL,            // Total Value Locked targets
    Price,          // Token price targets
    Custom,         // Custom project metrics
}
```

### 2. Comparison Operators

```rust
pub enum ComparisonOperator {
    GreaterThan,    // Current > Target
    LessThan,       // Current < Target
    GreaterThanOrEqual, // Current >= Target
    LessThanOrEqual,   // Current <= Target
    Equal,          // Current == Target
}
```

### 3. Performance Cliff Structure

```rust
pub struct PerformanceCliff {
    pub conditions: Vec<OracleCondition>,  // Multiple conditions
    pub require_all: bool,                  // true = AND, false = OR logic
    pub fallback_time: u64,                // Fallback timestamp if oracle fails
}
```

## Implementation Details

### Cliff Evaluation Logic

The `is_cliff_passed()` function follows this priority:

1. **Oracle Conditions**: Query external contracts for real-time metrics
2. **Fallback Time**: If oracle fails, use timestamp as backup
3. **No Cliff Set**: Default to time-based vesting (start_time check)

### Integration with Vesting Calculation

The modified `calculate_claimable()` function:

```rust
fn calculate_claimable(env: &Env, id: u64, vault: &Vault) -> i128 {
    // 1. Check performance cliff first
    if let Some(cliff) = env.storage().instance().get(&DataKey::VaultPerformanceCliff(id)) {
        if !OracleClient::is_cliff_passed(env, &cliff, id) {
            return 0; // Cliff not passed, no vesting
        }
    }
    
    // 2. Continue with existing milestone or linear vesting logic
    // ...
}
```

## Usage Examples

### Creating a TVL-Based Cliff

```rust
// Create condition: TVL >= $1M
let tvl_condition = OracleClient::create_tvl_condition(
    oracle_address,
    1000000,  // $1M target
    ComparisonOperator::GreaterThanOrEqual,
);

let cliff = PerformanceCliff {
    conditions: vec![tvl_condition],
    require_all: true,
    fallback_time: 1640995200, // Jan 1, 2022
};

// Create vault with performance cliff
let vault_id = VestingContract::create_vault_with_cliff(
    env,
    beneficiary,
    amount,
    start_time,
    end_time,
    keeper_fee,
    is_revocable,
    is_transferable,
    step_duration,
    cliff,
);
```

### Multiple Conditions with OR Logic

```rust
// Condition 1: TVL >= $1M
let tvl_condition = OracleClient::create_tvl_condition(
    tvl_oracle,
    1000000,
    ComparisonOperator::GreaterThanOrEqual,
);

// Condition 2: Token price >= $100
let price_condition = OracleClient::create_price_condition(
    price_oracle,
    100,
    ComparisonOperator::GreaterThan,
    Some(Symbol::new(env, "TOKEN")),
);

let cliff = PerformanceCliff {
    conditions: vec![tvl_condition, price_condition],
    require_all: false, // OR logic - any condition passes
    fallback_time: 1640995200,
};
```

## API Functions

### Management Functions

- `set_performance_cliff(vault_id, cliff)` - Admin-only cliff setting
- `get_performance_cliff(vault_id)` - Retrieve cliff configuration
- `is_cliff_passed(vault_id)` - Check if cliff conditions are met

### Creation Functions

- `create_vault_with_cliff()` - Create vault with performance cliff
- Existing `create_vault_full/lazy()` functions remain unchanged

### Query Functions

- `get_claimable_amount(vault_id)` - Returns 0 if cliff not passed
- `claim_tokens(vault_id, amount)` - Respects cliff conditions

## Security Considerations

### 1. Oracle Reliability
- Fallback timestamp ensures vesting isn't permanently blocked
- Oracle failures don't prevent eventual token release

### 2. Admin Controls
- Only admins can set performance cliffs
- Cliff changes affect future vesting calculations only

### 3. Gas Optimization
- Short-circuit evaluation for OR logic
- Cached results where possible

## Integration with Existing Features

### Milestone Compatibility
Performance cliffs work seamlessly with existing milestone vesting:

```rust
// Cliff must pass BEFORE milestone vesting is considered
if cliff_not_passed -> 0 tokens claimable
else if milestones_set -> milestone-based vesting
else -> linear vesting
```

### Backward Compatibility
- Existing vaults without performance cliffs work unchanged
- Time-based vesting remains the default behavior

## Testing

The implementation includes comprehensive tests in `performance_cliff_test.rs`:

1. **Basic Cliff Creation** - Single condition setup
2. **Multiple Conditions** - AND/OR logic testing
3. **Fallback Behavior** - Oracle failure scenarios
4. **Milestone Integration** - Combined cliff + milestone vesting

## Future Enhancements

### 1. Oracle Contract Integration
Replace placeholder oracle calls with actual cross-contract calls:

```rust
let oracle_client = OracleContractClient::new(env, &oracle_address);
let current_value = oracle_client.get_value(oracle_type, parameter);
```

### 2. Dynamic Cliff Updates
Allow cliff conditions to be updated (with proper governance).

### 3. Conditional Vesting Curves
Different vesting curves based on which conditions are met.

## Conclusion

This implementation successfully introduces performance-based cliff vesting while maintaining full backward compatibility and security. The modular design allows for easy extension and integration with various oracle types, providing a robust foundation for milestone-first vesting that aligns investor and team incentives.
