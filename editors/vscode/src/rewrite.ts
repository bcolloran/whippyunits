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

  /**
   * Replaces every unit label with an unknown exponent (`(m·sˀ)`) by the label the
   * line's `qty!(…)`/`vqty!(…)` denotes. One macro per line is assumed: every
   * unknown label in the hover — the field type, a matrix's entry unit, the
   * unit-matrix trace — comes from that one unit expression.
   */
  private recoverFromSource(hover: vscode.Hover, line: string): void {
    const unitExpr = macroUnitExpr(line);
    if (unitExpr === undefined) {
      return;
    }
    let label: string | undefined;
    for (const item of hover.contents) {
      if (typeof item === "string" || !item.value.includes(UNKNOWN_EXPONENT)) {
        continue;
      }
      label ??= this.formatter.unitLabel(unitExpr);
      if (label === undefined) {
        return;
      }
      item.value = replaceUnknownUnitLabels(item.value, label);
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

/**
 * The unit expression of the first `qty!(…)` or `vqty!(…)` on a line, if any:
 * `qty!(unit[, storage[, brand]])` and `vqty!(count, unit[, storage])`. A `vqty!`
 * with a bracketed per-entry unit list has no single unit and yields nothing.
 */
export function macroUnitExpr(line: string): string | undefined {
  const match = /(?<![A-Za-z0-9_])(v?qty)!\s*\(/.exec(line);
  if (!match) {
    return undefined;
  }
  const start = match.index + match[0].length;
  let depth = 1;
  let end = -1;
  for (let i = start; i < line.length; i++) {
    if (line[i] === "(") {
      depth++;
    } else if (line[i] === ")" && --depth === 0) {
      end = i;
      break;
    }
  }
  if (end < 0) {
    return undefined;
  }
  // Unit expressions never contain commas, so top-level commas separate the arguments
  const args = line.slice(start, end).split(",");
  const unit = (match[1] === "vqty" ? args[1] : args[0])?.trim();
  return unit && !unit.startsWith("[") ? unit : undefined;
}

/** Replaces every unit label containing an unknown exponent — `(m·sˀ)`, `sˀ` — with `label`. */
export function replaceUnknownUnitLabels(text: string, label: string): string {
  return text.replace(/\([^()]*ˀ[^()]*\)|[^\s<>,()[\]]*ˀ[^\s<>,()[\]]*/g, label);
}
