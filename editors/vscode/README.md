# whippyunits for VS Code

Makes rust-analyzer show whippyunits types as units instead of const-generic soup.

|            |                                                                                                                                             |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| **Before** | `let energy: Quantity<Scale<_2<0>, _3<0>, _5<0>, _Pi<0>>, Dimension<_M<1>, _L<2>, _T<-2>, _I<0>, _Θ<0>, _N<0>, _J<0>, _A<0>>, f64>` |
| **After**  | `let energy: Quantity<J, f64>`                                                                                                              |

It rewrites rust-analyzer's **hover tooltips**, **inlay type hints** and **diagnostic messages**. Double-clicking a rewritten type hint inserts a `qty!(...)` annotation instead of the raw type.

rust-analyzer itself is untouched: the extension hooks into the rust-analyzer extension's language client inside VS Code, so there is no proxy binary to install or configure, and it works wherever the rust-analyzer extension works — dev containers, Codespaces and SSH remotes included. (For other editors, use the [`lsp-proxy`](../../lsp-proxy) binary instead; both use the same formatting code.)

## Requirements

- The [rust-analyzer extension](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer). It is declared as a dependency, so VS Code installs it automatically.
- VS Code 1.90 or later.
- A toolchain rust-analyzer's bundled server can work with. rust-analyzer only supports proc-macro servers from roughly the last year of toolchains; if the workspace's `rust-toolchain.toml` pins something older, the extension shows a "toolchain too old" warning and no proc macro expands. Move the pin, or give rust-analyzer its own toolchain with the workspace setting `"rust-analyzer.server.extraEnv": { "RUSTUP_TOOLCHAIN": "stable" }` (in a dev container, under `customizations.vscode.settings`). Don't take the warning's other suggestion — switching to the toolchain's *own* rust-analyzer — for an old toolchain: that server predates fixes whippyunits needs.

## Install

### From a `.vsix`

```sh
code --install-extension whippyunits-vscode.vsix
```

In a remote or dev-container window, use the Command Palette instead: **Extensions: Install from VSIX...** installs it on the remote side, where rust-analyzer runs.

### Build it yourself

Needs Rust (via `rustup`) and Node 22.

```sh
cd editors/vscode
npm ci
npm run package        # -> whippyunits-vscode.vsix
```

The first build downloads `wasm-bindgen` and adds the `wasm32-unknown-unknown` target through `rustup`.

Match the extension version to the `whippyunits` version your project uses. The formatter parses the exact type layout of its release; a mismatch shows up as types left unformatted.

## Dev containers

Add the extension to `customizations.vscode.extensions` in `devcontainer.json`. A path to a `.vsix` works there as well as a Marketplace id, provided the file exists before VS Code attaches — build or download it in `onCreateCommand`, which VS Code waits for (it does **not** wait for `postCreateCommand`):

```jsonc
{
  "onCreateCommand": "cd editors/vscode && npm ci && npm run package",
  "customizations": {
    "vscode": {
      "extensions": [
        "rust-lang.rust-analyzer",
        "${containerWorkspaceFolder}/editors/vscode/whippyunits-vscode.vsix"
      ]
    }
  }
}
```

This repository's own [`.devcontainer/devcontainer.json`](../../.devcontainer/devcontainer.json) does exactly that. In a project that merely depends on whippyunits, commit a built `.vsix` (or download one in `onCreateCommand`) and point at its path.

The extension's own settings are ordinary user settings, so your choices follow you into every container.

## Using it

A **`⚖ whippyunits`** item appears at the right of the status bar whenever a Rust file is open:

| Status bar         | Meaning                                                                                   |
| ------------------ | ----------------------------------------------------------------------------------------- |
| `⚖ whippyunits`    | Hooked into rust-analyzer; types are being rewritten.                                     |
| `⚖ whippyunits ⊘`  | Hooked, but rewriting is switched off.                                                    |
| `⚖ whippyunits 🕓` | Waiting for rust-analyzer's server to start.                                              |
| `⚖ whippyunits ⚠`  | Something is wrong (rust-analyzer missing, unsupported version, formatter failed to load). Hover for details, click → *Show log*. |

Click it for the menu: toggle rewriting, switch between compact (`Quantity<m, f64>`) and verbose display, choose which of hovers / inlay hints / diagnostics to rewrite, re-apply the hooks, restart rust-analyzer, or open the log. Every toggle writes a user setting, and changes apply immediately — diagnostics are re-published and inlay hints re-requested; hovers update the next time you hover.

### Settings

| Setting                                            | Default | Effect                                                                                  |
| -------------------------------------------------- | ------- | --------------------------------------------------------------------------------------- |
| `whippyunits.enabled`                              | `true`  | Master switch.                                                                          |
| `whippyunits.verbose`                              | `false` | Full dimension and scale breakdown instead of the unit symbol (hovers and diagnostics; inlay hints are always compact). |
| `whippyunits.includeRaw`                           | `false` | Also show the original `Quantity<...>` type in hovers, in a *Raw* section.              |
| `whippyunits.rewrite.hover`                        | `true`  | Rewrite hover tooltips.                                                                 |
| `whippyunits.rewrite.inlayHints`                   | `true`  | Rewrite inlay hints.                                                                    |
| `whippyunits.rewrite.diagnostics`                  | `true`  | Rewrite diagnostic messages.                                                            |
| `whippyunits.inlayHints.doubleClickInsertsUnitMacro` | `true` | Double-clicking a rewritten type hint inserts `: qty!(...)` rather than the raw type.  |

### Commands

`whippyunits: Show Menu`, `whippyunits: Toggle Type Rewriting`, `whippyunits: Show Log`, `whippyunits: Re-apply rust-analyzer Hooks`.

## How it works, and what can break

- The rust-analyzer extension exports its `vscode-languageclient` client, and that library consults `client.middleware` on every request. This extension wraps rust-analyzer's `provideHover`, `provideInlayHints`, `resolveInlayHint` and `handleDiagnostics` middleware, calling the originals first and rewriting what they return. When rust-analyzer restarts it creates a new client; the extension notices and re-hooks.
- The rewriting is done by the `lsp-proxy` crate's formatting code compiled to WebAssembly ([`formatter-wasm/`](formatter-wasm)), so hovers look the same as with the proxy or the `whippyunits-pretty` CLI. It is textual: it recognises `Quantity<Scale<...>, Dimension<...>, T>` in rust-analyzer's output and has no access to rust-analyzer's type information.
- rust-analyzer marks the exported `client` as an unstable API. If a rust-analyzer update removes or changes it, this extension shows `⚠` in the status bar and rust-analyzer's output is left as-is — nothing else is affected.

## Development

```
editors/vscode/
├── src/               extension (TypeScript)
│   └── test/          node tests with a stubbed `vscode` module
├── formatter-wasm/    Rust crate: wasm-bindgen wrapper around lsp-proxy's formatter
├── wasm/              generated by `npm run build:wasm` (gitignored)
└── out/               generated by `npm run compile` (gitignored)
```

| Command              | Does                                                                     |
| -------------------- | ------------------------------------------------------------------------ |
| `npm run build:wasm` | `wasm-pack build` of `formatter-wasm` into `wasm/`                       |
| `npm run compile`    | `tsc` into `out/`                                                        |
| `npm run build`      | both                                                                     |
| `npm test`           | compile, then `node --test` (needs a prior `build:wasm`)                 |
| `npm run package`    | build, then `vsce package` → `whippyunits-vscode.vsix`                   |

To debug: open the `editors/vscode` folder in VS Code and press **F5**. That launches an Extension Development Host with the extension loaded from source; open a whippyunits project in it and hover over a quantity.
