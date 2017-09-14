//! Value module.

use std::cmp::{Eq, Ordering};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

/// A wrapper for `f64` which guarantees that the inner value
/// is finite and thus implements `Eq`, `Hash` and `Ord`.
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub struct Number(f64);

impl Number {
    /// Panics if `v` is not a real number
    /// (infinity, NaN, ..).
    pub fn new(v: f64) -> Self {
        if !v.is_finite() {
            panic!("Tried to create Number with a NaN / infinity");
        }

        Number(v)
    }

    /// Returns the wrapped float.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0 as u64);
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("Bug: Contract violation")
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Char(char),
    Map(BTreeMap<Value, Value>),
    Number(Number),
    Option(Option<Box<Value>>),
    String(String),
    Seq(Vec<Value>),
    Unit,
}
