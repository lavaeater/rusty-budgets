// @ts-check
const { defineConfig, devices } = require("@playwright/test");
const os = require("os");
const path = require("path");
const fs = require("fs");

// Rusty Budgets is a *fullstack* Dioxus app: `dx serve` builds both the wasm
// client and the server, and the server talks to a SQL database selected by the
// `DATABASE_URL` env var (welds/SQLite — see api/src/cqrs/runtime.rs).
//
// E2E isolation strategy (docs/testing.md, Layer 4): every run gets its own
// throwaway SQLite file so the tests never touch the developer's real data. The
// schema is created by the `api` migration binary; pointing its `DATA_FILE`
// (the JoyDB source it would copy from) at an empty temp file makes that copy a
// no-op, so we get a clean, empty, migrated database. The default user is then
// auto-created by the server on first request (get_default_user), and a fresh
// DB has no default budget — so the app boots into the "create budget" screen.

const PORT = Number(process.env.RB_E2E_PORT || 8080);
const BASE_URL = `http://127.0.0.1:${PORT}`;
const REPO_ROOT = path.resolve(__dirname, "..");

// Per-run throwaway DB + empty JoyDB source. Reuse across a `reuseExistingServer`
// dev loop by exporting RB_E2E_DB / RB_E2E_DATA yourself.
const TMP = fs.mkdtempSync(path.join(os.tmpdir(), "rb-e2e-"));
const E2E_DB = process.env.RB_E2E_DB || path.join(TMP, "e2e.sqlite");
const EMPTY_DATA = process.env.RB_E2E_DATA || path.join(TMP, "empty.json");
const DATABASE_URL = `sqlite://${E2E_DB}?mode=rwc`;

const SERVER_ENV = {
  ...process.env,
  DATABASE_URL,
  DATA_FILE: EMPTY_DATA,
};

// The first fullstack build (wasm client + server) on a cold target dir can take
// several minutes — give it a generous startup window.
const SERVER_TIMEOUT = 15 * 60 * 1000;

// Run the migration binary first, then boot the app. Chaining in a single
// `command` guarantees the schema exists before `dx serve` accepts requests,
// with no reliance on globalSetup/webServer ordering. `--hot-reload false`
// keeps a rebuild overlay from racing the tests (the oxidian lesson).
const MIGRATE = `cargo run -p api --bin api --features server`;
const SERVE = `dx serve -p web --fullstack true --web --addr 127.0.0.1 --port ${PORT} --hot-reload false`;

module.exports = defineConfig({
  testDir: "./tests",
  // Tests share one server + one DB, so run them serially and let describe.serial
  // blocks build up state deterministically.
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["html", { open: "never" }], ["list"]] : "list",
  timeout: 60 * 1000,
  expect: { timeout: 15 * 1000 },
  use: {
    baseURL: BASE_URL,
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: `${MIGRATE} && ${SERVE}`,
    cwd: REPO_ROOT,
    url: BASE_URL,
    timeout: SERVER_TIMEOUT,
    reuseExistingServer: !process.env.CI,
    env: SERVER_ENV,
    stdout: "pipe",
    stderr: "pipe",
  },
});
