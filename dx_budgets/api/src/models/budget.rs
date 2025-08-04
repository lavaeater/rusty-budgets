use joydb::Model;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::str::FromStr;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MonthBeginsOn {
    PreviousMonth(u32),
    CurrentMonth(u32),
}

impl Default for MonthBeginsOn {
    fn default() -> Self {
        MonthBeginsOn::PreviousMonth(25)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub month_begins_on: MonthBeginsOn,
    #[serde(serialize_with = "serialize_budget_items_as_string_keys",
        deserialize_with = "deserialize_budget_items_with_string_keys")]
    pub budget_items: HashMap<BudgetCategory, Vec<BudgetItem>>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub user_id: Uuid,
}

impl PartialEq for Budget {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Budget {
    pub fn get_available_spendable_budget(&self) -> f32 {
        self.budget_items
            .iter()
            .filter_map(|(key, items)| {
                if matches!(key, BudgetCategory::Income(_)) {
                    Some(
                        items.iter().map(|i| i.remaining_spendable_amount()).sum::<f32>(),
                    )
                } else {
                    None
                }
            })
            .sum()
    }

    pub fn spend_money_on(&mut self, target: &mut BudgetItem, amount: f32) {
        if amount > 0.0 && amount <= self.get_available_spendable_budget() {
            /* 
            Some splitting logic needed here, we need to split the amount
            over multiple budget items if not one can fit the entire amount
             */
            let mut amount_left = amount;
            for (category, items) in &mut self.budget_items {
                if matches!(category, BudgetCategory::Income(_)) {
                    for item in items.iter_mut().filter(|i| i.remaining_spendable_amount() > 0.0) {
                        if item.remaining_spendable_amount() > amount_left {
                            let transaction = BudgetTransaction::new(
                                "Spend Money",
                                BudgetTransactionType::default(),
                                amount_left,
                                Some(item.id),
                                target.id,
                                self.user_id,
                            );
                            item.store_outgoing_transaction(&transaction);
                            target.store_incoming_transaction(&transaction);
                            amount_left = 0.0;
                            break;
                        } else {
                            let amount_to_spend = item.remaining_spendable_amount();
                            let transaction = BudgetTransaction::new(
                                "Spend Money",
                                BudgetTransactionType::default(),
                                amount_to_spend,
                                Some(item.id),
                                target.id,
                                self.user_id,
                            );
                            item.store_outgoing_transaction(&transaction);
                            target.store_incoming_transaction(&transaction);
                            amount_left -= amount_to_spend;
                        }                        
                    }

                }
            }
        }
    }
}


impl Budget {
    pub fn new(name: &str, default_budget: bool, user_id: Uuid) -> Budget {
        Budget {
            id: Uuid::new_v4(),
            name: name.to_string(),
            default_budget,
            month_begins_on: MonthBeginsOn::PreviousMonth(25),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            user_id,
            ..Default::default()
        }
    }

    pub fn new_income_category(&mut self, category_name: &str) -> BudgetCategory {
        let category = BudgetCategory::Income(category_name.to_string());
        self.store_category(&category);
        category
    }
    
    pub fn store_category(&mut self, category: &BudgetCategory) {
        if let Vacant(e) = self.budget_items.entry(category.clone()) {
            e.insert(Vec::new());
        }
        self.touch();
    }

    pub fn new_expense_category(&mut self, category_name: &str) -> BudgetCategory {
        let category = BudgetCategory::Expense(category_name.to_string());
        self.store_category(&category);
        category

    }

    pub fn store_budget_item(&mut self, budget_item: &BudgetItem) {
        match self.budget_items.entry(budget_item.budget_category.clone()) {
            Vacant(e) => {
                e.insert(vec![budget_item.clone()]);
            }
            Occupied(mut e) => {
                e.get_mut().push(budget_item.clone());
            }
        }

        self.touch();
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BankTransaction {
    pub id: Uuid,
    pub text: String,
    pub amount: f32,
    pub budget_item: Uuid,
    pub bank_date: chrono::NaiveDate,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
}

impl Eq for BankTransaction {}

impl BankTransaction {
    pub fn new_from_user(
        text: &str,
        amount: f32,
        budget_item: Uuid,
        bank_date: chrono::NaiveDate,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            text: text.to_string(),
            amount,
            budget_item,
            bank_date,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum BudgetCategory {
    Income(String),
    Expense(String),
    Savings(String),
}

impl Default for BudgetCategory {
    fn default() -> Self {
        BudgetCategory::Expense("Övrigt".to_string())
    }
}

impl Display for BudgetCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetCategory::Income(s) => write!(f, "Income({})", s),
            BudgetCategory::Expense(s) => write!(f, "Expense({})", s),
            BudgetCategory::Savings(s) => write!(f, "Savings({})", s),
        }
    }
}


impl FromStr for BudgetCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Basic format: "VariantName(value)"
        if let Some(rest) = s.strip_prefix("Income(").and_then(|s| s.strip_suffix(")")) {
            return Ok(BudgetCategory::Income(rest.to_string()));
        } else if let Some(rest) = s.strip_prefix("Expense(").and_then(|s| s.strip_suffix(")")) {
            return Ok(BudgetCategory::Expense(rest.to_string()));
        } else if let Some(rest) = s.strip_prefix("Savings(").and_then(|s| s.strip_suffix(")")) {
            return Ok(BudgetCategory::Savings(rest.to_string()));
        }
        Err(format!("Unknown BudgetCategory format: {}", s))
    }
}

