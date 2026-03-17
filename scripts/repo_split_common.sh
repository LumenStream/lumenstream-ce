#!/usr/bin/env bash
set -euo pipefail

repo_split_root_dir() {
  cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
}

repo_split_current_branch() {
  local root_dir="$1"
  git -C "$root_dir" rev-parse --abbrev-ref HEAD
}

repo_split_current_commit() {
  local root_dir="$1"
  git -C "$root_dir" rev-parse HEAD
}

repo_split_require_clean_worktree() {
  local root_dir="$1"
  if [ -n "$(git -C "$root_dir" status --porcelain)" ]; then
    echo "source repository has uncommitted changes: $root_dir" >&2
    echo "commit or stash changes before cutting split repositories" >&2
    exit 1
  fi
}

repo_split_prepare_target() {
  local target_dir="$1"
  local force="${2:-0}"

  if [ -e "$target_dir" ]; then
    if [ "$force" != "1" ]; then
      echo "target already exists: $target_dir" >&2
      echo "re-run with --force to replace it" >&2
      exit 1
    fi
    rm -rf "$target_dir"
  fi
}

repo_split_clone_repo() {
  local root_dir="$1"
  local target_dir="$2"
  local branch="$3"

  git clone --branch "$branch" "$root_dir" "$target_dir" >/dev/null 2>&1
}

repo_split_write_role_manifest() {
  local target_dir="$1"
  local role="$2"
  local source_repo="$3"
  local source_branch="$4"
  local source_commit="$5"
  local upstream_ref="${6:-}"

  cat >"$target_dir/.lumenstream-repository-role.toml" <<EOF
role = "$role"
source_repository = "$source_repo"
source_branch = "$source_branch"
source_commit = "$source_commit"
generated_at_utc = "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
upstream = "$upstream_ref"
EOF
}

repo_split_write_ce_docs() {
  local target_dir="$1"
  local source_repo="$2"
  local source_branch="$3"
  local source_commit="$4"

  mkdir -p "$target_dir/docs"

  cat >"$target_dir/README.generated.md" <<EOF
# lumenstream-ce skeleton

This workspace was generated from \`$source_repo\`.

- role: CE upstream
- source branch: \`$source_branch\`
- source commit: \`$source_commit\`

Community Edition keeps:

- Jellyfin / Emby compatibility core
- media libraries / scan / scraper / search / playback
- Agent workflows
- playback domains / LumenBackend / multi-end routing
- user management

Commercial-only capabilities should remain downstream overlays.
EOF

  cat >"$target_dir/docs/generated-repository.md" <<EOF
# Generated repository profile

This repository was cut from \`$source_repo\` as the **Community Edition upstream**.

## Role

- public/community-facing upstream
- accepts shared core fixes and CE-safe features
- remains the sync source for commercial downstream

## Local bootstrap

\`\`\`bash
git remote -v
git status
\`\`\`

If needed, add your real hosting remote manually after publishing this repository.
EOF
}

repo_split_write_commercial_docs() {
  local target_dir="$1"
  local source_repo="$2"
  local source_branch="$3"
  local source_commit="$4"
  local upstream_ref="${5:-}"

  mkdir -p "$target_dir/docs"

  cat >"$target_dir/README.generated.md" <<EOF
# lumenstream-commercial skeleton

This workspace was generated from \`$source_repo\`.

- role: commercial downstream
- source branch: \`$source_branch\`
- source commit: \`$source_commit\`
- upstream: \`${upstream_ref:-<set later>}\`

Commercial downstream is expected to:

- absorb CE upstream fixes regularly
- keep billing / advanced traffic / invite rewards / audit export overlays
- preserve compatibility with CE shared core
EOF

  cat >"$target_dir/docs/generated-repository.md" <<EOF
# Generated repository profile

This repository was cut from \`$source_repo\` as the **Commercial Edition downstream**.

## Role

- private downstream overlay
- merges from CE upstream
- carries commercial-only modules and packaging

## Upstream sync

\`\`\`bash
git fetch upstream main
git merge --no-ff upstream/main
\`\`\`

Current upstream reference: \`${upstream_ref:-not configured}\`
EOF
}

repo_split_commit_overlay() {
  local target_dir="$1"
  local message="$2"

  git -C "$target_dir" add .lumenstream-repository-role.toml README.generated.md docs/generated-repository.md
  git -C "$target_dir" \
    -c user.name="LumenStream Split Bot" \
    -c user.email="split-bot@lumenstream.local" \
    commit -m "$message" >/dev/null 2>&1
}
