# Schema snapshot

`introspection.json` was captured from Dagger **v1.0.0-beta.21**
(commit `a7a2e64e853be92728f05aad9dc07bebe63c7448`) using the upstream
`cmd/codegen/introspection/introspection.graphql` query. Its schema version is
`v1.0.0`. It produces 924 API fields in 97 PureScript modules.

Run `npm run generate` to regenerate bindings and `npm run check-generated`
to detect drift. Pass a module-specific introspection file to
`node scripts/generate.mjs /path/to/schema.json` for dependency bindings.

Recursive input records use a `newtype` constructor; other input records remain
plain records. JSON field names match the GraphQL schema, including reserved
PureScript words such as `kind`.

The upstream schema and API descriptions are Apache-2.0 licensed; see
[LICENSE](LICENSE) and [../NOTICE](../NOTICE).

For runnable usage, see the [examples](../examples/README.md). The
[CI workflow](../../.github/workflows/test.yml) checks that generated bindings
match this snapshot.