fn deserialize_budget_items_with_string_keys<'de, D>(
    deserializer: D,
) -> Result<HashMap<BudgetCategory, Vec<BudgetItem>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct BudgetMapVisitor {
        marker: PhantomData<fn() -> HashMap<BudgetCategory, Vec<BudgetItem>>>,
    }

    impl<'de> Visitor<'de> for BudgetMapVisitor {
        type Value = HashMap<BudgetCategory, Vec<BudgetItem>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map with stringified BudgetCategory keys")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = HashMap::new();
            while let Some((key_str, value)) = access.next_entry::<String, Vec<BudgetItem>>()? {
                let key = BudgetCategory::from_str(&key_str)
                    .map_err(|e| de::Error::custom(format!("Key parse error: {}", e)))?;
                map.insert(key, value);
            }
            Ok(map)
        }
    }

    deserializer.deserialize_map(BudgetMapVisitor {
        marker: PhantomData,
    })
}

fn serialize_budget_items_as_string_keys<S>(
    map: &HashMap<BudgetCategory, Vec<BudgetItem>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map_ser = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        map_ser.serialize_entry(&k.to_string(), v)?;
    }
    map_ser.end()
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetItemPeriodicity {
    Once,
    #[default]
    Monthly,
    Quarterly,
    Yearly,
}

impl Display for BudgetItemPeriodicity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetItemPeriodicity::Once => write!(f, "Once"),
            BudgetItemPeriodicity::Monthly => write!(f, "Monthly"),
            BudgetItemPeriodicity::Quarterly => write!(f, "Quarterly"),
            BudgetItemPeriodicity::Yearly => write!(f, "Yearly"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Model)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budget_category: BudgetCategory,
    pub incoming_transactions: HashMap<Uuid, BudgetTransaction>,
    pub outgoing_transactions: HashMap<Uuid, BudgetTransaction>,
    pub bank_transactions: HashMap<Uuid, BankTransaction>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
    pub budget_id: Uuid,
}

impl Hash for BudgetItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl BudgetItem {
    pub fn new(
        budget_id: Uuid,
        name: &str,
        budget_category: &BudgetCategory,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            budget_id,
            name: name.to_string(),
            budget_category: budget_category.clone(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
            ..Default::default()
        }
    }
    
    pub fn remaining_spendable_amount(&self) -> f32 {
        self.incoming_transactions.values().map(|v| v.amount)
            .sum::<f32>()
            - self
            .outgoing_transactions.values().map(|v| v.amount)
            .sum::<f32>()
    }

    pub fn store_incoming_transaction(&mut self, budget_transaction: &BudgetTransaction) {
        match self.incoming_transactions.entry(budget_transaction.id) {
            Vacant(e) => {
                e.insert(budget_transaction.clone());
            }
            Occupied(mut e) => {
                e.insert(budget_transaction.clone());
            }
        }
        self.touch();
    }

    pub fn store_outgoing_transaction(&mut self, budget_transaction: &BudgetTransaction) {
        match self.outgoing_transactions.entry(budget_transaction.id) {
            Vacant(e) => {
                e.insert(budget_transaction.clone());
            }
            Occupied(mut e) => {
                e.insert(budget_transaction.clone());
            }
        }
        self.touch();
    }

    pub fn store_bank_transaction(&mut self, budget_transaction: &BankTransaction) {
        match self.bank_transactions.entry(budget_transaction.id) {
            Vacant(e) => {
                e.insert(budget_transaction.clone());
            }
            Occupied(mut e) => {
                e.insert(budget_transaction.clone());
            }
        }
        self.touch();
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum BudgetTransactionType {
    #[default]
    StartValue,
    Adjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Model)]
pub struct BudgetTransaction {
    pub id: Uuid,
    pub text: String,
    pub transaction_type: BudgetTransactionType,
    pub amount: f32,
    pub from_budget_item: Option<Uuid>,
    pub to_budget_item: Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
}

impl PartialEq for BudgetTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for BudgetTransaction {}

impl Hash for BudgetTransaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl BudgetTransaction {
    pub fn new(
        text: &str,
        transaction_type: BudgetTransactionType,
        amount: f32,
        from_budget_item: Option<Uuid>,
        to_budget_item: Uuid,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            text: text.to_string(),
            transaction_type,
            amount,
            to_budget_item,
            from_budget_item,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
        }
    }
}
