use crate::core::*;

pub fn make_resource() -> InfiniteQueryResource<Vec<&'static str>> {
    InfiniteQueryResource::new(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::LatestWins,
    )
}

pub fn make_bidirectional_resource() -> InfiniteQueryResource<Vec<&'static str>> {
    InfiniteQueryResource::new_bidirectional(
        QueryKey::from("items"),
        CachePolicy::Ttl { ttl_ms: 60_000 },
        RequestPolicy::LatestWins,
    )
}

/// Static page labels (`"page0"`, `"page1"`, ...) used by the default page
/// generator. Sized to comfortably cover the largest load count exercised by
/// the test suite (50 pages in `max_pages_50_allows_50_pages_and_evicts_on_51st`).
const PAGE_LABELS: [&str; 64] = [
    "page0", "page1", "page2", "page3", "page4", "page5", "page6", "page7",
    "page8", "page9", "page10", "page11", "page12", "page13", "page14", "page15",
    "page16", "page17", "page18", "page19", "page20", "page21", "page22", "page23",
    "page24", "page25", "page26", "page27", "page28", "page29", "page30", "page31",
    "page32", "page33", "page34", "page35", "page36", "page37", "page38", "page39",
    "page40", "page41", "page42", "page43", "page44", "page45", "page46", "page47",
    "page48", "page49", "page50", "page51", "page52", "page53", "page54", "page55",
    "page56", "page57", "page58", "page59", "page60", "page61", "page62", "page63",
];

/// Default page-content generator used by [`load_n_pages`].
///
/// Produces `vec!["page{i}"]` for page index `i`, matching the historical
/// behavior of `load_n_pages` before it was parameterized. Uses static labels
/// (no allocation).
fn default_page(i: usize) -> Vec<&'static str> {
    vec![PAGE_LABELS[i]]
}

/// Convenience: load N pages via `begin_fetch_next` + `complete_page_success`.
/// Each page contains a single element `"page{i}"` produced by
/// [`default_page`]. Returns the resource with pages loaded.
pub fn load_n_pages(n: usize) -> InfiniteQueryResource<Vec<&'static str>> {
    load_n_pages_with(n, default_page)
}

/// Load N pages with a caller-supplied page-content factory.
///
/// `page_fn` is called once per page index with the page number (0-based) and
/// must return the page's data vector. This lets tests that need non-default
/// page content (e.g. typed data, multi-element pages) avoid per-page
/// allocation while keeping the common case as a one-line `load_n_pages(n)`
/// call.
pub fn load_n_pages_with(
    n: usize,
    page_fn: impl Fn(usize) -> Vec<&'static str>,
) -> InfiniteQueryResource<Vec<&'static str>> {
    let mut r = make_resource();
    let mut seq = RequestSequencer::new();
    for i in 0..n {
        let has_more = i < n - 1;
        let id = r.begin_fetch_next(&mut seq, (i * 100) as u64).unwrap();
        r.complete_page_success(
            id,
            page_fn(i),
            has_more,
            true,
            ((i + 1) * 100) as u64,
        );
    }
    r
}
