# zed-tsrs

An independent, **opt-in** Zed extension for the [tsrs](https://github.com/oxc-project/tsrs) Rust TypeScript/JavaScript language server. Supports TypeScript, TSX, JavaScript, and JSX using Zed's existing grammars.

Installing this extension does not select it for existing projects. It does not proxy LSP, change other servers' settings, or automatically switch servers based on a project's TypeScript version.

## Enable per project

Add this to the target project's `.zed/settings.json`:

```json
{
  "languages": {
    "TypeScript": { "language_servers": ["tsrs"] },
    "TSX": { "language_servers": ["tsrs"] },
    "JavaScript": { "language_servers": ["tsrs"] },
    "JSX": { "language_servers": ["tsrs"] }
  }
}
```

These exclusive lists intentionally omit `vtsls`, `tsgo`, and every other language server for those languages in this project. Adding `"..."` may also start default servers; explicitly adding another server enables both. Other projects retain their own selections. For global opt-in, put the same configuration in Zed's user settings instead. Remove it to return to defaults.

## Executable selection

On every launch the extension resolves:

1. `lsp.tsrs.binary.path`, if set.
2. `tsrs` found through the worktree's PATH.
3. A validated cached installation of the selected npm version.
4. Installation of that exact version using Zed's npm runtime.

A local executable takes precedence even when `package_version` is set. Invalid explicit overrides fail rather than silently using another version. Explicit host paths are launched by Zed, not checked against the WASM sandbox filesystem. Use an absolute native executable path, especially on Windows (not an npm `.cmd` shim); keep its required library assets intact.

```json
{
  "lsp": {
    "tsrs": {
      "binary": {
        "path": "C:\\tools\\tsrs\\tsrs.exe",
        "arguments": ["--lsp", "-stdio"],
        "env": { "RUST_BACKTRACE": "1" }
      }
    }
  }
}
```

The default arguments are **`--lsp -stdio`**, verified against the published pinned release and upstream CLI. `binary.arguments`, when provided, **replaces** the entire argument list (including an empty list). Use it only for a compatible CLI. Custom binaries are not version-probed or upgraded by the extension.

### Managed installations

The default is pinned to **`0.25.0-ts7.1.0-beta`**, not `latest`. To choose another exact published version with the same distribution layout:

```json
{
  "lsp": {
    "tsrs": {
      "settings": {
        "package_version": "0.25.0-ts7.1.0-beta"
      }
    }
  }
}
```

`package_version` is an extension setting, removed before forwarding workspace configuration to the server. Tags and ranges are rejected. Changing the pin does not update an explicit or PATH binary.

Managed installation downloads npm packages using Zed's Node/npm runtime and registry configuration. This requires network access on first use (or after cache deletion), downloads native third-party code, and may require corporate proxy/registry configuration. No paid service or API key is required. Keep npm optional dependencies enabled.

Published targets: Windows and macOS x64/ARM64; Linux x64/ARM64, GNU and musl. 32-bit x86 is unsupported for managed installation. Linux libc detection uses Zed's host Node runtime. The extension launches the native package executable directly, retaining all bundled `.d.ts` files and notices. Separate version/platform directories under the extension work directory's `tsrs-versions/` prevent worktrees with different pins from replacing each other's running binaries. Valid caches are reused without registry version queries. Old versions are intentionally not automatically pruned.

## LSP configuration

Zed merges global/project LSP settings for the current worktree. `lsp.tsrs.initialization_options` is forwarded unchanged. `lsp.tsrs.settings`, excluding `package_version`, is forwarded as workspace configuration. No `tsgo` fallback, forced inlay-hint defaults, code-lens defaults, or JSON-array merging is applied. Only send settings supported by your chosen `tsrs` release; forwarding does not guarantee upstream support.

## Compatibility and limitations

- `tsrs` is experimental and tracks native TypeScript 7. **TypeScript 6 is manual, experimental opt-in**, not a promise of TS6 semantics or feature parity.
- No TypeScript-version detection or automatic selection is implemented. Server choice belongs to Zed.
- The pinned Windows binary passes the standalone LSP smoke test. Linux/macOS native execution has not been verified locally. ARM64/musl mappings have unit coverage, not local native execution coverage.
- The pinned release acknowledges shutdown but may exit with code 1 and exactly `context canceled`; the smoke test narrowly tolerates that known upstream behavior. Do not interpret it as clean exit-code conformance.
- Dev-extension loading, actual Zed server selection/isolation, npm/pnpm/yarn monorepos, and full feature compatibility still need the manual checks in [docs/validation.md](docs/validation.md).

## Troubleshooting

- **No server starts:** ensure the `tsrs` extension loads, the exact language name is selected, and Zed is recent enough to support opt-in manifests. Inspect Zed's language server logs.
- **Two TypeScript servers start:** use the exclusive lists above, not `"..."` or an additional server name.
- **Wrong binary/version:** inspect `binary.path` and the worktree PATH first; both override managed pins. Restart the language server after changing settings.
- **Cannot launch a custom binary:** check the absolute host path, executable permissions/architecture, CLI compatibility, and neighboring library files. npm wrappers may not be directly executable on Windows.
- **Installation fails:** check Zed's npm runtime, proxy/registry access, optional dependency settings, and the exact release's native package availability. Errors identify the missing package or asset. No fallback to a mismatched cached version is attempted.
- **Broken cache:** stop affected servers, remove only the affected `tsrs-versions/<version>/<platform>` directory from the extension work directory, and restart. The next launch installs again.
- **Module resolution differs:** reproduce with the same native `tsrs` version outside Zed, preserving the project's package manager, tsconfig, and workspace root. Report upstream compiler behavior separately from extension launch issues.

## Development checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --target wasm32-wasip1
node scripts/smoke-test.mjs /absolute/path/to/native/tsrs
```

The dependency-free smoke test creates a temporary project, checks all four language IDs plus completion/hover/definition/diagnostics, and tests shutdown with bounded deadlines. Run it against the pinned native distribution on each platform you want to validate. It does not automate the Zed GUI or certify all project/package-manager combinations.

## License and attribution

Apache-2.0; see [LICENSE](LICENSE) and [NOTICE](NOTICE). The original repository scaffold and extension patterns came from [zed-extensions/tsgo](https://github.com/zed-extensions/tsgo); this adapter replaces the inherited tsgo-specific behavior. `tsrs` is a separate upstream project with its own notices, retained in managed packages.
