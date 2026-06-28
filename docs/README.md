# balanc-docs

The balanc documentation site — a standalone React + TypeScript single-page app (Vite + React Router), deployable independently of the `balanc` Rust crate.

## Develop

```
npm install
npm run dev
```

## Build

```
npm run build      # outputs static assets to dist/
npm run typecheck  # tsc --noEmit
npm run preview    # serve the production build locally
```

`dist/` is a fully static bundle — no server-side code, no API. Any static host works.

## Deploy

This is a client-side-routed SPA (React Router with clean URLs, e.g. `/docs/getting-started`), so the host needs to fall back to `index.html` for unknown paths instead of 404ing.

- **Netlify** — `public/_redirects` (already included) handles this automatically. Build command `npm run build`, publish directory `dist`.
- **Vercel** — `vercel.json` (already included) rewrites all paths to `index.html`. Framework preset "Vite" works out of the box.
- **Cloudflare Pages** — build command `npm run build`, output directory `dist`; SPA fallback is automatic.
- **GitHub Pages** — GitHub Pages has no server-side rewrite support, so a clean-URL `BrowserRouter` needs either a `404.html` redirect trick or switching `App.tsx` to `HashRouter`. Not configured here since GitHub Pages isn't the primary target.

No build step depends on the Rust project — this directory can be copied out, given its own git remote, and deployed on its own.

## Structure

```
src/
├── App.tsx                 route table
├── context/ThemeContext.tsx  dark/light theme, persisted to localStorage
├── components/              shared UI: nav, sidebar, code blocks, doc-page chrome
│   └── code/                 hand-written syntax tokenizer + CodeBlock/Terminal
├── data/nav.ts               the sidebar's page tree (also drives prev/next + search)
├── pages/Landing.tsx         marketing landing page ("/")
└── pages/docs/*.tsx          one component per documentation page, routed under "/docs/*"
```

Every code sample is authored as a plain string and highlighted client-side by a small regex-based tokenizer (`src/components/code/tokenize.ts`) — no highlighting library, matching the source project's own zero-dependency approach where it reasonably applies to a docs site.
