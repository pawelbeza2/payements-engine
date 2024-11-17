use rust_decimal::Decimal;
use serde::Deserialize;

pub struct TransactionDetails {
    pub amount: Decimal,
    pub disputed: bool,
}

impl TransactionDetails {
    pub fn new(amount: Decimal) -> TransactionDetails {
        TransactionDetails {
            amount,
            disputed: false,
        }
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum TransactionValidationError {
    #[error("Amount is missing")]
    AmountMissing,
    #[error("Amount is negative")]
    AmountNegative,
}

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
    pub fn get_amount_or_error(&self) -> Result<Decimal, TransactionValidationError> {
        match self.amount {
            Some(amount) => {
                if amount.is_sign_negative() {
                    return Err(TransactionValidationError::AmountNegative.into());
                }
                Ok(amount)
            }
            None => Err(TransactionValidationError::AmountMissing.into()),
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
