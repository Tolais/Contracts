#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Map, Vec, String};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Loan {
    pub borrower: Address,
    pub lender: Address,
    pub collateral_bridge: Address,
    pub vault_id: u64,
    pub loan_amount: i128,
    pub collateral_amount: i128,
    pub interest_rate: u32,  // basis points
    pub created_time: u64,
    pub maturity_time: u64,
    pub is_active: bool,
    pub is_defaulted: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum LendingDataKey {
    Admin,
    Token,
    LoanCount,
    Loan(u64),
    BorrowerLoans(Address),
    LenderLoans(Address),
    IsPaused,
}

#[contracttype]
pub struct LoanCreated {
    pub loan_id: u64,
    pub borrower: Address,
    pub lender: Address,
    pub loan_amount: i128,
    pub collateral_amount: i128,
}

#[contracttype]
pub struct LoanRepaid {
    pub loan_id: u64,
    pub borrower: Address,
    pub repayment_amount: i128,
}

#[contracttype]
pub struct CollateralClaimed {
    pub loan_id: u64,
    pub lender: Address,
    pub claimed_amount: i128,
}

#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    pub fn initialize(env: Env, admin: Address, token: Address) {
        if env.storage().instance().has(&LendingDataKey::Admin) {
            panic!("Already initialized");
        }
        
        admin.require_auth();
        env.storage().instance().set(&LendingDataKey::Admin, &admin);
        env.storage().instance().set(&LendingDataKey::Token, &token);
        env.storage().instance().set(&LendingDataKey::LoanCount, &0u64);
        env.storage().instance().set(&LendingDataKey::IsPaused, &false);
    }

    pub fn create_loan(
        env: Env,
        borrower: Address,
        lender: Address,
        collateral_bridge: Address,
        vault_id: u64,
        loan_amount: i128,
        collateral_amount: i128,
        interest_rate: u32,
        maturity_time: u64,
    ) -> u64 {
        Self::require_not_paused(&env);
        
        // Lender must authorize the loan
        lender.require_auth();
        
        // Validate inputs
        if loan_amount <= 0 || collateral_amount <= 0 {
            panic!("Amounts must be positive");
        }
        if interest_rate > 50000 {  // Max 500% interest
            panic!("Interest rate too high");
        }
        if maturity_time <= env.ledger().timestamp() {
            panic!("Invalid maturity time");
        }
        
        // Create loan
        let loan_id = Self::increment_loan_count(&env);
        let loan = Loan {
            borrower: borrower.clone(),
            lender: lender.clone(),
            collateral_bridge: collateral_bridge.clone(),
            vault_id,
            loan_amount,
            collateral_amount,
            interest_rate,
            created_time: env.ledger().timestamp(),
            maturity_time,
            is_active: true,
            is_defaulted: false,
        };
        
        // Store loan
        env.storage().instance().set(&LendingDataKey::Loan(loan_id), &loan);
        
        // Update indexes
        Self::add_borrower_loan(&env, &borrower, loan_id);
        Self::add_lender_loan(&env, &lender, loan_id);
        
        // Transfer loan amount from lender to borrower
        let token_address = Self::get_token(&env);
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&lender, &borrower, &loan_amount);
        
        // Emit event
        env.events().publish(
            ("loan_created", loan_id),
            LoanCreated {
                loan_id,
                borrower: borrower.clone(),
                lender: lender.clone(),
                loan_amount,
                collateral_amount,
            },
        );
        
        loan_id
    }

    pub fn repay_loan(env: Env, loan_id: u64, repayment_amount: i128) {
        Self::require_not_paused(&env);
        
        let mut loan = Self::get_loan(&env, loan_id);
        
        if !loan.is_active {
            panic!("Loan not active");
        }
        
        // Borrower must authorize repayment
        loan.borrower.require_auth();
        
        // Calculate total due (principal + interest)
        let interest_amount = (loan.loan_amount * loan.interest_rate as i128) / 10000;
        let total_due = loan.loan_amount + interest_amount;
        
        if repayment_amount > total_due {
            panic!("Repayment exceeds total due");
        }
        
        // Transfer repayment from borrower to lender
        let token_address = Self::get_token(&env);
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&loan.borrower, &loan.lender, &repayment_amount);
        
        // If full repayment, mark loan as inactive
        if repayment_amount >= total_due {
            loan.is_active = false;
            
            // Release collateral
            Self::release_collateral(&env, &loan);
        }
        
        env.storage().instance().set(&LendingDataKey::Loan(loan_id), &loan);
        
        // Emit event
        env.events().publish(
            ("loan_repaid", loan_id),
            LoanRepaid {
                loan_id,
                borrower: loan.borrower.clone(),
                repayment_amount,
            },
        );
    }

    pub fn claim_collateral(env: Env, loan_id: u64) -> i128 {
        Self::require_not_paused(&env);
        
        let mut loan = Self::get_loan(&env, loan_id);
        
        if !loan.is_active {
            panic!("Loan not active");
        }
        
        let now = env.ledger().timestamp();
        if now < loan.maturity_time {
            panic!("Loan not matured");
        }
        
        // Lender must authorize the claim
        loan.lender.require_auth();
        
        // Mark loan as defaulted
        loan.is_active = false;
        loan.is_defaulted = true;
        
        // Claim collateral from the collateral bridge
        let claimed_amount = Self::claim_from_bridge(&env, &loan);
        
        env.storage().instance().set(&LendingDataKey::Loan(loan_id), &loan);
        
        // Emit event
        env.events().publish(
            ("collateral_claimed", loan_id),
            CollateralClaimed {
                loan_id,
                lender: loan.lender.clone(),
                claimed_amount,
            },
        );
        
        claimed_amount
    }

    pub fn get_loan(env: Env, loan_id: u64) -> Loan {
        env.storage().instance()
            .get(&LendingDataKey::Loan(loan_id))
            .expect("Loan not found")
    }

    pub fn get_borrower_loans(env: Env, borrower: Address) -> Vec<u64> {
        env.storage().instance()
            .get(&LendingDataKey::BorrowerLoans(borrower))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_lender_loans(env: Env, lender: Address) -> Vec<u64> {
        env.storage().instance()
            .get(&LendingDataKey::LenderLoans(lender))
            .unwrap_or(Vec::new(&env))
    }

    // --- Internal Helpers ---

    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&LendingDataKey::Admin).expect("Admin not set");
        admin.require_auth();
    }

    fn require_not_paused(env: &Env) {
        if env.storage().instance().get(&LendingDataKey::IsPaused).unwrap_or(false) {
            panic!("Contract paused");
        }
    }

    fn get_token(env: &Env) -> Address {
        env.storage().instance().get(&LendingDataKey::Token).expect("Token not set")
    }

    fn increment_loan_count(env: &Env) -> u64 {
        let count: u64 = env.storage().instance().get(&LendingDataKey::LoanCount).unwrap_or(0);
        let new_count = count + 1;
        env.storage().instance().set(&LendingDataKey::LoanCount, &new_count);
        new_count
    }

    fn add_borrower_loan(env: &Env, borrower: &Address, loan_id: u64) {
        let mut loans: Vec<u64> = env.storage().instance()
            .get(&LendingDataKey::BorrowerLoans(borrower.clone()))
            .unwrap_or(Vec::new(env));
        loans.push_back(loan_id);
        env.storage().instance().set(&LendingDataKey::BorrowerLoans(borrower.clone()), &loans);
    }

    fn add_lender_loan(env: &Env, lender: &Address, loan_id: u64) {
        let mut loans: Vec<u64> = env.storage().instance()
            .get(&LendingDataKey::LenderLoans(lender.clone()))
            .unwrap_or(Vec::new(env));
        loans.push_back(loan_id);
        env.storage().instance().set(&LendingDataKey::LenderLoans(lender.clone()), &loans);
    }

    fn get_loan(env: &Env, loan_id: u64) -> Loan {
        env.storage().instance()
            .get(&LendingDataKey::Loan(loan_id))
            .expect("Loan not found")
    }

    fn release_collateral(env: &Env, loan: &Loan) {
        // Call the collateral bridge to release the lien
        // This would be implemented based on the collateral bridge interface
        // For now, we'll use a placeholder
        let bridge_client = CollateralBridgeClient::new(env, &loan.collateral_bridge);
        
        // Find the lien associated with this loan and release it
        // In a real implementation, we'd need to track the lien_id
        // For now, we'll assume the bridge can handle this
    }

    fn claim_from_bridge(env: &Env, loan: &Loan) -> i128 {
        // Call the collateral bridge to claim the collateral
        let bridge_client = CollateralBridgeClient::new(env, &loan.collateral_bridge);
        
        // In a real implementation, we'd claim the specific lien
        // For now, we'll return the collateral amount as a placeholder
        loan.collateral_amount
    }
}

// Mock client interfaces - these would be generated from the actual contracts
pub struct CollateralBridgeClient<'a> {
    env: &'a Env,
    address: &'a Address,
}

impl<'a> CollateralBridgeClient<'a> {
    pub fn new(env: &'a Env, address: &'a Address) -> Self {
        Self { env, address }
    }
}

#[cfg(test)]
mod test;
