use std::hash::{Hash, Hasher};

/// Static string hashed at compile time.
///
/// Minor optimization that allows for faster string comparisons without having
/// to hash at runtime.
#[derive(Debug, Clone, Copy, Eq)]
pub struct StaticStr {
    pub hash: u64,
    pub string: &'static str,
}

impl PartialEq for StaticStr {
    fn eq(&self, other: &Self) -> bool {
        self.string == other.string
    }
}

impl Hash for StaticStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

/// Macro to create a [`StaticStr`] from a compile-time expression.
///
/// This was preferred over a function because it enforces the use of a
/// compile-time expression.
#[macro_export]
macro_rules! static_str {
    ($s:expr) => {{
        let string: &'static str = $s;
        StaticStr {
            hash: const_fnv1a_hash::fnv1a_hash_str_64(string),
            string,
        }
    }};
}
