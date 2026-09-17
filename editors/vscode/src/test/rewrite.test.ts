import "./setup";
import assert from "node:assert/strict";
import { test } from "node:test";
import * as vscode from "vscode";
import { Formatter } from "../formatter";
import { Rewriter, qtyMacroArgs, replaceUnresolvedQuantities } from "../rewrite";
import { DEFAULT_SETTINGS, EXTENSION_ROOT, FIELD_HOVER, FIELD_LINE, HOVER_ENERGY, RAW_LENGTH, fakeDocument } from "./fixtures";

const formatter = Formatter.load(EXTENSION_ROOT, DEFAULT_SETTINGS);

function range(): vscode.Range {
  return new vscode.Range(new vscode.Position(1, 0), new vscode.Position(1, 8));
}

/** A type hint the way rust-analyzer splits it: punctuation and named types are separate parts. */
function splitLabel(): { parts: vscode.InlayHintLabelPart[]; location: vscode.Location } {
  const [, generics] = RAW_LENGTH.split(/^Quantity/);
  const location = new vscode.Location(vscode.Uri.file("/src/lib.rs"), range());
  const quantity = new vscode.InlayHintLabelPart("Quantity");
  quantity.location = location;
  return {
    parts: [new vscode.InlayHintLabelPart(": "), quantity, new vscode.InlayHintLabelPart(generics)],
    location,
  };
}

test("hover: rewrites MarkdownString values in place, keeping their flags", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  const markdown = new vscode.MarkdownString(HOVER_ENERGY);
  markdown.isTrusted = true;
  const hover = rewriter.hover(new vscode.Hover([markdown]));

  assert.equal(hover.contents[0], markdown);
  assert.match(markdown.value, /Quantity<J, f64>/);
  assert.equal(markdown.isTrusted, true);
});

test("hover: untouched when rewriting is off", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS, enabled: false });
  const markdown = new vscode.MarkdownString(HOVER_ENERGY);
  rewriter.hover(new vscode.Hover([markdown]));
  assert.equal(markdown.value, HOVER_ENERGY);
});

test("inlay hints: collapses the split parts into ': ' plus one linked part", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  const { parts, location } = splitLabel();
  const hint = new vscode.InlayHint(new vscode.Position(3, 10), parts);

  rewriter.inlayHints([hint]);

  const label = hint.label as vscode.InlayHintLabelPart[];
  assert.equal(label.length, 2);
  assert.equal(label[0].value, ": ");
  assert.equal(label[1].value, "Quantity<m, f64>");
  assert.equal(label[1].location, location);
  assert.equal(hint.textEdits, undefined, "unresolved hints get no text edit");
});

test("inlay hints: a resolved hint gets a qty! text edit at its position", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  const position = new vscode.Position(3, 10);
  const hint = new vscode.InlayHint(position, splitLabel().parts);

  rewriter.resolvedInlayHint(hint);

  assert.equal(hint.textEdits?.length, 1);
  assert.equal(hint.textEdits?.[0].newText, ": qty!(m)");
  assert.equal(hint.textEdits?.[0].range.start, position);
});

test("inlay hints: string labels and non-whippyunits labels", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  const stringHint = new vscode.InlayHint(new vscode.Position(0, 0), `: ${RAW_LENGTH}`);
  const other = new vscode.InlayHint(new vscode.Position(0, 0), [new vscode.InlayHintLabelPart(": String")]);

  rewriter.inlayHints([stringHint, other]);

  assert.equal(stringHint.label, ": Quantity<m, f64>");
  assert.equal((other.label as vscode.InlayHintLabelPart[])[0].value, ": String");
});

test("inlay hints: a bare Unit<…> (whippyalgebra) is rewritten but gets no qty! edit", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  const [, inner] = RAW_LENGTH.split(/^Quantity<|, f64>$/);
  const hint = new vscode.InlayHint(new vscode.Position(0, 0), `: ${inner}`);

  rewriter.resolvedInlayHint(hint);

  assert.notEqual(hint.label, `: ${inner}`);
  assert.doesNotMatch(hint.label as string, /Scale</);
  assert.equal(hint.textEdits, undefined);
});

test("diagnostics: copies keep the prototype and extra fields; originals stay untouched", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  // vscode-languageclient publishes a Diagnostic subclass carrying protocol `data`
  class ProtocolDiagnostic extends vscode.Diagnostic {
    data = { rendered: "full rustc output" };
  }
  const message = `mismatched types: expected ${RAW_LENGTH}`;
  const original = new ProtocolDiagnostic(range(), message, 0);
  original.source = "rustc";
  const plain = new vscode.Diagnostic(range(), "unused variable", 1);

  const [rewritten, untouched] = rewriter.diagnostics([original, plain]);

  assert.notEqual(rewritten, original);
  assert.ok(rewritten instanceof ProtocolDiagnostic);
  assert.equal((rewritten as ProtocolDiagnostic).data.rendered, "full rustc output");
  assert.equal(rewritten.source, "rustc");
  assert.match(rewritten.message, /expected Quantity<m, f64>/);
  assert.equal(original.message, message);
  assert.equal(untouched, plain);
});

test("hover on a qty! field: the declared type's {const} exponent is recovered from the source line", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  const markdown = new vscode.MarkdownString(FIELD_HOVER);

  rewriter.hover(new vscode.Hover([markdown]), fakeDocument(FIELD_LINE), new vscode.Position(30, 8));

  assert.match(markdown.value, /```rust\npub vel32: Quantity<\(m·s⁻¹\), f32>\n```/);
  assert.doesNotMatch(markdown.value, /ˀ|Helper/);
});

test("hover on a qty! field without a document context keeps the honest unknown exponent", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  const markdown = new vscode.MarkdownString(FIELD_HOVER);

  rewriter.hover(new vscode.Hover([markdown]));

  assert.match(markdown.value, /```rust\npub vel32: Quantity<\(m·sˀ\), f32>\n```/);
});

test("hover: a line without qty! leaves an unknown exponent alone", () => {
  const rewriter = new Rewriter(formatter, { ...DEFAULT_SETTINGS });
  const markdown = new vscode.MarkdownString(FIELD_HOVER);

  rewriter.hover(new vscode.Hover([markdown]), fakeDocument("    pub vel32: Velocity<f32>,"), new vscode.Position(0, 0));

  assert.match(markdown.value, /m·sˀ/);
});

test("qtyMacroArgs and replaceUnresolvedQuantities", () => {
  assert.equal(qtyMacroArgs("    pub v: qty!(m / s, f32),"), "m / s, f32");
  assert.equal(qtyMacroArgs("let x: whippyunits::qty!( m ) = y;"), " m ");
  assert.equal(qtyMacroArgs("    pub v: vqty!(m / s, f32),"), undefined, "vqty! has a different argument shape");
  assert.equal(qtyMacroArgs("no macro here"), undefined);
  assert.equal(qtyMacroArgs("qty!(unbalanced"), undefined);
  assert.equal(
    replaceUnresolvedQuantities("a: Quantity<(m·sˀ), f32>, b: Quantity<m, f64>", "Quantity<(m·s⁻¹), f32>"),
    "a: Quantity<(m·s⁻¹), f32>, b: Quantity<m, f64>",
  );
});
