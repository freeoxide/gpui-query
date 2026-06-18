use crate::core::*;

pub fn make_resource() -> InfiniteQueryResource<Vec<String>> {
    InfiniteQueryResource::new(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::LatestWins,
    )
}

pub fn make_bidirectional_resource() -> InfiniteQueryResource<Vec<String>> {
    InfiniteQueryResource::new_bidirectional(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::LatestWins,
    )
}

/// Default page-content generator used by [`load_n_pages`].
///
/// Produces `vec![format!("page{i}")]` for page index `i`, matching the
/// historical behavior of `load_n_pages` before it was parameterized.
fn default_page(i: usize) -> Vec<String> {
    vec![format!("page{i}")]
}

/// Convenience: load N pages via `begin_fetch_next` + `complete_page_success`.
/// Each page contains a single element `format!("page{i}")` produced by
/// [`default_page`]. Returns the resource with pages loaded.
pub fn load_n_pages(n: usize) -> InfiniteQueryResource<Vec<String>> {
    load_n_pages_with(n, default_page)
}

/// Load N pages with a caller-supplied page-content factory.
///
/// `page_fn` is called once per page index with the page number (0-based) and
/// must return the page's data vector. This lets tests that need non-default
/// page content (e.g. typed data, multi-element pages) avoid the
/// `format!("page{i}")` allocation while keeping the common case as a
/// one-line `load_n_pages(n)` call.
pub fn load_n_pages_with(
    n: usize,
    page_fn: impl Fn(usize) -> Vec<String>,
) -> InfiniteQueryResource<Vec<String>> {
    let mut r = make_resource();
    let mut seq = RequestSequencer::new();
    for i in 0..n {
        let has_more = i < n - 1;
        let id = r.begin_fetch_next(&mut seq, (i * 100) as u128).unwrap();
        r.complete_page_success(
            id,
            page_fn(i),
            has_more,
            true,
            ((i + 1) * 100) as u128,
        );
    }
    r
}
