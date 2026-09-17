import * as vscode from "vscode";
import type { Settings } from "./config";
import type { Formatter } from "./formatter";

/** What the formatter prints for an exponent it could not read (e.g. rust-analyzer's `{const}`). */
const UNKNOWN_EXPONENT = "ˀ";

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

  /**
   * `document`/`position` are where the hover was requested. They let a hover
   * on a struct field recover from rust-analyzer showing the *declared* type,
   * in which negative exponents appear only as `{const}`: the `qty!(…)` on that
   * source line still says what the type is.
   */
  hover<T extends vscode.Hover | null | undefined>(
    hover: T,
    document?: vscode.TextDocument,
    position?: vscode.Position,
  ): T {
    if (!hover || !this.settings.enabled || !this.settings.rewriteHover) {
      return hover;
    }
    hover.contents = hover.contents.map((item) => this.hoverItem(item));
    if (document && position) {
      this.recoverFromSource(hover, document.lineAt(position.line).text);
    }
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

  /** Replaces `Quantity<…>` types with an unknown exponent by the type the line's `qty!(…)` denotes. */
  private recoverFromSource(hover: vscode.Hover, line: string): void {
    const args = qtyMacroArgs(line);
    if (args === undefined) {
      return;
    }
    let pretty: string | undefined;
    for (const item of hover.contents) {
      if (typeof item === "string" || !item.value.includes(UNKNOWN_EXPONENT)) {
        continue;
      }
      pretty ??= this.formatter.formatQtyMacro(args);
      if (pretty === undefined) {
        return;
      }
      item.value = replaceUnresolvedQuantities(item.value, pretty);
    }
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

/** The text between the parentheses of the first `qty!(…)` on a line, if there is one. */
export function qtyMacroArgs(line: string): string | undefined {
  const match = /(?<![A-Za-z0-9_])qty!\s*\(/.exec(line);
  if (!match) {
    return undefined;
  }
  const start = match.index + match[0].length;
  let depth = 1;
  for (let i = start; i < line.length; i++) {
    if (line[i] === "(") {
      depth++;
    } else if (line[i] === ")" && --depth === 0) {
      return line.slice(start, i);
    }
  }
  return undefined;
}

/** Replaces every `Quantity<…>` whose text contains an unknown exponent with `pretty`. */
export function replaceUnresolvedQuantities(text: string, pretty: string): string {
  let result = "";
  let from = 0;
  for (;;) {
    const start = text.indexOf("Quantity<", from);
    if (start < 0) {
      return result + text.slice(from);
    }
    let depth = 0;
    let end = -1;
    for (let i = start + "Quantity".length; i < text.length; i++) {
      if (text[i] === "<") {
        depth++;
      } else if (text[i] === ">" && --depth === 0) {
        end = i + 1;
        break;
      }
    }
    if (end < 0) {
      return result + text.slice(from);
    }
    const span = text.slice(start, end);
    result += text.slice(from, start) + (span.includes(UNKNOWN_EXPONENT) ? pretty : span);
    from = end;
  }
}
