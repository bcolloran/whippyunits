import * as path from "path";
import type * as vscode from "vscode";
import type { Settings } from "../config";

/** The extension root, where `npm run build:wasm` puts the wasm module. */
export const EXTENSION_ROOT = path.resolve(__dirname, "..", "..");

export const DEFAULT_SETTINGS: Settings = {
  enabled: true,
  verbose: false,
  includeRaw: false,
  rewriteHover: true,
  rewriteInlayHints: true,
  rewriteDiagnostics: true,
  unitMacroTextEdit: true,
};

/** `Quantity` types exactly as rust-analyzer prints them for whippyunits 0.3 (same fixtures as lsp-proxy/src/tests.rs). */
export const RAW_LENGTH =
  "Quantity<Unit<Scale<_2<0>, _3<0>, _5<0>, _Pi<0>>, Dimension<_M<0>, _L<1>, _T<0>, _I<0>, _Θ<0>, _N<0>, _J<0>, _A<0>>>, f64>";
export const RAW_ENERGY =
  "Quantity<Unit<Scale<_2<0>, _3<0>, _5<0>, _Pi<0>>, Dimension<_M<1>, _L<2>, _T<-2>, _I<0>, _Θ<0>, _N<0>, _J<0>, _A<0>>>, f64>";

export const HOVER_ENERGY = "```rust\nlet energy_j: " + RAW_ENERGY + " = 5.0.joules();\n```";

export const fakeLog = {
  trace() {},
  debug() {},
  info() {},
  warn() {},
  error() {},
  show() {},
} as unknown as vscode.LogOutputChannel;

export function tick(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** rust-analyzer's hover on `pub vel32: qty!(m / s, f32)`: the declared type, wrapper and `{const}` included. */
export const FIELD_HOVER =
  "```rust\npub vel32: <Helper<{const}, Quantity<Unit<Scale<_2<0>, _3<0>, _5<0>, _Pi<0>>, Dimension<_M<0>, _L<1>, _T<{const}>, _I<0>, _Θ<0>, _N<0>, _J<0>, _A<0>>>, f32, ()>> as GetSecondGeneric>::Type\n```";
export const FIELD_LINE = "    pub vel32: qty!(m / s, f32),";

/** Just enough of a TextDocument for `Rewriter.hover`. */
export function fakeDocument(line: string): vscode.TextDocument {
  return { lineAt: () => ({ text: line }) } as unknown as vscode.TextDocument;
}
