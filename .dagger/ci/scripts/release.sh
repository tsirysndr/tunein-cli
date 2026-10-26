#!/usr/bin/env bash
set -euo pipefail
# A failed view may mean an API outage, so a failed create must remain fatal.
if [[ "$1" == create ]]; then
  if gh release view "$RELEASE_TAG" >/dev/null 2>&1; then
    gh release edit "$RELEASE_TAG" --draft=false --prerelease=false
  else
    gh release create "$RELEASE_TAG" --verify-tag --generate-notes --draft=false --prerelease=false
  fi
fi
shopt -s nullglob
assets=()
for asset in ./*.tar.gz ./*.zip ./*.deb ./*.rpm ./*.sha256; do [[ ! -f "$asset" ]] || assets+=("$asset"); done
(( ${#assets[@]} > 0 )) || { echo "No release assets supplied" >&2; exit 1; }
gh release upload "$RELEASE_TAG" "${assets[@]}" --clobber
