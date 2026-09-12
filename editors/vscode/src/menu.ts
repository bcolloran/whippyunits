import * as vscode from "vscode";
import { readSettings, updateSetting } from "./config";
import type { RustAnalyzerHook } from "./raHook";

interface MenuItem extends vscode.QuickPickItem {
  run?(): Thenable<unknown> | void;
}

/** The quick-pick menu behind the status bar item. Every toggle writes a user setting. */
export async function showMenu(
  hook: RustAnalyzerHook,
  log: vscode.LogOutputChannel,
): Promise<void> {
  const settings = readSettings();
  const check = (on: boolean) => (on ? "$(pass-filled)" : "$(circle-large-outline)");
  const toggle = (key: string, current: boolean) => () => updateSetting(key, !current);

  const items: MenuItem[] = [
    {
      label: `${check(settings.enabled)} Rewrite whippyunits types`,
      description: settings.enabled ? "on" : "off",
      run: toggle("enabled", settings.enabled),
    },
    {
      label: `$(symbol-ruler) Display: ${settings.verbose ? "verbose" : "compact"}`,
      description: settings.verbose
        ? "full dimension and scale breakdown"
        : "unit symbol only, e.g. Quantity<m, f64>",
      run: toggle("verbose", settings.verbose),
    },
    {
      label: `${check(settings.includeRaw)} Show the raw type in hovers too`,
      run: toggle("includeRaw", settings.includeRaw),
    },
    { label: "Rewrite in", kind: vscode.QuickPickItemKind.Separator },
    {
      label: `${check(settings.rewriteHover)} Hovers`,
      run: toggle("rewrite.hover", settings.rewriteHover),
    },
    {
      label: `${check(settings.rewriteInlayHints)} Inlay hints`,
      run: toggle("rewrite.inlayHints", settings.rewriteInlayHints),
    },
    {
      label: `${check(settings.rewriteDiagnostics)} Diagnostics`,
      run: toggle("rewrite.diagnostics", settings.rewriteDiagnostics),
    },
    {
      label: `${check(settings.unitMacroTextEdit)} Double-clicking a type hint inserts qty!(…)`,
      run: toggle("inlayHints.doubleClickInsertsUnitMacro", settings.unitMacroTextEdit),
    },
    { label: "", kind: vscode.QuickPickItemKind.Separator },
    {
      label: "$(debug-restart) Re-apply hooks",
      description: `${hook.state}: ${hook.detail}`,
      run: () => {
        hook.ensureHooked();
        hook.refresh();
      },
    },
    {
      label: "$(refresh) Restart rust-analyzer",
      run: () => vscode.commands.executeCommand("rust-analyzer.restartServer"),
    },
    { label: "$(output) Show log", run: () => log.show() },
    {
      label: "$(settings-gear) Open settings",
      run: () =>
        vscode.commands.executeCommand(
          "workbench.action.openSettings",
          "@ext:whippyunits.whippyunits-vscode",
        ),
    },
  ];

  const picked = await vscode.window.showQuickPick(items, {
    title: "whippyunits",
    placeHolder: "Toggle a setting or pick an action",
  });
  await picked?.run?.();
}
