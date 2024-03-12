use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ProxyConfig {
    pub services: Vec<ServiceInfo>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ServiceInfo {
    pub service_name: String,
    pub from: String,
    pub to: String,
}
