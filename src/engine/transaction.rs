use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub r#type: TransactionType,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub transaction_id: u32,
    pub amount: Option<Decimal>,
}

impl Transaction {
    // Validate the transaction
    //
    // A transaction is valid in the following cases:
    // * type in [deposit, withdrawal] and (amount is present and non neagtive)
    // * type in [dispute, resolve, chargeback] and amount is not present
    pub fn is_valid(&self) -> bool {
        match self.r#type {
            TransactionType::Deposit | TransactionType::Withdraw => {
                self.amount.is_some() && self.amount.unwrap() >= Decimal::ZERO
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                self.amount.is_none()
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdraw,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback,
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::{Transaction, TransactionType};

    #[test]
    fn test_is_valid() {
        let valid_transactions = [
            Transaction {
                r#type: TransactionType::Deposit,
                client_id: 1,
                transaction_id: 1,
                amount: Some(Decimal::new(1, 4)),
            },
            Transaction {
                r#type: TransactionType::Withdraw,
                client_id: 1,
                transaction_id: 1,
                amount: Some(Decimal::new(1, 4)),
            },
            Transaction {
                r#type: TransactionType::Dispute,
                client_id: 1,
                transaction_id: 1,
                amount: None,
            },
            Transaction {
                r#type: TransactionType::Resolve,
                client_id: 1,
                transaction_id: 1,
                amount: None,
            },
            Transaction {
                r#type: TransactionType::Chargeback,
                client_id: 1,
                transaction_id: 1,
                amount: None,
            },
        ];

        for transaction in valid_transactions.iter() {
            assert!(transaction.is_valid());
        }

        let invalid_transactions = [
            Transaction {
                r#type: TransactionType::Deposit,
                client_id: 1,
                transaction_id: 1,
                amount: None,
            },
            Transaction {
                r#type: TransactionType::Withdraw,
                client_id: 1,
                transaction_id: 1,
                amount: None,
            },
            Transaction {
                r#type: TransactionType::Dispute,
                client_id: 1,
                transaction_id: 1,
                amount: Some(Decimal::new(1, 4)),
            },
            Transaction {
                r#type: TransactionType::Resolve,
                client_id: 1,
                transaction_id: 1,
                amount: Some(Decimal::new(1, 4)),
            },
            Transaction {
                r#type: TransactionType::Chargeback,
                client_id: 1,
                transaction_id: 1,
                amount: Some(Decimal::new(1, 4)),
            },
        ];

        for transaction in invalid_transactions.iter() {
            assert!(!transaction.is_valid());
        }
    }
}
