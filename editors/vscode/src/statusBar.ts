import * as vscode from "vscode";
import type { Settings } from "./config";
import type { HookState } from "./raHook";

export interface StatusInfo {
  state: HookState;
  detail: string;
  settings: Settings;
}

/** The `$(law) whippyunits` item at the right of the status bar; clicking it opens the menu. */
export class StatusBar implements vscode.Disposable {
  private readonly item: vscode.StatusBarItem;

  constructor() {
    this.item = vscode.window.createStatusBarItem(
      "whippyunits.status",
      vscode.StatusBarAlignment.Right,
      90,
    );
    this.item.name = "whippyunits";
    this.item.command = "whippyunits.showMenu";
    this.item.show();
  }

  update({ state, detail, settings }: StatusInfo): void {
    const tooltip = new vscode.MarkdownString(undefined, true);
    tooltip.appendMarkdown("**whippyunits**\n\n");
    this.item.backgroundColor = undefined;

    switch (state) {
      case "hooked":
        if (settings.enabled) {
          this.item.text = "$(law) whippyunits";
          tooltip.appendMarkdown(
            `Rewriting ${describeTargets(settings)} in ${settings.verbose ? "verbose" : "compact"} mode.`,
          );
        } else {
          this.item.text = "$(law) whippyunits $(circle-slash)";
          tooltip.appendMarkdown("Rewriting is off; rust-analyzer's output is shown as-is.");
        }
        break;
      case "waiting":
        this.item.text = "$(law) whippyunits $(clock)";
        tooltip.appendMarkdown(`Waiting for rust-analyzer: ${detail}`);
        break;
      case "no-rust-analyzer":
      case "error":
        this.item.text = "$(law) whippyunits $(warning)";
        this.item.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
        tooltip.appendMarkdown(detail);
        break;
    }

    tooltip.appendMarkdown("\n\nClick for options.");
    this.item.tooltip = tooltip;
  }

  dispose(): void {
    this.item.dispose();
  }
}

function describeTargets(settings: Settings): string {
  const targets = [
    settings.rewriteHover && "hovers",
    settings.rewriteInlayHints && "inlay hints",
    settings.rewriteDiagnostics && "diagnostics",
  ].filter((target): target is string => typeof target === "string");
  return targets.length > 0 ? targets.join(", ") : "nothing (every target is switched off)";
}
