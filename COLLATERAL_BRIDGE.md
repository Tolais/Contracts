# Vesting-to-Loan Collateral Bridge

## Overview

The Vesting-to-Loan Collateral Bridge is a sophisticated financial feature that allows beneficiaries to "borrow" against their future unvested tokens. This system enables team members and other token recipients to access liquidity without selling their tokens, preventing market pressure and providing financial flexibility during emergencies like buying a house.

## Architecture

The system consists of three main components:

1. **Enhanced Vesting Contract** - Extended with lien tracking capabilities
2. **Collateral Bridge Contract** - Manages liens and coordinates between vesting and lending
3. **Lending Contract** - Handles loan creation, repayment, and collateral claims

## Key Features

### Lien Mechanism
- **Lock Portion**: Beneficiaries can lock a portion of their unvested tokens as collateral
- **Multiple Liens**: Support for multiple concurrent liens on the same vault
- **Proportional Claims**: Lenders can claim tokens as they vest, proportional to their lien
- **Release on Repayment**: Liens are automatically released when loans are repaid

### Loan System
- **Flexible Terms**: Customizable loan amounts, interest rates, and maturity periods
- **Interest Calculation**: Basis points-based interest system (10000 = 100%)
- **Default Handling**: Automatic collateral claim on loan default after maturity
- **Early Repayment**: Support for early loan repayment with proportional lien release

### Security Features
- **Authorization Controls**: Vault owner authorization required for lien creation
- **Pause Mechanism**: Emergency pause functionality for all contracts
- **Validation Checks**: Comprehensive input validation and error handling
- **Audit Trail**: Event emissions for all major operations

## Workflow

### 1. Creating a Loan
```
Vault Owner → Authorizes Lien Creation
Lender → Creates Loan with Terms
Collateral Bridge → Creates Lien on Vault
Vesting Contract → Locks Tokens
Lending Contract → Transfers Loan Amount to Borrower
```

### 2. During Loan Period
```
Tokens Vest → Become Available for Claim
Lender → Can Claim Vested Tokens (up to locked amount)
Borrower → Can Repay Loan Early
```

### 3. Loan Maturity
```
Option A - Repaid:
  Borrower → Repays Full Amount + Interest
  Collateral Bridge → Releases Lien
  Tokens → Become Fully Available to Vault Owner

Option B - Defaulted:
  Lender → Claims Remaining Collateral
  Lien → Marked as Claimed
  Remaining Tokens → Return to Vault Owner
```

## Contract Interfaces

### Vesting Contract Extensions

#### New Vault Field
```rust
pub struct Vault {
    // ... existing fields ...
    pub locked_amount: i128,  // Amount locked for collateral liens
}
```

#### New Functions
- `lock_tokens(vault_id, amount)` - Lock tokens for collateral
- `unlock_tokens(vault_id, amount)` - Unlock released tokens
- `claim_by_lender(vault_id, lender, amount)` - Allow lender to claim vested tokens
- `set_collateral_bridge(address)` - Set authorized bridge contract
- `get_claimable_amount()` - Updated to exclude locked tokens

### Collateral Bridge Contract

#### Core Functions
- `create_lien(vault_id, lender, locked_amount, loan_amount, interest_rate, maturity_time)`
- `claim_collateral(lien_id)` - Claim vested tokens after maturity
- `release_lien(lien_id)` - Release lien on repayment
- `get_vault_liens(vault_id)` - Get all liens for a vault
- `get_lender_liens(lender)` - Get all liens for a lender

### Lending Contract

#### Core Functions
- `create_loan(borrower, lender, collateral_bridge, vault_id, loan_amount, collateral_amount, interest_rate, maturity_time)`
- `repay_loan(loan_id, repayment_amount)` - Repay loan partially or fully
- `claim_collateral(loan_id)` - Claim collateral on default
- `get_loan(loan_id)` - Get loan details

## Usage Examples

### Basic Loan Creation
```rust
// Create a loan against 1000 unvested tokens
let loan_id = lending_contract.create_loan(
    borrower,           // Address of the vault owner
    lender,             // Address of the lender
    collateral_bridge,  // Bridge contract address
    vault_id,           // ID of the vesting vault
    800i128,            // Loan amount (80% LTV)
    1000i128,           // Collateral amount
    1000u32,            // 10% interest rate (1000 basis points)
    maturity_time       // Loan maturity timestamp
);
```

### Lien Creation
```rust
// Create a lien to secure the loan
let lien_id = collateral_bridge.create_lien(
    vault_id,
    lender,
    1000i128,           // Amount to lock
    800i128,            // Loan amount
    1000u32,            // Interest rate
    maturity_time
);
```

### Collateral Claim
```rust
// After loan maturity and default
let claimed_amount = collateral_bridge.claim_collateral(lien_id);
```

## Risk Management

### For Lenders
- **LTV Limits**: Recommended loan-to-value ratios below 80%
- **Interest Rates**: Market-based interest rate calculations
- **Maturity Terms**: Reasonable loan periods based on vesting schedules
- **Diversification**: Spread lending across multiple vaults/borrowers

### For Borrowers
- **Partial Locking**: Only lock necessary token amounts
- **Early Repayment**: Avoid high interest costs through early repayment
- **Multiple Loans**: Consider impact of multiple concurrent liens
- **Market Conditions**: Monitor token price and vesting schedule

## Integration Guide

### Deployment Sequence
1. Deploy enhanced Vesting Contract
2. Deploy Collateral Bridge Contract
3. Deploy Lending Contract
4. Set Collateral Bridge address in Vesting Contract
5. Initialize all contracts with proper admin addresses

### Configuration
- Set appropriate interest rate limits
- Configure maximum LTV ratios
- Set admin and pause controls
- Test with small amounts before production use

## Security Considerations

### Authorization
- Vault owner must authorize lien creation
- Lender must authorize loan creation
- Admin controls for emergency operations
- Bridge contract authorization for vesting operations

### Validation
- Comprehensive input validation
- Overflow and underflow protection
- Timestamp validation for maturity periods
- Amount validation for positive values

### Emergency Controls
- Pause functionality for all contracts
- Admin transfer capabilities
- Emergency lien release mechanisms
- Circuit breaker patterns for extreme conditions

## Future Enhancements

### Planned Features
- **Interest-Only Loans**: Support for interest-only payment periods
- **Variable Interest Rates**: Dynamic interest rate adjustments
- **Secondary Market**: Lien trading and transfer capabilities
- **Insurance Integration**: Third-party insurance for loan protection

### Optimizations
- **Gas Efficiency**: Optimize storage patterns and computation
- **Batch Operations**: Support for batch lien operations
- **Oracle Integration**: Price oracle integration for dynamic LTV
- **Cross-Chain Support**: Multi-chain collateral bridge support

## Conclusion

The Vesting-to-Loan Collateral Bridge provides a powerful solution for token holders to access liquidity without selling their assets. This system benefits projects by reducing selling pressure, beneficiaries by providing financial flexibility, and lenders by creating new investment opportunities.

Proper implementation requires careful consideration of security, risk management, and user experience. The modular architecture allows for flexible deployment and future enhancements while maintaining the core functionality of providing liquidity without selling tokens.
