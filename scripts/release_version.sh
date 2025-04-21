#!/usr/bin/env bash
set -euo pipefail

VERSION="$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[0].version')"
DATE="$(date +%Y-%m-%d)"

echo "Preparing release version $VERSION"

# Check if the version is already in the changelog
if git rev-parse -q --verify "v$VERSION" >/dev/null 2>&1; then
  echo "Error: git tag v$VERSION already exists! Aborting."
  exit 1
fi

# Change to version and date if Unreleased
if ! grep -qE "^## \\[$VERSION\\]" CHANGELOG.md; then
  if grep -qE "^## \\[Unreleased\\]" CHANGELOG.md; then
    echo "Renaming [Unreleased] to [$VERSION] - $DATE"
    sed -i "s/^## \\[Unreleased\\]/## [$VERSION] - $DATE/" CHANGELOG.md
  else
    echo "Error: No '## [Unreleased]' or '## [$VERSION]' heading found in CHANGELOG.md."
    exit 1
  fi
fi

# Extract the changes text for this version
CHANGELOG_CONTENT="$(
  awk "/^## \\[$VERSION\\]/ {found=1; next} /^## \\[/ {found=0} found" CHANGELOG.md
)"

if [ -z "$(echo "$CHANGELOG_CONTENT" | sed 's/^[[:space:]]*\$//')" ]; then
  echo "Error: No content found for version $VERSION in CHANGELOG.md!"
  exit 1
fi

echo "Changelog content for version $VERSION:"
echo "$CHANGELOG_CONTENT"

echo "Creating signed git tag v$VERSION"
echo "$CHANGELOG_CONTENT" | git tag -a "v$VERSION" -F -

echo "Tag v$VERSION created."

# Check to continue
read -r -p "Tag v$VERSION created locally. Push tag to origin? [y/N] " answer
case "$answer" in
  [Yy]* )
    echo "Pushing tag v$VERSION to origin..."
    git push origin "v$VERSION"
    ;;
  * )
    echo "Skipping tag push."
    exit 0
    ;;
esac

read -r -p "Create GitHub release with 'gh release create v$VERSION --notes-from-tag'? [y/N] " answer
case "$answer" in
  [Yy]* )
    echo "Creating GitHub release from tag..."
    gh release create "v$VERSION" --notes-from-tag --verify-tag --title "v$VERSION"
    ;;
  * )
    echo "Skipping GitHub release creation."
    ;;
esac
