import * as vscode from "vscode";

export const SECTION = "whippyunits";

/** Snapshot of the `whippyunits.*` settings. */
export interface Settings {
  enabled: boolean;
  verbose: boolean;
  includeRaw: boolean;
  rewriteHover: boolean;
  rewriteInlayHints: boolean;
  rewriteDiagnostics: boolean;
  unitMacroTextEdit: boolean;
}

export function readSettings(): Settings {
  const config = vscode.workspace.getConfiguration(SECTION);
  return {
    enabled: config.get<boolean>("enabled", true),
    verbose: config.get<boolean>("verbose", false),
    includeRaw: config.get<boolean>("includeRaw", false),
    rewriteHover: config.get<boolean>("rewrite.hover", true),
    rewriteInlayHints: config.get<boolean>("rewrite.inlayHints", true),
    rewriteDiagnostics: config.get<boolean>("rewrite.diagnostics", true),
    unitMacroTextEdit: config.get<boolean>("inlayHints.doubleClickInsertsUnitMacro", true),
  };
}

/** Writes to user settings so the choice follows the user into every workspace (and every dev container). */
export function updateSetting(key: string, value: unknown): Thenable<void> {
  return vscode.workspace
    .getConfiguration(SECTION)
    .update(key, value, vscode.ConfigurationTarget.Global);
}
