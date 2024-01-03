use serde_derive::{Deserialize, Serialize};

// GRCOV_EXCL_START
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Extensions: usize {
        const UNWRAP_NEWTYPES = 0x1;
        const IMPLICIT_SOME = 0x2;
        const UNWRAP_VARIANT_NEWTYPES = 0x4;
        /// During deserialization, this extension requires that structs' names are stated explicitly.
        const EXPLICIT_STRUCT_NAMES = 0x8;
    }
}
// GRCOV_EXCL_STOP

impl Extensions {
    /// Creates an extension flag from an ident.
    #[must_use]
    pub fn from_ident(ident: &str) -> Option<Extensions> {
        match ident {
            "unwrap_newtypes" => Some(Extensions::UNWRAP_NEWTYPES),
            "implicit_some" => Some(Extensions::IMPLICIT_SOME),
            "unwrap_variant_newtypes" => Some(Extensions::UNWRAP_VARIANT_NEWTYPES),
            "explicit_struct_names" => Some(Extensions::EXPLICIT_STRUCT_NAMES),
            _ => None,
        }
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Extensions::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Extensions;

    fn roundtrip_extensions(ext: Extensions) {
        let ron = crate::to_string(&ext).unwrap();
        let ext2: Extensions = crate::from_str(&ron).unwrap();
        assert_eq!(ext, ext2);
    }

    // todo: maybe make a macro for this?
    #[test]
    fn test_extension_serde() {
        roundtrip_extensions(Extensions::default());
        roundtrip_extensions(Extensions::UNWRAP_NEWTYPES);
        roundtrip_extensions(Extensions::IMPLICIT_SOME);
        roundtrip_extensions(Extensions::UNWRAP_VARIANT_NEWTYPES);
        roundtrip_extensions(Extensions::EXPLICIT_STRUCT_NAMES);
        roundtrip_extensions(Extensions::UNWRAP_NEWTYPES | Extensions::IMPLICIT_SOME);
        roundtrip_extensions(Extensions::UNWRAP_NEWTYPES | Extensions::UNWRAP_VARIANT_NEWTYPES);
        roundtrip_extensions(Extensions::UNWRAP_NEWTYPES | Extensions::EXPLICIT_STRUCT_NAMES);
        roundtrip_extensions(Extensions::IMPLICIT_SOME | Extensions::UNWRAP_VARIANT_NEWTYPES);
        roundtrip_extensions(Extensions::IMPLICIT_SOME | Extensions::EXPLICIT_STRUCT_NAMES);
        roundtrip_extensions(Extensions::UNWRAP_VARIANT_NEWTYPES | Extensions::EXPLICIT_STRUCT_NAMES);
        roundtrip_extensions(
            Extensions::UNWRAP_NEWTYPES
                | Extensions::IMPLICIT_SOME
                | Extensions::UNWRAP_VARIANT_NEWTYPES
                | Extensions::EXPLICIT_STRUCT_NAMES,
        );
    }
}
