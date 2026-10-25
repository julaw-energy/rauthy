#!/usr/bin/env bash
# Builds the book into static/book/ so that `cargo build` embeds it and Rauthy serves it at
# /auth/v1/book/. Needs `mdbook` (0.4.x) and `mdbook-admonish`:
#   cargo install mdbook@0.4.52 mdbook-admonish --locked
# Text files are gzip-compressed (the server sends them with Content-Encoding: gzip), legacy font
# formats are dropped (the print page stays, the print button needs it), so the book adds only a few MB to the binary.
set -euo pipefail
cd "$(dirname "$0")"

OUT=../static/book
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mdbook build -d "$TMP"

find "$TMP" -type f \( -name '*.eot' -o -name '*.ttf' -o -name '*.otf' -o -name '*.svg' -path '*FontAwesome*' \) -delete

find "$TMP" -type f \( -name '*.html' -o -name '*.js' -o -name '*.css' -o -name '*.json' -o -name '*.svg' -o -name '*.txt' -o -name '*.xml' \) \
    -exec gzip -9 -n {} \;

rm -rf "$OUT"
mkdir -p "$OUT"
cp -r "$TMP"/. "$OUT"/
echo "book built into static/book: $(du -sh "$OUT" | cut -f1), $(find "$OUT" -type f | wc -l) files"
