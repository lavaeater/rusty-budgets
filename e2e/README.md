# Rusty Budgets E2E tests (Playwright)

End-to-end tests that drive the **real fullstack web build** in a headless
Chromium. Unlike a client-only app, Rusty Budgets has a server + a SQL database,
so these tests boot the actual server against a **throwaway SQLite database** —
they never touch your real `data.json` / dev database, and they need no OAuth.

See `../docs/testing.md` (Layer 4) for how this fits the overall test strategy.

## Prerequisites

- The Dioxus CLI (`dx`) on `PATH` — `curl -sSL https://dioxus.dev/install.sh | sh`
- A Rust toolchain with the `wasm32-unknown-unknown` target
  (`rustup target add wasm32-unknown-unknown`)
- Node 18+
- One-time setup:
  ```sh
  cd e2e
  npm install
  npx playwright install chromium
  ```

## Running

```sh
cd e2e
npm test              # headless
npm run test:headed   # watch it in a real browser
npm run report        # open the last HTML report (after a run)
```

Playwright starts everything itself via the `webServer` block in
`playwright.config.js`:

```
cargo run -p api --bin api --features server   # migrate a fresh SQLite DB
  && dx serve -p web --fullstack true --web --addr 127.0.0.1 --port 8080 --hot-reload false
```

The first fullstack build (wasm client + server) can take several minutes on a
cold `target/` dir; later runs reuse the warm build. Override the port with
`RB_E2E_PORT`.

## How the throwaway DB works

The server picks its database from `DATABASE_URL` (welds/SQLite). The config:

1. Creates a per-run temp SQLite path and points `DATABASE_URL` at it.
2. Runs the `api` migration binary to create the schema. That binary would
   normally copy an existing JoyDB `data.json` into SQL, so we point its
   `DATA_FILE` at an **empty** temp file — making the copy a no-op and leaving a
   clean, empty, migrated database.
3. Boots `dx serve` with the same `DATABASE_URL`.

On first request the server auto-creates the default user
(`get_default_user`), and because the DB has no budget yet, the app renders the
"Ingen budget hittad" create-budget screen — the natural starting point for the
onboarding spec.

To reuse a warm DB across a local dev loop, export `RB_E2E_DB` (and optionally
`RB_E2E_DATA`) to fixed paths.

## Current specs

| Spec | Covers |
| --- | --- |
| `onboarding.spec.js` | Fresh DB shows the create-budget screen; creating a budget lands on the overview (name heading + "Verktyg" tools). Runs `describe.serial` so state builds up over one shared DB. |

## Adding a spec

- Keep suites deterministic: the whole run shares one DB, so either use
  `test.describe.serial` and let state accumulate, or assert only on
  state-independent UI.
- Locate elements by role/text where possible; add a stable `id` /
  `data-testid` in the RSX when a selector is fragile.
- Good next targets (see docs/testing.md): import a Skandia `.xlsx` fixture →
  transactions appear; the tagging loop; transfer-pair resolution; budget-item
  creation.
