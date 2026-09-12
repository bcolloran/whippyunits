import * as vscode from "vscode";
import { readSettings, SECTION, updateSetting } from "./config";
import { Formatter } from "./formatter";
import { showMenu } from "./menu";
import { RustAnalyzerHook } from "./raHook";
import { Rewriter } from "./rewrite";
import { StatusBar } from "./statusBar";

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const log = vscode.window.createOutputChannel("whippyunits", { log: true });
  const status = new StatusBar();
  context.subscriptions.push(log, status);

  let settings = readSettings();
  let hook: RustAnalyzerHook | undefined;

  context.subscriptions.push(
    vscode.commands.registerCommand("whippyunits.showLog", () => log.show()),
    vscode.commands.registerCommand("whippyunits.showMenu", () =>
      hook ? showMenu(hook, log) : log.show(),
    ),
    vscode.commands.registerCommand("whippyunits.toggle", () =>
      updateSetting("enabled", !readSettings().enabled),
    ),
    vscode.commands.registerCommand("whippyunits.rehook", () => {
      hook?.ensureHooked();
      hook?.refresh();
    }),
  );

  let formatter: Formatter;
  try {
    formatter = Formatter.load(context.extensionPath, settings);
  } catch (err) {
    const detail = `Could not load the type formatter (wasm): ${String(err)}`;
    log.error(detail);
    status.update({ state: "error", detail, settings });
    return;
  }
  context.subscriptions.push({ dispose: () => formatter.dispose() });

  const rewriter = new Rewriter(formatter, settings);
  hook = new RustAnalyzerHook(rewriter, log);
  const activeHook = hook;
  context.subscriptions.push(activeHook);

  const render = () =>
    status.update({ state: activeHook.state, detail: activeHook.detail, settings });
  context.subscriptions.push(activeHook.onDidChangeState(render));

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration(SECTION)) {
        return;
      }
      settings = readSettings();
      rewriter.settings = settings;
      formatter.configure(settings);
      render();
      // Re-publish diagnostics and re-request inlay hints with the new settings
      activeHook.refresh();
    }),
  );

  render();
  await activeHook.start();
}

// context.subscriptions disposes the hook, which restores rust-analyzer's original middleware
export function deactivate(): void {}
