use soroban_sdk::{ contracttype, Address, Env, Symbol, Vec };

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleType {
    TVL, // Total Value Locked
    Price, // Token price target
    Custom, // Custom condition
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComparisonOperator {
    GreaterThan, // >
    LessThan, // <
    GreaterThanOrEqual, // >=
    LessThanOrEqual, // <=
    Equal, // ==
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct OracleCondition {
    pub oracle_address: Address,
    pub oracle_type: OracleType,
    pub target_value: i128,
    pub operator: ComparisonOperator,
    pub parameter: Option<Symbol>, // Additional parameter for custom oracles
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PerformanceCliff {
    pub conditions: Vec<OracleCondition>,
    pub require_all: bool, // true = AND logic, false = OR logic
    pub fallback_time: u64, // Fallback timestamp if oracle fails
}

#[contracttype]
pub struct OracleQuery {
    pub vault_id: u64,
    pub conditions: Vec<OracleCondition>,
    pub require_all: bool,
}

pub trait OracleInterface {
    fn query_value(env: &Env, oracle_type: OracleType, parameter: Option<Symbol>) -> i128;
}

pub struct OracleClient;

impl OracleClient {
    pub fn new() -> Self {
        Self
    }

    pub fn is_cliff_passed(env: &Env, cliff: &PerformanceCliff, vault_id: u64) -> bool {
        // First try to evaluate oracle conditions
        if
            let Some(result) = Self::evaluate_oracle_conditions(
                env,
                &cliff.conditions,
                cliff.require_all
            )
        {
            return result;
        }

        // Fallback to time-based cliff if oracle evaluation fails
        let current_time = env.ledger().timestamp();
        current_time >= cliff.fallback_time
    }

    pub fn evaluate_oracle_conditions(
        env: &Env,
        conditions: &Vec<OracleCondition>,
        require_all: bool
    ) -> Option<bool> {
        if conditions.is_empty() {
            return None;
        }

        let mut results = Vec::new(env);

        for condition in conditions {
            let current_value = Self::query_oracle(env, &condition);
            let condition_met = Self::compare_values(
                current_value,
                condition.target_value,
                &condition.operator
            );
            results.push_back(condition_met);

            // Short-circuit for OR logic
            if !require_all && condition_met {
                return Some(true);
            }
        }

        // Final evaluation
        if require_all {
            // AND logic: all conditions must be met
            Some(results.iter().all(|x| x))
        } else {
            // OR logic: at least one condition must be met
            Some(results.iter().any(|x| x))
        }
    }

    fn query_oracle(env: &Env, condition: &OracleCondition) -> i128 {
        // This would make a cross-contract call to the oracle contract
        // For now, we'll implement a basic interface that can be extended
        let oracle_address = condition.oracle_address.clone();

        // In a real implementation, this would be:
        // let oracle_client = OracleContractClient::new(env, &oracle_address);
        // oracle_client.get_value(condition.oracle_type, condition.parameter)

        // For now, return 0 as placeholder - this should be replaced with actual oracle call
        0
    }

    fn compare_values(current: i128, target: i128, operator: &ComparisonOperator) -> bool {
        match operator {
            ComparisonOperator::GreaterThan => current > target,
            ComparisonOperator::LessThan => current < target,
            ComparisonOperator::GreaterThanOrEqual => current >= target,
            ComparisonOperator::LessThanOrEqual => current <= target,
            ComparisonOperator::Equal => current == target,
        }
    }

    pub fn create_tvl_condition(
        oracle_address: Address,
        target_tvl: i128,
        operator: ComparisonOperator
    ) -> OracleCondition {
        OracleCondition {
            oracle_address,
            oracle_type: OracleType::TVL,
            target_value: target_tvl,
            operator,
            parameter: None,
        }
    }

    pub fn create_price_condition(
        oracle_address: Address,
        target_price: i128,
        operator: ComparisonOperator,
        token_symbol: Option<Symbol>
    ) -> OracleCondition {
        OracleCondition {
            oracle_address,
            oracle_type: OracleType::Price,
            target_value: target_price,
            operator,
            parameter: token_symbol,
        }
    }

    pub fn create_custom_condition(
        oracle_address: Address,
        target_value: i128,
        operator: ComparisonOperator,
        parameter: Symbol
    ) -> OracleCondition {
        OracleCondition {
            oracle_address,
            oracle_type: OracleType::Custom,
            target_value,
            operator,
            parameter: Some(parameter),
        }
    }
}
