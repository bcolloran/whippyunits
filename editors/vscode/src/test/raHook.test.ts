import "./setup";
import assert from "node:assert/strict";
import { test } from "node:test";
import * as vscode from "vscode";
import { Formatter } from "../formatter";
import { RUST_ANALYZER_ID, RustAnalyzerHook } from "../raHook";
import { Rewriter } from "../rewrite";
import { DEFAULT_SETTINGS, EXTENSION_ROOT, HOVER_ENERGY, RAW_LENGTH, fakeDocument, fakeLog, tick } from "./fixtures";
import { __setExtension } from "./vscodeStub";

// Loose types: these fakes only need the shape the hook touches
/* eslint-disable @typescript-eslint/no-explicit-any */

/** A fake vscode-languageclient client, as rust-analyzer's `Ctx.client` exposes it. */
function fakeClient() {
  const stateEmitter = new vscode.EventEmitter<{ oldState: number; newState: number }>();
  const provider = { refreshes: 0, onDidChangeInlayHints: { fire: () => void provider.refreshes++ } };
  const originalHover = async () => new vscode.Hover([new vscode.MarkdownString(HOVER_ENERGY)]);
  const originalDiagnostics = (uri: any, diagnostics: any, next: any) => next(uri, diagnostics);
  const middleware: any = { provideHover: originalHover, handleDiagnostics: originalDiagnostics };
  return {
    middleware,
    onDidChangeState: stateEmitter.event,
    getFeature: () => ({ getAllProviders: () => [provider] }),
    stateEmitter,
    provider,
    originalHover,
    originalDiagnostics,
  };
}

function setup(t: { after(fn: () => void): void }) {
  const holder: { current: any } = { current: fakeClient() };
  // Like rust-analyzer's Ctx: a getter that follows restarts
  const api = {
    get client() {
      return holder.current;
    },
  };
  __setExtension(RUST_ANALYZER_ID, { activate: async () => api });
  const rewriter = new Rewriter(Formatter.load(EXTENSION_ROOT, DEFAULT_SETTINGS), { ...DEFAULT_SETTINGS });
  const hook = new RustAnalyzerHook(rewriter, fakeLog);
  t.after(() => hook.dispose());
  return { holder, hook, rewriter };
}

test("hooks the live client, rewrites hovers, re-hooks after a restart, restores on dispose", async (t) => {
  const { holder, hook } = setup(t);
  const first = holder.current;

  await hook.start();

  assert.equal(hook.state, "hooked");
  assert.notEqual(first.middleware.provideHover, first.originalHover);
  assert.equal(first.provider.refreshes, 1, "asks for fresh inlay hints once hooked");
  const hover = await first.middleware.provideHover(fakeDocument("let x = 1;"), new vscode.Position(0, 0), {}, () => null);
  assert.match(hover.contents[0].value, /Quantity<J, f64>/);

  // Restart: rust-analyzer disposes the client (api.client becomes undefined) and later creates a new one
  holder.current = undefined;
  first.stateEmitter.fire({ oldState: 2, newState: 1 });
  await tick(20);
  assert.equal(hook.state, "waiting");
  assert.equal(first.middleware.provideHover, first.originalHover, "old client's middleware restored");

  const second = fakeClient();
  holder.current = second;
  await tick(600); // the retry scheduled on the state change
  assert.equal(hook.state, "hooked");
  assert.notEqual(second.middleware.provideHover, second.originalHover);

  hook.dispose();
  assert.equal(second.middleware.provideHover, second.originalHover);
  assert.equal(second.middleware.handleDiagnostics, second.originalDiagnostics);
});

test("diagnostics flow through rust-analyzer's handler and are re-published on refresh", async (t) => {
  const { holder, hook, rewriter } = setup(t);
  const client = holder.current;
  await hook.start();

  const published: vscode.Diagnostic[][] = [];
  const next = (_uri: unknown, diagnostics: vscode.Diagnostic[]) => void published.push(diagnostics);
  const uri = vscode.Uri.file("/src/main.rs");
  const diagnostic = new vscode.Diagnostic(
    new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 1)),
    `expected ${RAW_LENGTH}`,
  );

  client.middleware.handleDiagnostics(uri, [diagnostic], next);
  assert.equal(published.length, 1);
  assert.match(published[0][0].message, /expected Quantity<m, f64>/);

  // Turning rewriting off re-publishes the originals without a new message from the server
  rewriter.settings = { ...DEFAULT_SETTINGS, enabled: false };
  hook.refresh();
  assert.equal(published.length, 2);
  assert.equal(published[1][0], diagnostic);
});

test("reports a missing rust-analyzer extension", async (t) => {
  const { hook } = setup(t);
  __setExtension(RUST_ANALYZER_ID, undefined);

  await hook.start();

  assert.equal(hook.state, "no-rust-analyzer");
});

test("reports a rust-analyzer API without a client", async (t) => {
  const { hook } = setup(t);
  __setExtension(RUST_ANALYZER_ID, { activate: async () => ({}) });

  await hook.start();

  assert.equal(hook.state, "error");
  assert.match(hook.detail, /not supported/);
});
