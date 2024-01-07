use serde_derive::{Deserialize, Serialize};

// GRCOV_EXCL_START
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Extensions: usize {
        const UNWRAP_NEWTYPES = 0x1;
        const IMPLICIT_SOME = 0x2;
        const UNWRAP_VARIANT_NEWTYPES = 0x4;
        /// During serialization, this extension emits struct names. See also [`PrettyConfig::struct_names`](crate::ser::PrettyConfig::struct_names) for the [`PrettyConfig`](crate::ser::PrettyConfig) equivalent.
        ///
        /// During deserialization, this extension requires that structs' names are stated explicitly.
        const EXPLICIT_STRUCT_NAMES = 0x8;
    }
}
// GRCOV_EXCL_STOP

impl Extensions {
    /// Creates an extension flag from an ident.
    #[must_use]
    pub fn from_ident(ident: &str) -> Option<Extensions> {
        for (name, extension) in Extensions::all().iter_names() {
            if ident == name.to_lowercase() {
                return Some(extension);
            }
        }

        None
    }
}

// GRCOV_EXCL_START
impl Default for Extensions {
    fn default() -> Self {
        Extensions::empty()
    }
}
// GRCOV_EXCL_STOP

#[cfg(test)]
mod tests {
    use super::Extensions;

    fn roundtrip_extensions(ext: Extensions) {
        let ron = crate::to_string(&ext).unwrap();
        let ext2: Extensions = crate::from_str(&ron).unwrap();
        assert_eq!(ext, ext2);
    }

    #[test]
    fn test_extension_serde() {
        // iterate over the powerset of all extensions (i.e. every possible combination of extensions)
        for bits in Extensions::empty().bits()..=Extensions::all().bits() {
            let extensions = Extensions::from_bits_retain(bits);
            roundtrip_extensions(extensions);
        }
    }
}
