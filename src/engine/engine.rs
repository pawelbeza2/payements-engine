use anyhow::Result;
use dashmap::DashMap;
use std::{error::Error, sync::Arc};

use super::account::Account;
use super::account_manager::{AccountManager, AccountManagerError};
use super::transaction::{Transaction, TransactionType, TransactionValidationError};

use log::warn;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum EngineError {
    #[error("Transaction validation error: {0}")]
    TransactionValidationError(#[from] TransactionValidationError),
    #[error("AccountManager error: {0}")]
    AccountManagerError(#[from] AccountManagerError),
}

pub struct Engine {
    accounts: Arc<DashMap<u16, AccountManager>>,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            accounts: Arc::new(DashMap::new()),
        }
    }

    pub fn accounts(&self) -> Result<Vec<Account>> {
        Ok(self
            .accounts
            .iter()
            .map(|acc| acc.value().account.clone())
            .collect())
    }

    pub async fn process_transactions<I, E>(&mut self, transacations_iter: I) -> Result<()>
    where
        I: Iterator<Item = std::result::Result<Transaction, E>>,
        E: Error + Sync + Send + 'static,
    {
        for transaction in transacations_iter {
            if let Ok(transaction) = transaction {
                let transaction_id = transaction.transaction_id;
                let accounts = Arc::clone(&self.accounts);
                if let Err(e) = Self::process_transaction(accounts, transaction).await {
                    // Log error and continue processing
                    warn!("Error processing transaction {}: {}", transaction_id, e);
                }
            }
        }

        Ok(())
    }

    pub async fn process_transaction(
        accounts: Arc<DashMap<u16, AccountManager>>,
        transaction: Transaction,
    ) -> Result<(), EngineError> {
        // Get existing or create new account manager
        let mut account_manager = accounts
            .entry(transaction.client_id)
            .or_insert(AccountManager::new(transaction.client_id));

        // Process the transaction
        let transaction_id = transaction.transaction_id;
        return match transaction.r#type {
            TransactionType::Deposit => {
                let amount = transaction.get_amount_or_error()?;
                account_manager
                    .deposit(transaction_id, amount)
                    .map_err(EngineError::from)
            }
            TransactionType::Withdraw => {
                let amount = transaction.get_amount_or_error()?;
                account_manager.withdraw(amount).map_err(EngineError::from)
            }
            TransactionType::Dispute => account_manager
                .dispute(transaction_id)
                .map_err(EngineError::from),
            TransactionType::Resolve => account_manager
                .resolve(transaction_id)
                .map_err(EngineError::from),
            TransactionType::Chargeback => account_manager
                .chargeback(transaction_id)
                .map_err(EngineError::from),
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::Engine;
    use tokio::test;

    // Helper macro to read transaction records from a string, process them and compare them to the expected output.
    macro_rules! assert_account_balance {
        ($input:expr => $expected:expr) => {{
            // Initialize the engine and prepare input and expected output strings
            let mut engine = Engine::new();
            let input = $input
                .split_whitespace()
                .map(|s| format!("{}\n", s))
                .collect::<String>();
            let expected_output = $expected
                .split_whitespace()
                .map(|s| format!("{}\n", s))
                .collect::<String>();

            // Create a CSV reader from the input string
            let reader = csv::ReaderBuilder::new()
                .delimiter(b',')
                .flexible(true)
                .trim(csv::Trim::All)
                .from_reader(input.as_bytes());

            // Process transactions
            engine
                .process_transactions(reader.into_deserialize())
                .await
                .unwrap();

            // Get and sort accounts
            let mut accounts = engine.accounts().unwrap();
            accounts.sort_by_key(|a| a.client_id);

            // Serialize accounts to CSV
            let mut output_writer = csv::WriterBuilder::new()
                .delimiter(b',')
                .has_headers(true)
                .from_writer(vec![]);
            for account in accounts {
                output_writer.serialize(account).unwrap();
            }

            // Convert output to string and compare with expected output
            let output = String::from_utf8(output_writer.into_inner().unwrap()).unwrap();
            if output != expected_output {
                println!("Actual: {}", output);
            }
            assert_eq!(output, expected_output);
        }};
    }

    #[test]
    async fn test_deposits() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1.1
                deposit,1,2,2.2
                deposit,1,3,3.3
            "
            =>
            "
                client,available,held,total,locked
                1,6.6,0.0,6.6,false
            "
        )
    }

    #[test]
    async fn test_deposits_withdrawals_zero_balance() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1
                deposit,1,2,2
                withdrawal,1,3,3
            "
            =>
            "
                client,available,held,total,locked
                1,0.0,0.0,0.0,false
            "
        )
    }

    #[test]
    async fn test_withdrawals_positive_balance() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1.0
                deposit,1,2,2.0
                withdrawal,1,3,2.0
            "
            =>
            "
                client,available,held,total,locked
                1,1.0,0.0,1.0,false
            "
        )
    }

    #[test]
    async fn test_withdrawals_negative_balance() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1
                deposit,1,2,2
                withdrawal,1,3,4
            "
            =>
            "
                client,available,held,total,locked
                1,3.0,0.0,3.0,false
            "
        )
    }

    #[test]
    async fn test_dispute() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,15.0
                deposit,1,2,5.0
                dispute,1,1,
            "
            =>
            "
                client,available,held,total,locked
                1,5.0,15.0,20.0,false
            "
        )
    }

    #[test]
    async fn test_dispute_with_nonexisting_id() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1
                deposit,1,2,2
                dispute,1,3,
            "
            =>
            "
                client,available,held,total,locked
                1,3.0,0.0,3.0,false
            "
        )
    }

    #[test]
    async fn test_dispute_with_insufficient_funds() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1
                withdrawal,1,2,0.5
                dispute,1,3,1
            "
            =>
            "
                client,available,held,total,locked
                1,0.5,0.0,0.5,false
            "
        )
    }

    #[test]
    async fn test_resolution() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1.0
                deposit,1,2,2.0
                dispute,1,2,
                resolve,1,2,
            "
            =>
            "
                client,available,held,total,locked
                1,3.0,0.0,3.0,false
            "
        )
    }

    #[test]
    async fn test_resolution_with_nonexisting_id() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1.0
                deposit,1,2,2.0
                dispute,1,2,
                resolve,1,3,
            "
            =>
            "
                client,available,held,total,locked
                1,1.0,2.0,3.0,false
            "
        )
    }

    #[test]
    async fn test_resolution_without_dispute() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1.0
                deposit,1,2,2.0
                resolve,1,2,
            "
            =>
            "
                client,available,held,total,locked
                1,3.0,0.0,3.0,false
            "
        )
    }

    #[test]
    async fn test_chargeback() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1.0
                deposit,1,2,2.0
                dispute,1,2,
                chargeback,1,2,
            "
            =>
            "
                client,available,held,total,locked
                1,1.0,0.0,1.0,true
            "
        )
    }

    #[test]
    async fn test_transaction_after_freeze() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1.0
                deposit,1,2,2.0
                dispute,1,2,
                chargeback,1,2,
                deposit,1,1,1.0
            "
            =>
            "
                client,available,held,total,locked
                1,1.0,0.0,1.0,true
            "
        )
    }

    #[test]
    async fn test_chargeback_without_dispute() {
        assert_account_balance!(
            "
                type,client,tx,amount
                deposit,1,1,1.0
                deposit,1,2,2.0
                chargeback,1,2,
            "
            =>
            "
                client,available,held,total,locked
                1,3.0,0.0,3.0,false
            "
        )
    }
}
