use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Meta {
    inner: HashMap<String, String>,
}

impl Meta {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.inner.insert(k.into(), v.into());
    }

    pub fn get(&self, k: &str) -> Option<&str> {
        self.inner.get(k).map(String::as_str)
    }
}
