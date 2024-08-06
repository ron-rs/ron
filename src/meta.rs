use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

/// The metadata and inner [Fields] of a field
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Field {
    meta: String,
    fields: Option<Fields>,
}

impl Field {
    /// Create a new empty field metadata
    pub const fn empty() -> Self {
        Self {
            meta: String::new(),
            fields: None,
        }
    }

    /// Create a new field metadata
    pub fn new(meta: impl Into<String>, fields: Option<Fields>) -> Self {
        Self {
            meta: meta.into(),
            fields,
        }
    }

    /// Set the metadata of this field
    pub fn with_meta(&mut self, meta: impl Into<String>) -> &mut Self {
        self.meta = meta.into();
        self
    }

    /// Set the inner fields of this field
    pub fn with_fields(&mut self, fields: Option<Fields>) -> &mut Self {
        self.fields = fields;
        self
    }

    /// Get the metadata of this field
    pub fn get_meta(&self) -> &str {
        self.meta.as_str()
    }

    /// Return whether this field has inner fields
    pub fn has_fields(&self) -> bool {
        self.fields.is_some()
    }

    /// Get a reference to the inner fields of this field, if it has any
    pub fn fields(&self) -> Option<&Fields> {
        self.fields.as_ref()
    }

    /// Get a mutable reference to the inner fields of this field, if it has any
    pub fn fields_mut(&mut self) -> Option<&mut Fields> {
        self.fields.as_mut()
    }
}

/// Mapping of names to [Field]s
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Fields {
    fields: HashMap<String, Field>,
}

impl Fields {
    /// Return a new, empty metadata field map
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a reference to the field with the provided `name`, if it exists
    pub fn get_field(&self, name: impl AsRef<str>) -> Option<&Field> {
        self.fields.get(name.as_ref())
    }

    /// Get a mutable reference to the field with the provided `name`, if it exists
    pub fn get_field_mut(&mut self, name: impl AsRef<str>) -> Option<&mut Field> {
        self.fields.get_mut(name.as_ref())
    }

    /// Get a mutable reference to the field with the provided `name`,
    /// inserting an empty [`Field`] if it didn't exist
    pub fn field(&mut self, name: impl Into<String>) -> &mut Field {
        self.fields.entry(name.into()).or_insert_with(Field::empty)
    }
}

impl<K: Into<String>> FromIterator<(K, Field)> for Fields {
    fn from_iter<T: IntoIterator<Item = (K, Field)>>(iter: T) -> Self {
        Self {
            fields: HashMap::from_iter(iter.into_iter().map(|(k, v)| (k.into(), v))),
        }
    }
}
