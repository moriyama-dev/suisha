#!/bin/sh
# Builds the suisha-cli package: the `suisha` command, the root helpers and
# the apt source, without anything a window needs.
# Usage: scripts/build-cli-deb.sh [PATH/TO/suisha-cli] [OUTPUT_DIRECTORY]
# Build the binary first: cargo build --release -p suisha-cli
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="${1:-$ROOT/target/release/suisha-cli}"
OUT="${2:-$ROOT/target/release/bundle/deb}"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
ARCH="$(dpkg --print-architecture)"
# The oldest glibc the binary runs on, read from the symbols it uses.
GLIBC="$(objdump -T "$BINARY" | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/^GLIBC_//' | sort -V | tail -n 1)"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
PKG="$STAGE/suisha-cli"

install -D -m 0755 "$BINARY" "$PKG/usr/bin/suisha"
for helper in suisha-root suisha-auto-update suisha-self-update suisha-auto-update-access; do
    install -D -m 0755 "$ROOT/packaging/linux/$helper" "$PKG/usr/lib/suisha/$helper"
done
install -D -m 0644 "$ROOT/packaging/linux/dev.moriyama.suisha.policy" \
    "$PKG/usr/share/polkit-1/actions/dev.moriyama.suisha.policy"
install -D -m 0644 "$ROOT/packaging/linux/suisha-archive-keyring.asc" \
    "$PKG/usr/share/keyrings/suisha-archive-keyring.asc"
install -D -m 0644 "$ROOT/LICENSE" "$PKG/usr/share/doc/suisha-cli/copyright"

mkdir "$PKG/DEBIAN"
install -m 0755 "$ROOT/packaging/linux/postinst" "$ROOT/packaging/linux/postrm" "$PKG/DEBIAN/"
(cd "$PKG" && find usr -type f -exec md5sum {} + > DEBIAN/md5sums)

# Until 0.6.0 the suisha package had the command and the helpers itself.
cat > "$PKG/DEBIAN/control" <<CONTROL
Package: suisha-cli
Version: $VERSION
Architecture: $ARCH
Maintainer: Yoshiro Moriyama <7315004+moriyama-dev@users.noreply.github.com>
Installed-Size: $(du -sk "$PKG/usr" | cut -f1)
Depends: libc6 (>= $GLIBC), libgcc-s1
Recommends: sudo
Breaks: suisha (<< 0.6.0)
Replaces: suisha (<< 0.6.0)
Section: admin
Priority: optional
Homepage: https://github.com/moriyama-dev/suisha
Description: Keep a computer's software up to date from a terminal
 The suisha command brings apt, snap, Flatpak and Homebrew together: see what
 has updates, update everything at once or on a schedule, search, install and
 remove, and set up a new computer from a saved list. It needs no desktop, so
 it works on servers and over SSH. The Suisha window (package suisha) is built
 on it.
CONTROL

mkdir -p "$OUT"
dpkg-deb --root-owner-group --build "$PKG" "$OUT/suisha-cli_${VERSION}_${ARCH}.deb"
