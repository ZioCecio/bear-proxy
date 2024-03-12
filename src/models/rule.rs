use serde::Deserialize;

pub struct ParsedRule {
    pub id: usize,
    pub service_name: String,
    pub rule: Vec<u8>,
    pub action: RuleAction,
}

pub enum RuleAction {
    AddRule,
    RemoveRule,
}

#[derive(Deserialize)]
pub struct RuleDTO {
    pub b64_rule: String,
}
