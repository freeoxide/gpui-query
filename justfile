# gpui-query
# Run these with: just <recipe>

set positional-arguments

# Default: list available recipes
default:
    @just --list

# ---- Tests ----

# Run all tests with every feature combination
test:
    cargo test --all-features

# Run tests for a specific feature
test-feature feature:
    cargo test --features "{{ feature }}"

# ---- Docs ----

# Build the Docusaurus docs site
docs-build:
    cd website && npm run build

# Start the docs dev server
docs-dev:
    cd website && npm run start

# ---- Website (TanStack Start) ----

# Install web dependencies
web-install:
    cd web && bun install

# Build the web app (copies Docusaurus output into it)
web-build:
    cd web && bun run build

# Start the web dev server
web-dev:
    cd web && bun run dev

# ---- CI Workflows ----

# Trigger a manual crate publish for a given tag (e.g. just publish v0.1.1)
publish tag:
    gh workflow run "Publish Crate (Manual)" --field tag="{{ tag }}"

# Manually trigger the Deploy Website workflow
deploy:
    gh workflow run "Deploy Website"

# Check status of recent workflow runs
ci-status:
    gh run list --limit 10

# Watch a specific workflow run by its ID
ci-watch run_id:
    gh run watch {{ run_id }}

# View logs for a specific run
ci-logs run_id:
    gh run view {{ run_id }} --log

# ---- Releases ----

# Open the GitHub releases page
releases:
    open "https://github.com/freeoxide/gpui-query/releases"

# Create a new release: bumps version in CHANGELOG, commits, and pushes
# Usage: just release 0.1.2
release version:
    @echo "Preparing release {{ version }}..."
    @grep -q "\[{{ version }}\]" CHANGELOG.md || (echo "Error: [{{ version }}] not found in CHANGELOG.md. Add a changelog entry first." && exit 1)
    git add CHANGELOG.md
    git diff --cached --quiet && echo "Nothing staged. Make sure CHANGELOG.md has changes." && exit 1
    git commit -m "chore: release v{{ version }}"
    git push origin master
    @echo "Pushed. The Changelog Release workflow will handle the rest: tag, GitHub Release, publish, deploy."

# ---- Secrets ----

# Set CARGO_REGISTRY_TOKEN (prompts for the token value)
set-cargo-token:
    gh secret set CARGO_REGISTRY_TOKEN

# List configured secrets
secrets:
    gh secret list
