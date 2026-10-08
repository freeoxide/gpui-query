use super::QueryKey;

/// Key matcher for bulk operations like `invalidate_queries`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryKeyFilter<'a> {
    Exact(&'a QueryKey),
    Prefix(&'a QueryKey),
    All,
}

impl<'a> QueryKeyFilter<'a> {
    pub fn matches(&self, key: &QueryKey) -> bool {
        match self {
            Self::Exact(k) => key == *k,
            Self::Prefix(k) => key.starts_with(k),
            Self::All => true,
        }
    }
}
