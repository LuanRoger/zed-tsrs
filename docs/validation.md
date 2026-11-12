# Implementation and validation status

The implementation plan's adapter, managed installation, settings, automated
checks, and documentation are implemented. This is not certification of every
release acceptance criterion. Automatic version-aware selection remains deferred.

## Automated coverage

- Unit tests: override/PATH/managed precedence, per-worktree pin parsing,
  extension-only setting filtering, exact version validation, eight native
  platform mappings, unsupported CPUs, and cache package/binary/library validation.
- WASI release build: `wasm32-wasip1`, Zed extension API 0.7.0.
- Native smoke script: initialization, TypeScript/TSX/JavaScript/JSX document IDs,
  completion, hover, definition, diagnostics, shutdown and exit.
- CI: Rust checks and native x64 Windows/Linux/macOS distribution smoke tests.

Native smoke coverage does not exercise the Zed WASM host's npm installation API.
A Zed development-extension session is required to verify that path end to end.
The pinned native release may acknowledge shutdown then exit 1 with exactly
`context canceled`; the smoke test tolerates only this known upstream outcome.

## Manual checks before publishing

- [ ] Confirm repository URL/ownership and no `tsrs` extension ID collision in
      Zed's extension registry; this repository does not publish automatically.
- [ ] Install this repository via `zed: install dev extension` on a current Zed.
- [ ] Open a default TS project and confirm no tsrs process starts.
- [ ] Enable the README's exclusive per-project lists; confirm tsrs starts and
      vtsls does not start for those languages.
- [ ] Verify completion, hover, diagnostics, definition, TSX and JSX in Zed.
- [ ] Open a separate default project and confirm its existing server is unchanged.
- [ ] Verify global opt-in and project overrides; restart servers after changes.
- [ ] Test a custom native executable and environment override ahead of PATH/cache.
- [ ] With no PATH executable, verify fresh managed install, offline cache reuse,
      incomplete-cache recovery, and two worktrees pinned to different versions.
- [ ] Test npm, pnpm and yarn projects, including workspace/monorepo boundaries.
- [ ] Exercise experimental TS6 and TS7 projects without claiming semantic parity.
- [ ] Run native installation checks on ARM64 and Linux musl; CI currently runs x64
      GNU/Linux, macOS, and Windows only.
- [ ] Check shutdown in Zed, including the documented upstream exit-code caveat.

Do not mark these as passed based on manifest inspection or standalone LSP tests.
