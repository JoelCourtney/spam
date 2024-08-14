use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct Story {
    pub title: String,
    text: String,
    entries: HashMap<String, String>,
    description: String,
}