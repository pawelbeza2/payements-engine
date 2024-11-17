use rust_decimal::Decimal;
use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Clone)]
pub struct Account {
    pub client_id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn new(id: u16) -> Account {
        Account {
            client_id: id,
            available: Decimal::new(0, 4),
            held: Decimal::new(0, 4),
            locked: false,
        }
    }

    pub fn calculate_total(&self) -> Decimal {
        self.available + self.held
    }
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        fn format_decimal(value: Decimal) -> String {
            let mut value_str = value.normalize().to_string();
            if !value_str.contains('.') {
                value_str.push_str(".0");
            }
            return value_str;
        }

        let mut state: <S as Serializer>::SerializeStruct =
            serializer.serialize_struct("Account", 5)?;
        state.serialize_field("client", &self.client_id)?;
        state.serialize_field("available", &format_decimal(self.available))?;
        state.serialize_field("held", &format_decimal(self.held))?;
        state.serialize_field("total", &format_decimal(self.calculate_total()))?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}
