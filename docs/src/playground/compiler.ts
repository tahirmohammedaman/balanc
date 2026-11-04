// Lazy access to the balanc compiler compiled to WebAssembly (playground/ in the
// repo root). `playground/build.sh` (or `npm run build:wasm`) writes the
// wasm-bindgen output to ./pkg, which is gitignored. The glob keeps the site
// buildable without it: if ./pkg is missing the playground says so instead of
// failing the build. Nothing here is imported by the landing page, so neither
// the glue nor the .wasm lands in its bundle.

export interface Compiler {
  /** Exactly what `balanc --json playground.bal` prints. */
  check(source: string): string;
  /** Exactly what `balanc playground.bal` prints (stdout or stderr). */
  check_pretty(source: string): string;
}

interface WasmModule extends Compiler {
  default: () => Promise<unknown>;
}

const loaders = import.meta.glob<WasmModule>("./pkg/balanc_playground.js");

export class CompilerMissingError extends Error {
  constructor() {
    super("This build of the site was made without the WebAssembly compiler.");
  }
}

let pending: Promise<Compiler> | null = null;

/** Fetches and instantiates the compiler once; later calls share the result. */
export function loadCompiler(): Promise<Compiler> {
  if (!pending) {
    const load = loaders["./pkg/balanc_playground.js"];
    pending = load
      ? load().then(async (mod) => {
          await mod.default();
          return { check: mod.check, check_pretty: mod.check_pretty };
        })
      : Promise.reject(new CompilerMissingError());
    // Let a failed fetch be retried on the next call.
    pending.catch(() => {
      pending = null;
    });
  }
  return pending;
}

/** A diagnostic as `--json` prints it. */
export interface JsonDiagnostic {
  code: string;
  message: string;
  file: string;
  line: number;
  col: number;
  secondary?: { label: string; line: number; col: number }[];
}

export type JsonResult = { ok: true; report: string } | { ok: false; diagnostics: JsonDiagnostic[] };
