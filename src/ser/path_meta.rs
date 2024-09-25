//! Path-based metadata to serialize with a value.
//!
//! Path-based in this context means that the metadata is linked
//! to the data in a relative and hierarchical fashion by tracking
//! the current absolute path of the field being serialized.
//!
//! # Example
//!
//! ```
//! # use ron::ser::{PrettyConfig, path_meta::Field};
//!
//! #[derive(serde::Serialize)]
//! struct Creature {
//!     seconds_since_existing: usize,
//!     linked: Option<Box<Self>>,
//! }
//!
//! let mut config = PrettyConfig::default();
//!
//! config
//!     .path_meta
//!     // The path meta defaults to no root structure,
//!     // so we either provide a prebuilt one or initialize
//!     // an empty one to build.
//!     .get_or_insert_with(Field::empty)
//!     .build_fields(|fields| {
//!         fields
//!             // Get or insert the named field
//!             .field("seconds_since_existing")
//!             .with_doc("Outer seconds_since_existing");
//!         fields
//!             .field("linked")
//!             // Doc metadata is serialized preceded by three forward slashes and a space for each line
//!             .with_doc("Optional.\nProvide another creature to be wrapped.")
//!             // Even though it's another Creature, the fields have different paths, so they are addressed separately.
//!             .build_fields(|fields| {
//!                 fields
//!                     .field("seconds_since_existing")
//!                     .with_doc("Inner seconds_since_existing");
//!             });
//!     });
//!
//! let value = Creature {
//!     seconds_since_existing: 0,
//!     linked: Some(Box::new(Creature {
//!         seconds_since_existing: 0,
//!         linked: None,
//!     })),
//! };
//!
//! let s = ron::ser::to_string_pretty(&value, config).unwrap();
//!
//! assert_eq!(s, r#"(
//!     /// Outer seconds_since_existing
//!     seconds_since_existing: 0,
//!     /// Optional.
//!     /// Provide another creature to be wrapped.
//!     linked: Some((
//!         /// Inner seconds_since_existing
//!         seconds_since_existing: 0,
//!         linked: None,
//!     )),
//! )"#);
//! ```
//!
//! # Identical paths
//!
//! Especially in enums and tuples it's possible for fields
//! to share a path, thus being unable to be addressed separately.
//!
//! ```no_run
//! enum Kind {
//!     A {
//!         field: (),
//!     },  // ^
//!         // cannot be addressed separately because they have the same path
//!     B { // v
//!         field: (),
//!     },
//! }
//! ```
//!
//! ```no_run
//! struct A {
//!     field: (),
//! }
//!
//! struct B {
//!     field: (),
//! }
//!
//! type Value = (
//!     A,
//!  // ^
//!  // These are different types, but they share the path `field`
//!  // v
//!     B,
//! );
//! ```

use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

/// The metadata and inner [`Fields`] of a field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Field {
    doc: String,
    fields: Option<Fields>,
}

impl Field {
    /// Create a new empty field metadata.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            doc: String::new(),
            fields: None,
        }
    }

    /// Create a new field metadata from parts.
    pub fn new(doc: impl Into<String>, fields: Option<Fields>) -> Self {
        Self {
            doc: doc.into(),
            fields,
        }
    }

    /// Get a shared reference to the documentation metadata of this field.
    #[inline]
    #[must_use]
    pub fn doc(&self) -> &str {
        self.doc.as_str()
    }

    /// Get a mutable reference to the documentation metadata of this field.
    #[inline]
    #[must_use]
    pub fn doc_mut(&mut self) -> &mut String {
        &mut self.doc
    }

    /// Set the documentation metadata of this field.
    ///
    /// ```
    /// # use ron::ser::path_meta::Field;
    ///
    /// let mut field = Field::empty();
    ///
    /// assert_eq!(field.doc(), "");
    ///
    /// field.with_doc("some meta");
    ///
    /// assert_eq!(field.doc(), "some meta");
    /// ```
    pub fn with_doc(&mut self, doc: impl Into<String>) -> &mut Self {
        self.doc = doc.into();
        self
    }

    /// Get a shared reference to the inner fields of this field, if it has any.
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
    /// # use ron::ser::path_meta::{Field, Fields};
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
    /// # use ron::ser::path_meta::{Field, Fields};
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
    /// # use ron::ser::path_meta::Field;
    ///
    /// let mut field = Field::empty();
    ///
    /// field.build_fields(|fields| {
    ///     fields.field("inner field");
    /// });
    ///
    /// assert_eq!(field.fields().map(|fields| fields.contains("inner field")), Some(true));
    /// ```
    pub fn build_fields(&mut self, builder: impl FnOnce(&mut Fields)) -> &mut Self {
        let mut fields = Fields::default();
        builder(&mut fields);
        self.with_fields(Some(fields));
        self
    }
}

/// Mapping of names to [`Field`]s.
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
    /// # use ron::ser::path_meta::{Fields, Field};
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
    /// # use ron::ser::path_meta::{Fields, Field};
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
    /// # use ron::ser::path_meta::{Fields, Field};
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
    /// # use ron::ser::path_meta::{Fields, Field};
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
    /// # use ron::ser::path_meta::{Fields, Field};
    ///
    /// let mut fields = Fields::default();
    ///
    /// assert!(fields.insert("field", Field::empty()).is_none());
    /// assert!(fields.insert("field", Field::empty()).is_some());
    /// ```
    pub fn insert(&mut self, name: impl Into<String>, field: Field) -> Option<Field> {
        self.fields.insert(name.into(), field)
    }

    /// Remove a field with the given name from the map.
    ///
    /// ```
    /// # use ron::ser::path_meta::{Fields, Field};
    ///
    /// let mut fields: Fields = [("a", Field::empty())].into_iter().collect();
    ///
    /// assert_eq!(fields.remove("a"), Some(Field::empty()));
    /// assert_eq!(fields.remove("a"), None);
    /// ```
    pub fn remove(&mut self, name: impl AsRef<str>) -> Option<Field> {
        self.fields.remove(name.as_ref())
    }

    /// Get a mutable reference to the field with the provided `name`,
    /// inserting an empty [`Field`] if it didn't exist.
    ///
    /// ```
    /// # use ron::ser::path_meta::Fields;
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
