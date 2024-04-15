use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(pub u64);

impl Hash {
    pub fn of<T: std::hash::Hash>(value: &T) -> Self {
        use std::hash::Hasher;

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);

        Hash(hasher.finish())
    }
}

/// A string that has been pre-hashed for faster comparisons.
///
/// Minor optimization that allows for faster string comparisons without having
/// to hash at runtime.
#[derive(Debug, Clone, Copy)]
pub struct HashedStr<'a> {
    pub hash: Hash,
    pub string: &'a str,
}

impl<'a> HashedStr<'a> {
    #[must_use]
    pub fn new(string: &'a str) -> Self {
        HashedStr {
            // todo: this should not use const_fnv1a_hash, a faster implementation should be used instead
            hash: Hash(const_fnv1a_hash::fnv1a_hash_str_64(string)),
            string,
        }
    }
}

impl std::hash::Hash for HashedStr<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl Eq for HashedStr<'_> {}

impl PartialEq for HashedStr<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl PartialOrd for HashedStr<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.string.cmp(other.string))
    }
}

impl<'a, S: Deref<Target = &'a str>> From<S> for HashedStr<'a> {
    fn from(val: S) -> Self {
        HashedStr {
            hash: Hash(const_fnv1a_hash::fnv1a_hash_str_64(&val)),
            string: &val,
        }
    }
}

impl Deref for HashedStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.string
    }
}

/// Macro to create a [`HashedStr`] from a compile-time expression.
///
/// This was preferred over a function because it enforces the use of a
/// compile-time expression.
#[macro_export]
macro_rules! hashed_str {
    ($s:expr) => {{
        let string: &'static str = $s;
        $crate::HashedStr {
            hash: $crate::Hash(const_fnv1a_hash::fnv1a_hash_str_64(string)),
            string,
        }
    }};
}
