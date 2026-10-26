# Vendored PureScript Dagger SDK

Source: https://github.com/tsirysndr/dagger-sdk/tree/156eb76/purescript
Commit: 156eb76d1c67e4f39d9bdbae28c76631ffe9c056 (Dagger v1.0.0-beta.21 migration).

The SDK is not published yet. This snapshot makes CI independent of a sibling
checkout or an unpublished remote commit. Tests are omitted. Keep LICENSE,
NOTICE, and schema/LICENSE with redistributions. Bootstrap Go bindings are
committed because beta module execution does not generate them automatically.

To update, copy the tracked purescript directory from a reviewed upstream
commit, excluding test/, then run `python3 .dagger/generate.py` at the tunein-cli root.

Local patch: `src/Dagger/Internal/Runtime.js` uses Node's HTTP transport instead
of fetch so the configured deadline also governs delayed response headers.
Regression coverage: `.dagger/tests/transport.test.mjs`. Preserve this patch
when refreshing the vendored SDK until it is incorporated upstream.

Snapshot and transport patch copied from Rocksky commit 85042e1e.

Local runtime patch: use Node 24 (`node:24-bookworm-slim`) for SDK compilation
and module execution. Preserve this image choice when refreshing the SDK.
