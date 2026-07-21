# gpui-query-persist

Reference disk-backed [`Persister`](https://docs.rs/gpui-query/latest/gpui_query/client/trait.Persister.html) for [gpui-query](https://crates.io/crates/gpui-query), modeled after [TanStack Query Persisters](https://tanstack.com/query/latest/docs/framework/react/plugins/persistQueryClient).

gpui-query's `persist` feature gives you the `Persister` trait and the `persist_with` driver, but leaves storage to you. This crate ships `FilePersister`, an atomic, durable, file-system adapter you can drop in: JSON or bincode, tolerant of missing or corrupt files, and safe to run on GPUI's background executor with no `tokio` required.

## Install

```sh
cargo add gpui-query-persist
```

```toml
[dependencies]
gpui-query-persist = "0.1.0"
```

The crate pulls in [gpui-query](https://crates.io/crates/gpui-query) with the `persist`, `client`, and `hook` features already enabled, plus `dirs`, `tempfile`, `bincode`, `serde`, `serde_json`, and `thiserror`.

## What it does

- **Atomic write.** Each save serializes the snapshot to a sibling `NamedTempFile` (via [`tempfile`]), fsyncs it (`F_FULLFSYNC` on macOS, plain `fsync` elsewhere), then renames it over the target. On POSIX the parent directory is fsynced after the replace so the rename survives power loss.
- **Tolerant load.** A missing file yields an empty snapshot. A corrupt or unparseable file is logged and treated as empty (no panic). A version mismatch returns `PersistError::VersionMismatch`, so callers can distinguish "corrupt" from "wrong format".
- **No `tokio`.** Writes are serialized through a `std::sync::Mutex` and run on GPUI's `background_executor`; reads take the same lock briefly. The persister performs synchronous `std::fs` I/O, which is what the background executor is designed for.

## Quick start

Hand a `FilePersister` to `QueryClient::persist_with` when your app starts. Keep the returned `PersistHandle` alive for as long as you want saves to continue.

```rust
use gpui::App;
use gpui_query::client::{PersistOptions, QueryClient};
use gpui_query_persist::FilePersister;

App::new().run(|cx| {
    cx.set_global(QueryClient::new());

    // JSON at an explicit path:
    let persister = FilePersister::json("cache/gpui-query.json");
    // ...or rooted at the OS cache dir:
    // let persister = FilePersister::in_cache_dir("my-app").unwrap();

    let _handle = cx.update_global::<QueryClient, _>(|client, cx| {
        client.persist_with(persister, PersistOptions::default(), cx)
    });
    // ... your views
});
```

### Constructors

```rust
// Pick a format explicitly:
FilePersister::new("path/to/cache.bin", PersistFormat::Bincode);

// JSON / bincode shortcuts:
FilePersister::json("path/to/cache.json");
FilePersister::bincode("path/to/cache.bin");

// Rooted at the OS cache dir: <cache_dir>/<app_name>/gpui-query-cache.json
FilePersister::in_cache_dir("my-app")?; // -> Result<Self, PersistError>

// Inspect the on-disk path:
FilePersister::json("path/to/cache.json").path(); // -> &Path
```

`PersistFormat` is `Json` (human-readable, the default for `in_cache_dir`) or `Bincode` (compact, not human-readable). `NoopPersister` is re-exported for tests or disabled modes.

## Links

- Website: <https://gpui-query.freeoxide.com>
- Docs: <https://gpui-query.freeoxide.com/docs/>
- Source: <https://github.com/freeoxide/gpui-query>
- gpui-query: <https://crates.io/crates/gpui-query>
- TanStack Query Persisters: <https://tanstack.com/query/latest/docs/framework/react/plugins/persistQueryClient>

## Author

**hmziqrs**

- Website: <https://hmziq.rs>
- GitHub: <https://github.com/hmziqrs>
- X: <https://x.com/hmziqrs>

## License

MIT. See the [LICENSE](../../LICENSE) file for details.
