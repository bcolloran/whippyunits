import * as vscode from "vscode";
import type { Settings } from "./config";
import type { Formatter } from "./formatter";

/**
 * Rewrites the VS Code objects rust-analyzer's middleware hands back.
 *
 * Every method is a no-op when the relevant setting is off, so the hooks can
 * stay installed while the user toggles rewriting on and off.
 */
export class Rewriter {
  constructor(
    private readonly formatter: Formatter,
    public settings: Settings,
  ) {}

  hover<T extends vscode.Hover | null | undefined>(hover: T): T {
    if (!hover || !this.settings.enabled || !this.settings.rewriteHover) {
      return hover;
    }
    hover.contents = hover.contents.map((item) => this.hoverItem(item));
    return hover;
  }

  private hoverItem(
    item: vscode.MarkdownString | vscode.MarkedString,
  ): vscode.MarkdownString | vscode.MarkedString {
    if (typeof item === "string") {
      return this.formatter.containsQuantity(item) ? this.formatter.formatHover(item) : item;
    }
    if ("language" in item) {
      // Deprecated `{ language, value }` form: a bare code block, no markdown fences
      return this.formatter.containsQuantity(item.value)
        ? { language: item.language, value: this.formatter.formatText(item.value) }
        : item;
    }
    // Mutate in place to keep isTrusted / supportHtml / supportThemeIcons
    if (this.formatter.containsQuantity(item.value)) {
      item.value = this.formatter.formatHover(item.value);
    }
    return item;
  }

  inlayHints<T extends vscode.InlayHint[] | null | undefined>(hints: T): T {
    if (!hints || !this.settings.enabled || !this.settings.rewriteInlayHints) {
      return hints;
    }
    for (const hint of hints) {
      this.inlayHint(hint, false);
    }
    return hints;
  }

  resolvedInlayHint<T extends vscode.InlayHint | null | undefined>(hint: T): T {
    if (!hint || !this.settings.enabled || !this.settings.rewriteInlayHints) {
      return hint;
    }
    this.inlayHint(hint, true);
    return hint;
  }

  /**
   * Rewrites a hint's label the way the lsp-proxy does: the whole label is
   * formatted as one piece of text and re-emitted as a leading `: ` part (when
   * present) plus a single type part. That handles a bare `Quantity`, a bare
   * `Unit<Scale…>` and composites like whippyalgebra's `MixedUnitMatrix<…>`
   * uniformly. Returns whether the hint was changed.
   */
  inlayHint(hint: vscode.InlayHint, resolved: boolean): boolean {
    const label = hint.label;
    const full = typeof label === "string" ? label : label.map((part) => part.value).join("");
    if (!this.formatter.containsQuantity(full)) {
      return false;
    }
    const prefix = full.startsWith(": ") ? ": " : "";
    const typeText = full.slice(prefix.length);
    const prettyType = this.formatter.formatInlayType(typeText);
    if (prettyType === typeText) {
      return false;
    }

    if (typeof label === "string") {
      hint.label = prefix + prettyType;
    } else {
      // Keep the first available link so the pretty label still navigates to a definition
      const located = label.find((part) => part.location !== undefined);
      const part = new vscode.InlayHintLabelPart(prettyType);
      part.location = located?.location;
      part.tooltip = located?.tooltip;
      part.command = located?.command;
      hint.label = prefix ? [new vscode.InlayHintLabelPart(prefix), part] : [part];
    }

    // Double-clicking a hint applies its textEdits; rust-analyzer's would insert the raw
    // type. The qty! completion only makes sense for a plain quantity, not e.g. a matrix.
    if (
      this.settings.unitMacroTextEdit &&
      (resolved || hint.textEdits) &&
      typeText.startsWith("Quantity<")
    ) {
      const range = hint.textEdits?.[0]?.range ?? new vscode.Range(hint.position, hint.position);
      hint.textEdits = [new vscode.TextEdit(range, this.formatter.unitMacroAnnotation(prettyType))];
    }
    return true;
  }

  diagnostics(diagnostics: vscode.Diagnostic[]): vscode.Diagnostic[] {
    if (!this.settings.enabled || !this.settings.rewriteDiagnostics) {
      return diagnostics;
    }
    return diagnostics.map((diagnostic) => this.diagnostic(diagnostic));
  }

  private diagnostic(diagnostic: vscode.Diagnostic): vscode.Diagnostic {
    const message = this.text(diagnostic.message);
    let relatedChanged = false;
    const relatedInformation = diagnostic.relatedInformation?.map((info) => {
      const text = this.text(info.message);
      if (text === info.message) {
        return info;
      }
      relatedChanged = true;
      return new vscode.DiagnosticRelatedInformation(info.location, text);
    });
    if (message === diagnostic.message && !relatedChanged) {
      return diagnostic;
    }
    // Copy rather than mutate so the originals can be re-published when settings change.
    // A prototype-preserving copy keeps vscode-languageclient's extra fields (e.g. `data`),
    // which it round-trips into code action requests.
    const copy: vscode.Diagnostic = Object.assign(
      Object.create(Object.getPrototypeOf(diagnostic)),
      diagnostic,
    );
    copy.message = message;
    copy.relatedInformation = relatedInformation;
    return copy;
  }

  private text(text: string): string {
    return this.formatter.containsQuantity(text) ? this.formatter.formatText(text) : text;
  }
}
