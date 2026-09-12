/**
 * Routes `require("vscode")` to the stub. Import this first in every test file:
 * imports are hoisted in order, so it runs before the modules under test load.
 */
import Module from "module";
import * as path from "path";

const stubPath = path.join(__dirname, "vscodeStub.js");
const loader = Module as unknown as {
  _resolveFilename(request: string, ...rest: unknown[]): string;
};
const originalResolve = loader._resolveFilename;
loader._resolveFilename = function (request: string, ...rest: unknown[]) {
  return request === "vscode" ? stubPath : originalResolve.call(this, request, ...rest);
};
