# Vynk landing page

A single-page introduction to Vynk, with an interactive admission model, command examples, source links, and installation instructions. Built with Next.js, React, TypeScript, and custom CSS.

Project source: https://github.com/pratham-srivastava-07/vynk

## Development

Use Node.js 22.13 or newer, preferably an LTS release.

```sh
cd ui
npm ci
npm run dev
```

Open http://localhost:3000. A running Vynk server is not required.

Set `NEXT_PUBLIC_SITE_URL` to the deployed origin for canonical and social metadata. Vercel deployments use `VERCEL_PROJECT_PRODUCTION_URL` automatically when the explicit value is absent.

## Verification

```sh
npm run lint
npm test
npm run test:browser
npm run format:check
npm run build
```

Browser tests use installed Microsoft Edge locally and bundled Chromium in CI. They start the development server if needed and cover desktop/mobile layout, simulation controls, tabs, clipboard behavior, navigation, reduced motion, text contrast, and mobile target sizes. Screenshots are saved in the ignored test-results directory.

Production builds prerender the page. Font downloads through next/font need network access during build. Use npm start to preview the production build.

## Structure

- app/page.tsx: page content and sections.
- app/globals.css: visual system and responsive styles.
- components/cache-art.tsx: SVG illustration.
- components/cache-playground.tsx: interactive demonstration.
- components/controls.tsx: navigation, command tabs, copy controls.
- lib/simulation.ts: deterministic demonstration model.
- lib/site.ts: project and installation links.

The browser model uses 12 slots, exact frequency counters, exact LRU, and decay every 24 requests. It illustrates trade-offs; it is not a benchmark of Vynk or Redis. Vynk uses sampled approximate LRU and a bounded frequency sketch. Rust test counts describe a recorded verification run, not live status.

No server API, analytics, or persistence is needed for this page.
