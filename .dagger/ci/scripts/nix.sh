#!/bin/sh
set -eu
cd /src
cachix_path=$(nix build --inputs-from . nixpkgs#cachix --no-link --print-out-paths)
"$cachix_path/bin/cachix" use tsirysndr
case "$1" in
  flake) nix build -L ;;
  package) nix-build -E 'with import <nixpkgs> { }; callPackage ./nix/package.nix { }' ;;
  *) echo "Unknown Nix build mode" >&2; exit 1 ;;
esac
if [ -n "${CACHIX_AUTH_TOKEN:-}" ]; then
  "$cachix_path/bin/cachix" push tsirysndr ./result
fi
