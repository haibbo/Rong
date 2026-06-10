# Rong website

Marketing/landing site for **Rong (融)** — the JavaScript runtime for Rust.
Built with [Astro](https://astro.build) and deployed as static files to GitHub Pages.

## Module API pages

`/modules/<crate>/` pages are generated **at build time** from the repo's
existing markdown — no copies to maintain:

- `../docs/api/<name>.md` — the public JS API references (same source that
  `@rongjs/rong-skill` packages)
- `../modules/<crate>/README.md` — fallback for modules without a `docs/api`
  entry (currently only `rong_cron`)

The wiring lives in [`src/content.config.ts`](./src/content.config.ts)
(Astro content collections with glob loaders) and
[`src/pages/modules/[slug].astro`](./src/pages/modules/). The module list
itself is in [`src/data/modules.ts`](./src/data/modules.ts) — add a row there
when a new module crate lands; everything else updates automatically on build.

## Develop

```bash
cd website
npm install
npm run dev      # http://localhost:4321/Rong
```

## Build

```bash
npm run build    # outputs static site to ./dist
npm run preview  # serve the production build locally
```

## Deploy to GitHub Pages

This site is configured for a **project Pages site** at
`https://lingxia-dev.github.io/Rong` — note `base: '/Rong'` in
[`astro.config.mjs`](./astro.config.mjs).

Two options:

1. **GitHub Actions (recommended).** The workflow in
   [`.github/workflows/deploy.yml`](./.github/workflows/deploy.yml) builds and
   publishes on every push to `master` that touches `website/`. In the repo,
   go to **Settings → Pages → Build and deployment → Source: GitHub Actions**.
   (Move this workflow to the repo-root `.github/workflows/` if it isn't picked up.)

2. **Manual.** Run `npm run build` and push the contents of `dist/` to a
   `gh-pages` branch, or upload it however you prefer.

### Deploying somewhere else?

- **User/org site** (`<user>.github.io`) or **custom domain** → set `base: '/'`
  in `astro.config.mjs` and update `site`.
- The repo name in `base` must match exactly (case-sensitive) for asset URLs to resolve.
