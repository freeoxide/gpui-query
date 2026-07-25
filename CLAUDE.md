@AGENTS.md

## Claude Code notes

- AGENTS.md above is the source of truth for layout, commands, features, and release flow. Keep this file thin; edit AGENTS.md when the facts change.
- A committed project skill lives at `.claude/skills/gpui-query/SKILL.md`. Use it for crate work — query, mutation, caching, retry, persistence, HTTP-cache, or observer behavior; the core/client/hook layers; and the http/persist satellites.
- Run `just test` (= `cargo test --all-features`) before claiming any Rust change is done. Bare `cargo test` uses default features (`client`) and skips the `hook`/`persist` test modules.
- Never edit `web/dist/**` or the generated `llms.txt` / `llms-full.txt` — they are produced by `just web-build`. Rebuild the site to refresh them.
