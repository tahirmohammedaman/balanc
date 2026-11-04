# balanc-docs

The balanc documentation site — a standalone React + TypeScript single-page app (Vite + React Router), deployable independently of the `balanc` Rust crate.

## Develop

```
npm install
npm run dev
```

## Compiler output

Every program and every line of compiler output on the site is read from
`src/data/outputs.generated.json`, produced by running the real compiler over
`examples/**/*.bal` and `tests/cases/*.bal`:

```
cargo build                      # in the repo root
npm run gen:outputs              # in docs/
```

The script fails if any code in `src/diag/codes.rs` has no program that
triggers it (add one under `examples/errors/`). CI (`.github/workflows/docs.yml`)
regenerates the file and fails if the committed copy is stale.

## Playground (WebAssembly)

`/playground` type-checks a snippet in the browser with the real compiler,
compiled to `wasm32-unknown-unknown` from the `playground/` crate in the repo
root. Build it before `npm run build`:

```
rustup target add wasm32-unknown-unknown
npm run build:wasm                # = ../playground/build.sh
```

This writes `src/playground/pkg/` (gitignored). The page and the `.wasm` are
lazy-loaded, so neither is in the landing bundle. See `playground/README.md`.

## Build

```
npm run build      # outputs static assets to dist/
npm run typecheck  # tsc --noEmit
npm run preview    # serve the production build locally
```

`dist/` is a fully static bundle — no server-side code, no API. Any static host works.

## Deploy

This is a client-side-routed SPA (React Router with clean URLs, e.g. `/docs/getting-started`), so the host needs to fall back to `index.html` for unknown paths instead of 404ing.

- **Netlify** — add a `public/_redirects` containing `/*  /index.html  200`. It is not included, because Cloudflare Workers also reads it and it conflicts with the Workers SPA fallback, causing a redirect loop. Build command `npm run build`, publish directory `dist`.
- **Vercel** — `vercel.json` (already included) rewrites all paths to `index.html`. Framework preset "Vite" works out of the box.
- **Cloudflare Workers** — `wrangler.jsonc` (already included) serves `dist/` as static assets with `not_found_handling: "single-page-application"`. Run `npm run build && npx wrangler deploy`.
- **Cloudflare Pages** — build command `npm run build`, output directory `dist`; SPA fallback is automatic.
- **GitHub Pages** — GitHub Pages has no server-side rewrite support, so a clean-URL `BrowserRouter` needs either a `404.html` redirect trick or switching `App.tsx` to `HashRouter`. Not configured here since GitHub Pages isn't the primary target.

The only build step that depends on the Rust project is the playground's
WebAssembly compiler (see below). Without it the site still builds and deploys
on its own; `/playground` then says the compiler isn't included.

## Structure

```
src/
├── App.tsx                 route table
├── context/ThemeContext.tsx  dark/light theme, persisted to localStorage
├── components/              shared UI: nav, sidebar, code blocks, doc-page chrome
│   └── code/                 highlight.js grammars (hljs-balanc.ts) + CodeBlock/Terminal/ProgramBlock
├── data/nav.ts               the sidebar's page tree (also drives prev/next + search)
├── data/programs.ts          typed access to outputs.generated.json
├── pages/Landing.tsx         marketing landing page ("/")
├── pages/Playground.tsx      in-browser type-checker ("/playground", lazy-loaded)
├── playground/compiler.ts    loads the wasm-bindgen output in playground/pkg/
└── pages/docs/*.tsx          one component per documentation page, routed under "/docs/*"
```

Code is highlighted with highlight.js core. `src/components/code/hljs-balanc.ts`
defines two grammars, `balanc` (`.bal` source, token set mirroring
`src/lex/mod.rs`) and `balanc-output` (diagnostics and trial balances), as plain
`LanguageFn`s usable in any highlight.js setup.
