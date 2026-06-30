//! Tests for the `*_with_policy` fetcher variant ("server wins"): a fetcher
//! returning `Fetched<T>` can override the caller's per-query `CachePolicy` on
//! success.

use gpui::{AppContext as _, Entity, TestAppContext};

use crate::core::{CachePolicy, Fetched, QueryError, QueryResource, QueryStatus};
use crate::hook::*;
use crate::tests::test_support::*;

// ── use_query_with_policy ────────────────────────────────────────────────

#[gpui::test]
fn test_use_query_with_policy_overrides_cache_policy(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    // Caller asks for NoCache; the fetcher (the "server") overrides to a TTL.
    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_with_policy(
            QueryOptions::new("override").cache_policy(CachePolicy::NoCache),
            |_signal| async move {
                Ok::<_, QueryError>(Fetched::with_policy(
                    "data",
                    CachePolicy::Ttl { ttl_ms: 60_000 },
                ))
            },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        // Server wins: the resource's policy is the server's, not the caller's.
        assert_eq!(resource.cache_policy(), CachePolicy::Ttl { ttl_ms: 60_000 });
        assert_eq!(resource.data(), Some(&"data"));
    });
}

#[gpui::test]
fn test_use_query_with_policy_none_keeps_caller_policy(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    let caller_policy = CachePolicy::Ttl { ttl_ms: 5_000 };
    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_with_policy(
            QueryOptions::new("keep").cache_policy(caller_policy),
            |_signal| async move { Ok::<_, QueryError>(Fetched::new("data")) },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        // No server policy → the caller's policy is retained.
        assert_eq!(resource.cache_policy(), caller_policy);
        assert_eq!(resource.data(), Some(&"data"));
    });
}

#[gpui::test]
fn test_use_query_with_policy_stale_while_revalidate_override(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<u32, QueryError>>,
    }

    // The server can hand back a StaleWhileRevalidate policy (e.g. parsed from
    // `Cache-Control: max-age=30, stale-while-revalidate=60`).
    let server_policy = CachePolicy::StaleWhileRevalidate {
        ttl_ms: 30_000,
        stale_ms: 60_000,
    };
    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query_with_policy(
            QueryOptions::new("swr").cache_policy(CachePolicy::NoCache),
            move |_signal| {
                let server_policy = server_policy;
                async move { Ok::<_, QueryError>(Fetched::with_policy(42_u32, server_policy)) }
            },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        assert_eq!(resource.cache_policy(), server_policy);
        assert_eq!(resource.data(), Some(&42));
    });
}

// ── fetch_query_with_policy (refetch path) ───────────────────────────────

#[gpui::test]
fn test_fetch_query_with_policy_overrides_on_refetch(cx: &mut TestAppContext) {
    setup_query_client(cx);

    struct H {
        entity: Entity<QueryResource<&'static str, QueryError>>,
    }

    // Start with a plain fetch using NoCache. NoCache (no TTL) is required so an
    // immediate refetch is not short-circuited as a cache hit: `is_cache_fresh`
    // uses an inclusive `age_ms <= ttl_ms` boundary, so any `Ttl` policy would
    // treat a same-millisecond refetch as fresh and skip the fetch.
    let harness = cx.new(|cx| {
        let (entity, _sub) = use_query(
            QueryOptions::new("refetch-override").cache_policy(CachePolicy::NoCache),
            |_signal| async move { Ok::<_, QueryError>("first") },
            cx,
        );
        H { entity }
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        assert_eq!(resource.cache_policy(), CachePolicy::NoCache);
        assert_eq!(resource.data(), Some(&"first"));
    });

    // Refetch through the with_policy variant, overriding the policy to a TTL.
    harness.update(cx, |this, cx| {
        fetch_query_with_policy(
            &this.entity,
            || async move {
                Ok::<_, QueryError>(Fetched::with_policy(
                    "second",
                    CachePolicy::Ttl { ttl_ms: 60_000 },
                ))
            },
            cx,
        );
    });

    cx.run_until_parked();

    cx.update(|cx| {
        let resource = harness.read(cx).entity.read(cx);
        assert_eq!(resource.status(), QueryStatus::Success);
        // Server wins on refetch: the resource's policy is now the server's TTL.
        assert_eq!(resource.cache_policy(), CachePolicy::Ttl { ttl_ms: 60_000 });
        assert_eq!(resource.data(), Some(&"second"));
    });
}
