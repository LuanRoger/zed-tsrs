# Unreleased — 0.3.0

- Replace the inherited tsgo adapter with independent server ID `tsrs`.
- Require explicit selection for TypeScript, TSX, JavaScript and JSX; leave
  existing project defaults untouched.
- Resolve configured binary, worktree PATH, then an exact pinned native npm
  distribution, retaining libraries and isolated version/platform caches.
- Support binary arguments/environment, managed version pins, and forwarding
  user initialization/workspace configuration without inherited tsgo defaults.
- Add platform/settings/cache tests, WASI build CI and native LSP smoke tests.

Existing scaffold settings using `typescript-ls` or `tsgo` are not read. Opt in
using `language_servers: ["tsrs"]` and move intended options under `lsp.tsrs`.

This is a development implementation, not a published or fully certified stable
release. See validation.md for remaining in-editor/platform/package-manager checks.
