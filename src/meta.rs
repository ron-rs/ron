use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

/// The metadata and inner [Fields] of a field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Field {
    meta: String,
    fields: Option<Fields>,
}

impl Field {
    /// Create a new empty field metadata.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            meta: String::new(),
            fields: None,
        }
    }

    /// Create a new field metadata.
    pub fn new(meta: impl Into<String>, fields: Option<Fields>) -> Self {
        Self {
            meta: meta.into(),
            fields,
        }
    }

    /// Get the metadata of this field.
    #[must_use]
    pub fn meta(&self) -> &str {
        &self.meta
    }

    /// Set the metadata of this field.
    ///
    /// ```
    /// # use ron::meta::Field;
    ///
    /// let mut field = Field::empty();
    ///
    /// assert_eq!(field.meta(), "");
    ///
    /// field.with_meta("some meta");
    ///
    /// assert_eq!(field.meta(), "some meta");
    /// ```
    pub fn with_meta(&mut self, meta: impl Into<String>) -> &mut Self {
        self.meta = meta.into();
        self
    }

    /// Return whether the Field has metadata.
    ///
    /// ```
    /// # use ron::meta::Field;
    ///
    /// let mut field = Field::empty();
    ///
    /// assert!(!field.has_meta());
    ///
    /// field.with_meta("some");
    ///
    /// assert!(field.has_meta());
    /// ```
    #[must_use]
    pub fn has_meta(&self) -> bool {
        !self.meta.is_empty()
    }

    /// Get a reference to the inner fields of this field, if it has any.
    #[must_use]
    pub fn fields(&self) -> Option<&Fields> {
        self.fields.as_ref()
    }

    /// Get a mutable reference to the inner fields of this field, if it has any.
    pub fn fields_mut(&mut self) -> Option<&mut Fields> {
        self.fields.as_mut()
    }

    /// Return whether this field has inner fields.
    ///
    /// ```
    /// # use ron::meta::{Field, Fields};
    ///
    /// let mut field = Field::empty();
    ///
    /// assert!(!field.has_fields());
    ///
    /// field.with_fields(Some(Fields::default()));
    ///
    /// assert!(field.has_fields());
    /// ```
    #[must_use]
    pub fn has_fields(&self) -> bool {
        self.fields.is_some()
    }

    /// Set the inner fields of this field.
    ///
    /// ```
    /// # use ron::meta::{Field, Fields};
    ///
    /// let mut field = Field::empty();
    ///
    /// assert!(!field.has_fields());
    ///
    /// field.with_fields(Some(Fields::default()));
    ///
    /// assert!(field.has_fields());
    ///
    /// field.with_fields(None);
    ///  
    /// assert!(!field.has_fields());
    /// ```
    pub fn with_fields(&mut self, fields: Option<Fields>) -> &mut Self {
        self.fields = fields;
        self
    }

    /// Ergonomic shortcut for building some inner fields.
    ///
    /// ```
    /// # use ron::meta::Field;
    ///
    /// let mut field = Field::empty();
    ///
    /// field.build_fields(|fields| {
    ///     fields.field("inner field");
    /// });
    ///
    /// assert!(field.fields().is_some_and(|fields| fields.contains("inner field")));
    /// ```
    pub fn build_fields(&mut self, builder: impl FnOnce(&mut Fields)) -> &mut Self {
        let mut fields = Fields::default();
        builder(&mut fields);
        self.with_fields(Some(fields));
        self
    }
}

/// Mapping of names to [Field]s.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Fields {
    fields: HashMap<String, Field>,
}

impl Fields {
    /// Return a new, empty metadata field map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return whether this field map contains no fields.
    ///
    /// ```
    /// # use ron::meta::{Fields, Field};
    ///
    /// let mut fields = Fields::default();
    ///
    /// assert!(fields.is_empty());
    ///
    /// fields.insert("", Field::empty());
    ///
    /// assert!(!fields.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Return whether this field map contains a field with the given name.
    ///
    /// ```
    /// # use ron::meta::{Fields, Field};
    ///
    /// let fields: Fields = [("a thing", Field::empty())].into_iter().collect();
    ///
    /// assert!(fields.contains("a thing"));
    /// assert!(!fields.contains("not a thing"));
    /// ```
    pub fn contains(&self, name: impl AsRef<str>) -> bool {
        self.fields.contains_key(name.as_ref())
    }

    /// Get a reference to the field with the provided `name`, if it exists.
    ///
    /// ```
    /// # use ron::meta::{Fields, Field};
    ///
    /// let fields: Fields = [("a thing", Field::empty())].into_iter().collect();
    ///
    /// assert!(fields.get("a thing").is_some());
    /// assert!(fields.get("not a thing").is_none());
    /// ```
    pub fn get(&self, name: impl AsRef<str>) -> Option<&Field> {
        self.fields.get(name.as_ref())
    }

    /// Get a mutable reference to the field with the provided `name`, if it exists.
    ///
    /// ```
    /// # use ron::meta::{Fields, Field};
    ///
    /// let mut fields: Fields = [("a thing", Field::empty())].into_iter().collect();
    ///
    /// assert!(fields.get_mut("a thing").is_some());
    /// assert!(fields.get_mut("not a thing").is_none());
    /// ```
    pub fn get_mut(&mut self, name: impl AsRef<str>) -> Option<&mut Field> {
        self.fields.get_mut(name.as_ref())
    }

    /// Insert a field with the given name into the map.
    ///
    /// ```
    /// # use ron::meta::{Fields, Field};
    ///
    /// let mut fields = Fields::default();
    ///
    /// assert!(fields.insert("field", Field::empty()).is_none());
    /// assert!(fields.insert("field", Field::empty()).is_some());
    /// ```
    pub fn insert(&mut self, name: impl Into<String>, field: Field) -> Option<Field> {
        self.fields.insert(name.into(), field)
    }

    /// Get a mutable reference to the field with the provided `name`,
    /// inserting an empty [`Field`] if it didn't exist.
    ///
    /// ```
    /// # use ron::meta::Fields;
    ///
    /// let mut fields = Fields::default();
    ///
    /// assert!(!fields.contains("thing"));
    ///
    /// fields.field("thing");
    ///
    /// assert!(fields.contains("thing"));
    /// ```
    pub fn field(&mut self, name: impl Into<String>) -> &mut Field {
        self.fields.entry(name.into()).or_insert_with(Field::empty)
    }
}

impl<K: Into<String>> FromIterator<(K, Field)> for Fields {
    fn from_iter<T: IntoIterator<Item = (K, Field)>>(iter: T) -> Self {
        Self {
            fields: iter.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
}
