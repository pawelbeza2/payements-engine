use std::collections::hash_map::Entry;
use std::collections::HashMap;

use rust_decimal::Decimal;

use super::account::Account;
use super::transaction::TransactionDetails;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum AccountManagerError {
    #[error("Account is locked")]
    AccountLocked,
    #[error("Transaction already exists")]
    TransactionExist,
    #[error("Transaction does not exist")]
    TransactionNotExist,
    #[error("Transaction already disputed")]
    TransactionDisputed,
    #[error("Transaction not disputed")]
    TransactionNotDisputed,
    #[error("Insufficient funds")]
    InsufficientFunds,
}

pub struct AccountManager {
    pub account: Account,
    pub transactions: HashMap<u32, TransactionDetails>,
}

impl AccountManager {
    pub fn new(id: u16) -> AccountManager {
        AccountManager {
            account: Account::new(id),
            transactions: HashMap::new(),
        }
    }

    fn assure_account_active(&self) -> Result<(), AccountManagerError> {
        self.account
            .locked
            .then(|| Err(AccountManagerError::AccountLocked))
            .unwrap_or(Ok(()))
    }

    // Deposit funds into account.
    //
    // * Increment available balance by the transaction amount
    // * Record the transaction
    pub fn deposit(
        &mut self,
        transaction_id: u32,
        amount: Decimal,
    ) -> Result<(), AccountManagerError> {
        self.assure_account_active()?;
        match self.transactions.entry(transaction_id) {
            Entry::Occupied(_) => return Err(AccountManagerError::TransactionExist),
            Entry::Vacant(entry) => {
                self.account.available += amount;
                entry.insert(TransactionDetails::new(amount));
                Ok(())
            }
        }
    }

    // Withdraw funds from account.
    //
    // * Decrement available balance by the transaction amount
    // * Record the transaction
    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), AccountManagerError> {
        if self.account.available < amount {
            return Err(AccountManagerError::InsufficientFunds);
        }

        self.account.available -= amount;
        Ok(())
    }

    // Dispute a transaction.
    //
    // * Mark the transaction as disputed
    // * Move the transaction amount from available to held
    pub fn dispute(&mut self, transaction_id: u32) -> Result<(), AccountManagerError> {
        self.assure_account_active()?;

        let disputed_transaction = self
            .transactions
            .get_mut(&transaction_id)
            .ok_or_else(|| AccountManagerError::TransactionNotExist)?;
        if disputed_transaction.disputed {
            return Err(AccountManagerError::TransactionDisputed);
        }

        disputed_transaction.disputed = true;
        self.account.available -= disputed_transaction.amount;
        self.account.held += disputed_transaction.amount;

        Ok(())
    }

    // Resolve a dispute.
    //
    // * Mark the transaction as not disputed
    // * Move the transaction amount from held to available
    pub fn resolve(&mut self, transaction_id: u32) -> Result<(), AccountManagerError> {
        self.assure_account_active()?;

        let disputed_transaction = self
            .transactions
            .get_mut(&transaction_id)
            .ok_or_else(|| AccountManagerError::TransactionNotExist)?;
        if !disputed_transaction.disputed {
            return Err(AccountManagerError::TransactionNotDisputed);
        }

        disputed_transaction.disputed = false;
        self.account.available += disputed_transaction.amount;
        self.account.held -= disputed_transaction.amount;

        Ok(())
    }

    // Chargeback a transaction.
    //
    // * Mark the transaction as not disputed
    // * Decrement held balance by the transaction amount
    // * Mark the account as locked
    pub fn chargeback(&mut self, transaction_id: u32) -> Result<(), AccountManagerError> {
        self.assure_account_active()?;

        let disputed_transaction = self
            .transactions
            .get_mut(&transaction_id)
            .ok_or_else(|| AccountManagerError::TransactionNotExist)?;
        if !disputed_transaction.disputed {
            return Err(AccountManagerError::TransactionNotDisputed);
        }

        disputed_transaction.disputed = false;
        self.account.held -= disputed_transaction.amount;
        self.account.locked = true;

        Ok(())
    }
}
