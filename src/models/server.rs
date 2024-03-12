use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::Sender, Mutex};
use sqlite::Connection;

use super::rule::ParsedRule;

pub struct WebServerState {
    pub channels: HashMap<String, Sender<ParsedRule>>,
    pub db_connection: Arc<Mutex<Connection>>,
}