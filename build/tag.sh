#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>" >&2
  echo "Example: $0 0.2.0" >&2
  exit 1
fi

VERSION="${1#v}"

# Validate semver (major.minor.patch) with optional -beta.N suffix
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-beta\.[0-9]+)?$ ]]; then
  echo "Error: version must be semver (e.g. 1.2.3 or 1.2.3-beta.1)" >&2
  exit 1
fi

IS_BETA=false
[[ "$VERSION" == *beta* ]] && IS_BETA=true

BRANCH=$(git branch --show-current)

if [ "$IS_BETA" = true ]; then
  # Beta releases must originate from a feature branch, not master
  if [ "$BRANCH" = "master" ]; then
    echo "Error: beta releases must be tagged from a feature branch, not master" >&2
    exit 1
  fi
else
  # Stable releases must originate from master
  if [ "$BRANCH" != "master" ]; then
    echo "Error: stable releases must be tagged from master (currently on $BRANCH)" >&2
    exit 1
  fi
fi

# Ensure clean working tree
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree is dirty — commit or stash changes first" >&2
  exit 1
fi

# Ensure local branch is up to date with origin
git fetch origin "$BRANCH" --quiet
LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse "origin/${BRANCH}")
if [ "$LOCAL" != "$REMOTE" ]; then
  echo "Error: local $BRANCH ($LOCAL) differs from origin/$BRANCH ($REMOTE)" >&2
  echo "Pull or push first." >&2
  exit 1
fi

# Ensure tag doesn't already exist
if git rev-parse "v${VERSION}" >/dev/null 2>&1; then
  echo "Error: tag v${VERSION} already exists" >&2
  exit 1
fi

# Ensure version is actually changing
CURRENT=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
if [ "$CURRENT" = "$VERSION" ]; then
  echo "Error: Cargo.toml already has version ${VERSION}" >&2
  exit 1
fi

# Update version in Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml

# Verify the version was actually updated
if ! grep -q "^version = \"${VERSION}\"" Cargo.toml; then
  echo "Error: failed to update version in Cargo.toml" >&2
  exit 1
fi

# Regenerate Cargo.lock with the new version
cargo check

# Commit, tag, and push
git add Cargo.toml Cargo.lock
git commit -m "Bump to v${VERSION}"
git tag "v${VERSION}"

echo "Ready to push v${VERSION}. Review:"
git log --oneline -1
echo ""
read -rp "Push to origin/${BRANCH} with tag? [y/N] " confirm
if [[ "$confirm" =~ ^[Yy]$ ]]; then
  git push origin "$BRANCH"
  git push origin "v${VERSION}"
  echo "Pushed v${VERSION}"
else
  echo "Aborted. To undo: git reset HEAD~1 && git tag -d v${VERSION}"
fi
