import * as vscode from "vscode";
import type * as lc from "vscode-languageclient/node";
import type { Rewriter } from "./rewrite";

/**
 * What rust-analyzer's `activate()` returns — see `RustAnalyzerExtensionApi`
 * in rust-analyzer's editors/code/src/main.ts. `client` is a getter that
 * follows server restarts (each restart creates a new client).
 */
export interface RustAnalyzerApi {
  readonly client?: lc.LanguageClient;
}

export type HookState = "no-rust-analyzer" | "waiting" | "hooked" | "error";

export const RUST_ANALYZER_ID = "rust-lang.rust-analyzer";
const INLAY_HINT_METHOD = "textDocument/inlayHint";
/** How often to re-check that the current client is the one we patched. Cheap: two property reads. */
const POLL_MS = 2000;
/** vscode-languageclient's `State` enum, for logs (the import above is type-only). */
const STATE_NAMES: Record<number, string> = { 1: "Stopped", 2: "Running", 3: "Starting", 4: "StartFailed" };

type HookedMiddleware = Pick<
  lc.Middleware,
  "provideHover" | "provideInlayHints" | "resolveInlayHint" | "handleDiagnostics"
>;

interface Patch {
  client: lc.LanguageClient;
  middleware: lc.Middleware;
  originals: HookedMiddleware;
  stateListener: vscode.Disposable;
}

interface Published {
  uri: vscode.Uri;
  diagnostics: vscode.Diagnostic[];
  next: lc.HandleDiagnosticsSignature;
}

/**
 * Wraps the middleware of rust-analyzer's live language client.
 *
 * rust-analyzer passes an object literal as `middleware`, and
 * vscode-languageclient reads `client.middleware.<hook>` on every request, so
 * replacing the properties takes effect immediately and needs no restart.
 */
export class RustAnalyzerHook implements vscode.Disposable {
  private api?: RustAnalyzerApi;
  private patch?: Patch;
  private poll?: ReturnType<typeof setInterval>;
  /** Last diagnostics per file as rust-analyzer published them, for re-publishing after a settings change. */
  private readonly published = new Map<string, Published>();
  private readonly stateEmitter = new vscode.EventEmitter<void>();

  state: HookState = "waiting";
  detail = "";
  readonly onDidChangeState = this.stateEmitter.event;

  constructor(
    private readonly rewriter: Rewriter,
    private readonly log: vscode.LogOutputChannel,
  ) {}

  async start(): Promise<void> {
    const extension = vscode.extensions.getExtension<RustAnalyzerApi>(RUST_ANALYZER_ID);
    if (!extension) {
      this.setState("no-rust-analyzer", "The rust-analyzer extension is not installed.");
      return;
    }
    try {
      this.api = await extension.activate();
    } catch (err) {
      this.setState("error", `rust-analyzer failed to activate: ${String(err)}`);
      return;
    }
    if (typeof this.api !== "object" || this.api === null || !("client" in this.api)) {
      this.setState(
        "error",
        "rust-analyzer's extension API does not expose its language client; this rust-analyzer version is not supported.",
      );
      return;
    }
    this.ensureHooked();
    this.poll = setInterval(() => this.ensureHooked(), POLL_MS);
  }

  /** Idempotent: make sure the client rust-analyzer is using right now carries our middleware. */
  ensureHooked(): void {
    const client = this.api?.client;
    if (this.patch && this.patch.client !== client) {
      // The server was stopped or restarted
      this.unpatch();
    }
    if (!client) {
      this.setState("waiting", "rust-analyzer's server is not running.");
      return;
    }
    if (!this.patch) {
      this.apply(client);
    }
  }

  /** Re-publishes known diagnostics and re-requests inlay hints, e.g. after a settings change. */
  refresh(): void {
    for (const { uri, diagnostics, next } of this.published.values()) {
      next(uri, this.rewriter.diagnostics(diagnostics));
    }
    if (this.patch) {
      this.refreshInlayHints(this.patch.client);
    }
  }

  dispose(): void {
    if (this.poll) {
      clearInterval(this.poll);
    }
    this.unpatch();
    this.stateEmitter.dispose();
  }

