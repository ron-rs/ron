use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Field {
    meta: String,
    inner: Option<Fields>,
}

impl Field {
    pub const fn new() -> Self {
        Self {
            meta: String::new(),
            inner: None,
        }
    }

    pub fn get_meta(&self) -> &str {
        self.meta.as_str()
    }

    pub fn set_meta(&mut self, meta: impl Into<String>) {
        self.meta = meta.into();
    }

    pub fn has_inner(&self) -> bool {
        self.inner.is_some()
    }

    pub fn set_inner(&mut self, fields: Fields) {
        self.inner = Some(fields);
    }

    pub fn inner(&self) -> Option<&Fields> {
        self.inner.as_ref()
    }

    pub fn inner_mut(&mut self) -> Option<&mut Fields> {
        self.inner.as_mut()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Fields {
    fields: HashMap<String, Field>,
}

impl Fields {
    pub fn new() -> Self {
        Self {
            fields: HashMap::default(),
        }
    }

    pub fn field(&self, name: impl AsRef<str>) -> Option<&Field> {
        self.fields.get(name.as_ref())
    }

    pub fn field_mut(&mut self, name: impl AsRef<str>) -> Option<&mut Field> {
        self.fields.get_mut(name.as_ref())
    }

    pub fn field_mut_or_default(&mut self, name: impl Into<String>) -> &mut Field {
        self.fields.entry(name.into()).or_default()
    }
}
