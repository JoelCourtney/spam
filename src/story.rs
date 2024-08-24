use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct Story {
    pub title: String,
    text: String,
    entries: HashMap<String, String>,
    description: String,
    instruction: String,
    model: String
}

impl Story {
    pub(crate) fn new(title: String) -> Self {
        Self {
            title,
            ..Self::default()
        }
    }
}
