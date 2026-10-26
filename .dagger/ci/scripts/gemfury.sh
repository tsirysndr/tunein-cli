#!/usr/bin/env bash
set -euo pipefail
# Both fields come through Dagger Secret arguments. Never put credentials in a URL.
[[ "$FURY_ACCOUNT" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo "Invalid Gemfury account" >&2; exit 1; }
shopt -s nullglob
for pkg in /assets/*.deb /assets/*.rpm; do
  curl --fail --silent --show-error --user "$FURY_TOKEN:" \
    --form "package=@${pkg}" "https://push.fury.io/${FURY_ACCOUNT}/"
done
