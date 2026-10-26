#!/usr/bin/env bash
set -euo pipefail

# Pin the exact beta CLI and verify the official release checksum.
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) url="https://dl.dagger.io/dagger/releases/1.0.0-beta.21/dagger_v1.0.0-beta.21_darwin_arm64.tar.gz"; checksum="781f4862c022a19b9a4c498b59e1d9cfda1ee3cdc8ce210e4545bd8842a7416d" ;;
  Darwin-x86_64) url="https://dl.dagger.io/dagger/releases/1.0.0-beta.21/dagger_v1.0.0-beta.21_darwin_amd64.tar.gz"; checksum="fdbe191c965d240ee9cc90719fa9dca1586805cbb89b0a0087314a84a0937354" ;;
  Linux-aarch64) url="https://dl.dagger.io/dagger/releases/1.0.0-beta.21/dagger_v1.0.0-beta.21_linux_arm64.tar.gz"; checksum="ba9d55638636918a3404313511bceba5cc0af22f487c1c1b6a5ba60ce70a61bb" ;;
  Linux-x86_64) url="https://dl.dagger.io/dagger/releases/1.0.0-beta.21/dagger_v1.0.0-beta.21_linux_amd64.tar.gz"; checksum="c5ac777a5c99089869660943381136e22177b59a46412e310c2fef594760a041" ;;
  *) echo "Unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

destination="$(cd "$(dirname "$0")" && pwd)/bin"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
curl --fail --silent --show-error --location "$url" -o "$temporary/dagger.tar.gz"
if command -v sha256sum >/dev/null; then
  printf "%s  %s\n" "$checksum" "$temporary/dagger.tar.gz" | sha256sum -c -
else
  printf "%s  %s\n" "$checksum" "$temporary/dagger.tar.gz" | shasum -a 256 -c -
fi
mkdir -p "$destination"
tar -xzf "$temporary/dagger.tar.gz" -C "$destination" dagger
"$destination/dagger" version
if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf "%s\n" "$destination" >> "$GITHUB_PATH"
fi
