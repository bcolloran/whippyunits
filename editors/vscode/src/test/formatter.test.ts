import "./setup";
import assert from "node:assert/strict";
import { test } from "node:test";
import { Formatter } from "../formatter";
import { DEFAULT_SETTINGS, EXTENSION_ROOT, FIELD_HOVER, HOVER_ENERGY, RAW_ENERGY, RAW_LENGTH } from "./fixtures";

test("loads the wasm build and formats a hover like the proxy does", () => {
  const formatter = Formatter.load(EXTENSION_ROOT, DEFAULT_SETTINGS);
  assert.ok(formatter.containsQuantity(HOVER_ENERGY));
  assert.ok(!formatter.containsQuantity("```rust\nlet x: String\n```"));

  const out = formatter.formatHover(HOVER_ENERGY);
  assert.match(out, /Quantity<J, f64>/);
  assert.doesNotMatch(out, /_A<0>/);
});

test("display settings change the output", () => {
  const formatter = Formatter.load(EXTENSION_ROOT, DEFAULT_SETTINGS);
  const compact = formatter.formatHover(HOVER_ENERGY);

  formatter.configure({ ...DEFAULT_SETTINGS, includeRaw: true });
  const withRaw = formatter.formatHover(HOVER_ENERGY);
  assert.match(withRaw, /Raw:/);
  assert.ok(withRaw.includes(RAW_ENERGY), "raw section keeps the original type");

  formatter.configure({ ...DEFAULT_SETTINGS, verbose: true });
  const verbose = formatter.formatHover(HOVER_ENERGY);
  assert.notEqual(verbose, compact);
  assert.doesNotMatch(verbose, /_A<0>/);
});

test("inlay types and their qty! annotations", () => {
  const formatter = Formatter.load(EXTENSION_ROOT, DEFAULT_SETTINGS);
  assert.equal(formatter.formatInlayType(RAW_LENGTH), "Quantity<m, f64>");
  assert.equal(formatter.unitMacroAnnotation("Quantity<m, f64>"), ": qty!(m)");
  assert.equal(formatter.unitMacroAnnotation("Quantity<mm, i32>"), ": qty!(mm, i32)");
});

test("formatText rewrites every type in a diagnostic message", () => {
  const formatter = Formatter.load(EXTENSION_ROOT, DEFAULT_SETTINGS);
  const out = formatter.formatText(`mismatched types: expected ${RAW_LENGTH}, found ${RAW_ENERGY}`);
  assert.match(out, /expected Quantity<m, f64>, found Quantity<J, f64>/);
});

test("unitLabel renders a unit expression the way the formatter does", () => {
  const formatter = Formatter.load(EXTENSION_ROOT, DEFAULT_SETTINGS);
  assert.equal(formatter.unitLabel("m / s"), "(m·s⁻¹)");
  assert.equal(formatter.unitLabel("kg * m / s ^ 2"), "N");
  assert.equal(formatter.unitLabel("m"), "m");
  assert.equal(formatter.unitLabel("not a unit expression ((("), undefined);
});

test("formatHover sees through the qty! lift-trace wrapper", () => {
  const formatter = Formatter.load(EXTENSION_ROOT, DEFAULT_SETTINGS);
  const out = formatter.formatHover(FIELD_HOVER);
  assert.doesNotMatch(out, /Helper|GetSecondGeneric/);
  assert.equal(
    out,
    "```rust\ndeclarators::TestNoGeneric\n```\n\n```rust\npub vel32: Quantity<(m·sˀ), f32>\n```\n\n---\nsize = 4, align = 0x4, offset = 0xC, no Drop",
  );
});
