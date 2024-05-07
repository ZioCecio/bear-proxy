use serde::{Deserialize, Serialize};

pub struct ParsedRule {
    pub id: usize,
    pub service_name: Option<String>,
    pub rule: Option<Vec<u8>>,
    pub action: RuleAction,
}

pub enum RuleAction {
    AddRule,
    RemoveRule,
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum RuleTypeDTO {
    Ascii,
    Hex,
    Base64,
}

#[derive(Deserialize, Debug)]
pub struct RuleDTO {
    pub service_name: String,
    pub rule_text: String,
    pub rule_type: RuleTypeDTO,
}

#[derive(Serialize, Deserialize)]
pub struct Rule {
    pub id: i64,
    pub b64_rule: String,
    pub service_name: String,
}
