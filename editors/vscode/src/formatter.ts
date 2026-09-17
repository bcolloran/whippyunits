import * as path from "path";
import type { Settings } from "./config";

/** The module wasm-pack (`--target nodejs`) generates from formatter-wasm/. */
interface WasmModule {
  Formatter: new (verbose: boolean, includeRaw: boolean) => WasmFormatter;
  contains_quantity(text: string): boolean;
}

interface WasmFormatter {
  format_hover(markdown: string): string;
  format_text(text: string): string;
  format_inlay_type(typeText: string): string;
  unit_macro_annotation(prettyType: string): string;
  format_qty_macro(args: string): string | undefined;
  free(): void;
}

/**
 * Text-level formatting, backed by the same Rust code as the `lsp-proxy`
 * binary so hovers look identical whichever integration is used.
 */
export class Formatter {
  private inner: WasmFormatter;

  private constructor(
    private readonly wasm: WasmModule,
    settings: Settings,
  ) {
    this.inner = new wasm.Formatter(settings.verbose, settings.includeRaw);
  }

  /** Loads `<extension>/wasm/whippyunits_formatter.js`; throws if the build is missing or broken. */
  static load(extensionPath: string, settings: Settings): Formatter {
    const modulePath = path.join(extensionPath, "wasm", "whippyunits_formatter.js");
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const wasm = require(modulePath) as WasmModule;
    return new Formatter(wasm, settings);
  }

  /** Rebuilds the Rust-side formatter with new display settings. */
  configure(settings: Settings): void {
    this.inner.free();
    this.inner = new this.wasm.Formatter(settings.verbose, settings.includeRaw);
  }

  dispose(): void {
    this.inner.free();
  }

  containsQuantity(text: string): boolean {
    return this.wasm.contains_quantity(text);
  }

  formatHover(markdown: string): string {
    return this.inner.format_hover(markdown);
  }

  formatText(text: string): string {
    return this.inner.format_text(text);
  }

  /** Rewrites every whippyunits type inside inlay-hint type text (label without its `: `). */
  formatInlayType(typeText: string): string {
    return this.inner.format_inlay_type(typeText);
  }

  unitMacroAnnotation(prettyType: string): string {
    return this.inner.unit_macro_annotation(prettyType);
  }

  /** The pretty type denoted by the arguments of a `qty!(…)` invocation, if they parse. */
  formatQtyMacro(args: string): string | undefined {
    return this.inner.format_qty_macro(args);
  }
}
