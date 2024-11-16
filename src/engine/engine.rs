use anyhow::Result;
use std::error::Error;

use super::account::TransactionDetails;
use super::transaction::Transaction;
use super::{account, Account, TransactionType};

use log::warn;

pub struct Engine {
    accounts: std::collections::HashMap<u16, account::Account>,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            accounts: std::collections::HashMap::new(),
        }
    }

    pub fn accounts(&self) -> Result<Vec<&Account>> {
        Ok(self.accounts.values().collect())
    }

    pub fn process_transactions<I, E>(&mut self, transacations_iter: I) -> Result<()>
    where
        I: Iterator<Item = std::result::Result<Transaction, E>>,
        E: Error + Sync + Send + 'static,
    {
        for transaction in transacations_iter {
            self.process_transaction(transaction?)?;
        }

        Ok(())
    }

    fn process_transaction(&mut self, transaction: Transaction) -> Result<()> {
        let transaction_id = transaction.transaction_id;

        // Check if the transaction is valid
        if !transaction.is_valid() {
            return Err(anyhow::anyhow!(
                "Invalid transaction: {}",
                transaction_id
            ));
        }

        // Check that the account exists.
        // If it doesn't, create a new account.
        let acc = self
            .accounts
            .entry(transaction.client_id)
            .or_insert(Account::new(transaction.client_id));

        // If the account is locked, ignore the transaction
        // Log attempt to make transaction on locked account
        if acc.locked {
            warn!(
                "Attempt to make transaction on locked account: {}",
                transaction_id
            );
            return Ok(());
        }

        // Process the transaction
        let result = match transaction.r#type {
            TransactionType::Deposit => self.deposit(transaction),
            TransactionType::Withdraw => self.withdraw(transaction),
            TransactionType::Dispute => self.dispute(transaction),
            TransactionType::Resolve => self.resolve(transaction),
            TransactionType::Chargeback => self.chargeback(transaction),
        };
        
        if result.is_err() {
            warn!(
                "Error processing transaction: {} {}",
                transaction_id, result.unwrap_err()
            );
        }

        Ok(())
    }

    // Deposit funds into account.
    //
    // * Increment available balance by the transaction amount
    // * Record the transaction (we need only the amount)
    fn deposit(&mut self, transaction: Transaction) -> Result<()> {
        let amount = transaction
            .amount
            .ok_or_else(|| anyhow::anyhow!("Deposit: presence of amount field should be asserted before calling deposit"))?;
        let account = self
            .accounts
            .get_mut(&transaction.client_id)
            .ok_or_else(|| anyhow::anyhow!("Deposit: existance of account should be asserted before calling deposit"))?;

        account.available += amount;
        account
            .transactions
            .insert(transaction.transaction_id, TransactionDetails::new(amount));

        Ok(())
    }

    // Withdraw funds from account.
    //
    // * Decrement available balance by the transaction amount
    // * Record the transaction (we need only the amount)
    fn withdraw(&mut self, transaction: Transaction) -> Result<()> {
        let account = self
            .accounts
            .get_mut(&transaction.client_id)
            .ok_or_else(|| anyhow::anyhow!("Withdraw: account with client_id does not exist
                Existance of account should be asserted before calling withdraw"))?;
        let amount = transaction.amount.ok_or_else(|| anyhow::anyhow!(
            "Withdraw: presence of amount field should be asserted before calling withdraw",
        ))?;

        if account.available < amount {
            warn!(
                "Attempt to withdraw more funds than available",
            );
            return Ok(());
        }

        account.available -= amount;
        account
            .transactions
            .insert(transaction.transaction_id, TransactionDetails::new(amount));

        Ok(())
    }

    // Dispute a transaction.
    //
    // * Move the transaction amount from available to held
    // * Mark the transaction as disputed
    fn dispute(&mut self, transaction: Transaction) -> Result<()> {
        let account = self
            .accounts
            .get_mut(&transaction.client_id)
            .ok_or_else(|| anyhow::anyhow!("Dispute: account with client_id {} does not exist. 
                Existance should be asserted before calling dispute", transaction.client_id))?;

        let disputed_transaction = account
            .transactions
            .get_mut(&transaction.transaction_id)
            .ok_or_else(|| anyhow::anyhow!("Dispute: transaction does not exist.
                Existance should be asserted before calling dispute"))?;


        if disputed_transaction.disputed {
            warn!(
                "Attempt to dispute already disputed transaction",
            );
            return Ok(());
        }

        if disputed_transaction.amount > account.available {
            warn!(
                "Attempt to dispute transaction with insufficient funds",
            );
            return Ok(());
        }

        account.available -= disputed_transaction.amount;
        account.held += disputed_transaction.amount;
        disputed_transaction.disputed = true;

        Ok(())
    }

    // Resolve a dispute.
    //
    // * Move the transaction amount from held to available
    // * Mark the transaction as not disputed
    fn resolve(&mut self, transaction: Transaction) -> Result<()> {
        let account = self
            .accounts
            .get_mut(&transaction.client_id)
            .ok_or_else(|| anyhow::anyhow!("Resolve: account with client_id {} does not exist.
                Existance should be asserted before calling resolve", transaction.client_id))?;

        let disputed_transaction = account
            .transactions
            .get_mut(&transaction.transaction_id)
            .ok_or_else(|| anyhow::anyhow!("Resolve: transaction does not exist.
                Existance should be asserted before calling resolve"))?;

        if !disputed_transaction.disputed {
            warn!("Attempt to resolve non-disputed transaction");
            return Ok(());
        }

        if disputed_transaction.amount > account.held {
            return Err(anyhow::anyhow!(
                "Resolve: Insufficient funds. This shuld never happen, held should never fall below amount.
                    Double check that we properly block transactions from blocked accounts.",
            ));
        }

        account.available += disputed_transaction.amount;
        account.held -= disputed_transaction.amount;
        disputed_transaction.disputed = false;

        Ok(())
    }

    fn chargeback(&mut self, transaction: Transaction) -> Result<()> {
        let account = self.accounts.get_mut(&transaction.client_id).ok_or_else(|| anyhow::anyhow!(
            "Chargeback: existance of account should be asserted before calling chargeback",
        ))?;

        let disputed_transaction = account
            .transactions
            .get_mut(&transaction.transaction_id)
            .ok_or_else(|| anyhow::anyhow!("Chargeback: disputed transaction not found"))?;

        if !disputed_transaction.disputed {
            warn!(
                "Attempt to chargeback non-disputed transaction",
            );
            return Ok(());
        }

        if disputed_transaction.amount > account.held {
            return Err(anyhow::anyhow!(
                "Chargeback: Insufficient funds. This should never happen, held should never fall below amount.
                    Double check that we allow to chargeback only disputed transactions.",
            ));
        }

        account.held -= disputed_transaction.amount;
        account.locked = true;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Engine;

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
    fn test_deposits() {
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
    fn test_deposits_withdrawals_zero_balance() {
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
    fn test_withdrawals_positive_balance() {
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
    fn test_withdrawals_negative_balance() {
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
    fn test_dispute() {
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
    fn test_dispute_with_nonexisting_id() {
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
    fn test_dispute_with_insufficient_funds() {
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
    fn test_resolution() {
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
    fn test_resolution_with_nonexisting_id() {
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
    fn test_resolution_without_dispute() {
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
    fn test_chargeback() {
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
    fn test_transaction_after_freeze() {
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
    fn test_chargeback_without_dispute() {
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