  private apply(client: lc.LanguageClient): void {
    const middleware = client.middleware;
    const originals: HookedMiddleware = {
      provideHover: middleware.provideHover,
      provideInlayHints: middleware.provideInlayHints,
      resolveInlayHint: middleware.resolveInlayHint,
      handleDiagnostics: middleware.handleDiagnostics,
    };
    const rewriter = this.rewriter;

    middleware.provideHover = async (document, position, token, next) => {
      const hover = await (originals.provideHover
        ? originals.provideHover(document, position, token, next)
        : next(document, position, token));
      return rewriter.hover(hover);
    };

    middleware.provideInlayHints = async (document, range, token, next) => {
      const hints = await (originals.provideInlayHints
        ? originals.provideInlayHints(document, range, token, next)
        : next(document, range, token));
      return rewriter.inlayHints(hints);
    };

    middleware.resolveInlayHint = async (hint, token, next) => {
      const resolved = await (originals.resolveInlayHint
        ? originals.resolveInlayHint(hint, token, next)
        : next(hint, token));
      return rewriter.resolvedInlayHint(resolved);
    };

    middleware.handleDiagnostics = (uri, diagnostics, next) => {
      // rust-analyzer's own handler massages the diagnostics and then calls `next`; we slip in before that call
      const publish: lc.HandleDiagnosticsSignature = (uri, diagnostics) => {
        this.published.set(uri.toString(), { uri, diagnostics, next });
        next(uri, rewriter.diagnostics(diagnostics));
      };
      if (originals.handleDiagnostics) {
        originals.handleDiagnostics(uri, diagnostics, publish);
      } else {
        publish(uri, diagnostics);
      }
    };

    const stateListener = client.onDidChangeState((event) => {
      this.log.debug(
        `rust-analyzer client: ${STATE_NAMES[event.oldState] ?? event.oldState} -> ${STATE_NAMES[event.newState] ?? event.newState}`,
      );
      // On a restart rust-analyzer creates a new client; give it a moment to appear, then re-hook
      setTimeout(() => this.ensureHooked(), 0);
      setTimeout(() => this.ensureHooked(), 500);
    });

    this.patch = { client, middleware, originals, stateListener };
    this.setState("hooked", "Hooked into rust-analyzer's language client.");
    // Hints requested between the server starting and our hook landing were shown raw; ask for them again
    this.refreshInlayHints(client);
  }

  private unpatch(): void {
    if (!this.patch) {
      return;
    }
    const { middleware, originals, stateListener } = this.patch;
    middleware.provideHover = originals.provideHover;
    middleware.provideInlayHints = originals.provideInlayHints;
    middleware.resolveInlayHint = originals.resolveInlayHint;
    middleware.handleDiagnostics = originals.handleDiagnostics;
    stateListener.dispose();
    this.published.clear();
    this.patch = undefined;
    this.log.info("Restored rust-analyzer's original middleware.");
  }

  private refreshInlayHints(client: lc.LanguageClient): void {
    try {
      // The inlay-hint feature keeps one provider per document selector, each with the
      // EventEmitter VS Code listens to. Firing it is what the server's
      // `workspace/inlayHint/refresh` request does too. Best effort: this reaches into
      // vscode-languageclient internals, so failure is logged and otherwise ignored.
      const feature = client.getFeature(INLAY_HINT_METHOD) as unknown as
        | { getAllProviders?(): Iterable<{ onDidChangeInlayHints?: vscode.EventEmitter<void> }> }
        | undefined;
      for (const provider of feature?.getAllProviders?.() ?? []) {
        provider.onDidChangeInlayHints?.fire();
      }
    } catch (err) {
      this.log.debug(`Could not ask for an inlay hint refresh: ${String(err)}`);
    }
  }

  private setState(state: HookState, detail: string): void {
    if (state === this.state && detail === this.detail) {
      return;
    }
    this.state = state;
    this.detail = detail;
    this.log.info(`${state}: ${detail}`);
    this.stateEmitter.fire();
  }
}
