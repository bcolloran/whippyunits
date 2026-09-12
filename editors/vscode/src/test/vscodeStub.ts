/**
 * Minimal stand-in for the `vscode` module so the extension's logic can run
 * under plain node (`npm test`). Only what rewrite.ts and raHook.ts touch.
 */

export class Position {
  constructor(
    public line: number,
    public character: number,
  ) {}
}

export class Range {
  constructor(
    public start: Position,
    public end: Position,
  ) {}
}

export class Uri {
  private constructor(public path: string) {}
  static file(path: string): Uri {
    return new Uri(path);
  }
  toString(): string {
    return `file://${this.path}`;
  }
}

export class Location {
  constructor(
    public uri: Uri,
    public range: Range,
  ) {}
}

export class MarkdownString {
  isTrusted?: boolean;
  constructor(
    public value = "",
    public supportThemeIcons = false,
  ) {}
  appendMarkdown(text: string): this {
    this.value += text;
    return this;
  }
}

export class Hover {
  constructor(
    public contents: unknown[],
    public range?: Range,
  ) {}
}

export class InlayHintLabelPart {
  location?: Location;
  tooltip?: unknown;
  command?: unknown;
  constructor(public value: string) {}
}

export class InlayHint {
  tooltip?: unknown;
  textEdits?: TextEdit[];
  constructor(
    public position: Position,
    public label: string | InlayHintLabelPart[],
    public kind?: number,
  ) {}
}

export class TextEdit {
  constructor(
    public range: Range,
    public newText: string,
  ) {}
}

export class Diagnostic {
  source?: string;
  code?: unknown;
  tags?: unknown;
  relatedInformation?: DiagnosticRelatedInformation[];
  constructor(
    public range: Range,
    public message: string,
    public severity = 0,
  ) {}
}

export class DiagnosticRelatedInformation {
  constructor(
    public location: Location,
    public message: string,
  ) {}
}

export class EventEmitter<T> {
  private readonly listeners = new Set<(event: T) => void>();
  readonly event = (listener: (event: T) => void) => {
    this.listeners.add(listener);
    return { dispose: () => this.listeners.delete(listener) };
  };
  fire(event: T): void {
    for (const listener of [...this.listeners]) {
      listener(event);
    }
  }
  dispose(): void {
    this.listeners.clear();
  }
}

export class ThemeColor {
  constructor(public id: string) {}
}

export const StatusBarAlignment = { Left: 1, Right: 2 };
export const ConfigurationTarget = { Global: 1, Workspace: 2, WorkspaceFolder: 3 };
export const QuickPickItemKind = { Separator: -1, Default: 0 };

const registeredExtensions = new Map<string, unknown>();

export const extensions = {
  getExtension: (id: string) => registeredExtensions.get(id),
};

/** Test hook: what `vscode.extensions.getExtension(id)` returns. */
export function __setExtension(id: string, extension: unknown): void {
  if (extension === undefined) {
    registeredExtensions.delete(id);
  } else {
    registeredExtensions.set(id, extension);
  }
}
