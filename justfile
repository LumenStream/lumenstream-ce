# justfile for lumenstream

# List all available recipes
default:
    @just --list

# Build for local development
build:
    cargo build --workspace

# Build with release optimizations
build-release:
    cargo build --workspace --release

# Check code without producing artifacts
check:
    cargo check --workspace

# Run workspace tests
test:
    cargo test --workspace

# Run all targets (including examples / benches / tests)
test-all:
    cargo test --workspace --all-targets

# Run tests with output
test-verbose:
    cargo test --workspace --all-targets -- --nocapture

# Run clippy linter
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Format check (CI-friendly)
fmt-check:
    cargo fmt --all -- --check

# Run ls-app
run-app:
    cargo run -p ls-app

# Validate shell script syntax
scripts-check:
    bash -n ../scripts/contract_test_api.sh ../scripts/replay_client_callchains.sh docker/fullstack-entrypoint.sh docker/web-entrypoint.sh docker/write-web-runtime-config.sh scripts/export_ce_upstream.sh scripts/init_commercial_downstream.sh scripts/sync_from_ce_upstream.sh

# Replay typical client callchains
replay-callchains:
    bash ../scripts/replay_client_callchains.sh

# Run API contract tests
contract-test:
    bash ../scripts/contract_test_api.sh

# Frontend type check
web-check:
    cd web && bun run check

# Frontend tests
web-test:
    cd web && bun run test

# Frontend lint check
web-lint:
    cd web && bun run lint

web-format:
    cd web && bun run format

# Frontend format check
web-fmt-check:
    cd web && bun run format:check

# Export current workspace as CE upstream snapshot
export-ce target:
    bash scripts/export_ce_upstream.sh {{target}}

# Export current workspace as CE upstream snapshot, replacing target if it exists
export-ce-force target:
    bash scripts/export_ce_upstream.sh --force {{target}}

# Initialize a local commercial downstream workspace from current repo
init-commercial target upstream="":
    bash scripts/init_commercial_downstream.sh {{target}} {{upstream}}

# Initialize a local commercial downstream workspace from current repo, replacing target if it exists
init-commercial-force target upstream="":
    bash scripts/init_commercial_downstream.sh --force {{target}} {{upstream}}

# Sync current branch from CE upstream remote
sync-from-upstream remote="upstream" branch="main":
    bash scripts/sync_from_ce_upstream.sh {{remote}} {{branch}}

# Cut local CE/commercial split repositories under a base directory
cut-repos base:
    bash scripts/cut_split_repositories.sh {{base}}

# Cut local CE/commercial split repositories under a base directory, replacing existing outputs
cut-repos-force base:
    bash scripts/cut_split_repositories.sh --force {{base}}

# Run local prek hooks
precommit:
    prek run --all-files

# Install prek git hook + hook environments
precommit-install:
    prek install
    prek install-hooks

# Full CI check
ci: fmt-check test scripts-check web-check web-test web-lint web-fmt-check build-release
    @echo "All CI checks passed."

# Quick development cycle
dev: fmt check test
    @echo "Development checks passed."
