#!/usr/bin/env bash
set -euo pipefail

# Path to the AUR repository
AUR_DIR="${AUR_DIR:-/home/john/devel/cyme-bin}"
PKGBUILD="$AUR_DIR/PKGBUILD"

NEW_VER=${1:-$(grep -m 1 '^version =' Cargo.toml | tr -d '" ' | cut -d'=' -f2)}
# Strip 'v' prefix if present for pkgver
NEW_VER=${NEW_VER#v}
NEW_REL=${2:-1}

echo "Updating PKGBUILD to version $NEW_VER"

# Update pkgver in PKGBUILD
sed -i "s/^pkgver=.*/pkgver=$NEW_VER/" "$PKGBUILD"
# Set rel
sed -i "s/^pkgrel=.*/pkgrel=$NEW_REL/" "$PKGBUILD"

# Function to get sha512sum from GitHub release
get_sum() {
    local arch=$1
    local url="https://github.com/tuna-f1sh/cyme/releases/download/v${NEW_VER}/cyme-v${NEW_VER}-${arch}-unknown-linux-gnu.tar.gz"
    echo "Fetching sum for $arch from $url..." >&2
    curl -sL "$url" | sha512sum | awk '{print $1}'
}

SUM_X86_64=$(get_sum "x86_64")
SUM_AARCH64=$(get_sum "aarch64")

echo "x86_64 sum: $SUM_X86_64"
echo "aarch64 sum: $SUM_AARCH64"

# Update sums in PKGBUILD
sed -i "s/^sha512sums_x86_64=.*/sha512sums_x86_64=('$SUM_X86_64')/" "$PKGBUILD"
sed -i "s/^sha512sums_aarch64=.*/sha512sums_aarch64=('$SUM_AARCH64')/" "$PKGBUILD"

pushd "$AUR_DIR" > /dev/null

# Clean up any old tarballs to avoid conflicts if they exist
rm -f cyme-bin-*.tar.gz

echo "Running makepkg --skippgpcheck --check..."
# We might need to run this for each arch if we want to be thorough, 
# but usually checking the current host arch is enough for a basic check.
makepkg -f --skippgpcheck --check

echo "Updating .SRCINFO..."
makepkg --printsrcinfo > .SRCINFO

popd > /dev/null

echo "Successfully updated PKGBUILD and .SRCINFO to v$NEW_VER"
