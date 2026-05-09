# Hello World WebUI FAST

[![CI](https://github.com/janechu/hello-world-webui-fast/actions/workflows/ci.yml/badge.svg)](https://github.com/janechu/hello-world-webui-fast/actions/workflows/ci.yml)

This project is a minimal Rust + TypeScript example that uses the `microsoft-webui` 0.0.12 Rust crate to server-render one FAST 3.x custom element. The element template is authored in WebUI declarative syntax (`{{...}}` content bindings, `<for>` / `<if>`), transformed by the `fast-v3` plugin into FAST 3.x `<f-template>` markup with hydration markers, and hydrated on the client with `@microsoft/fast-element@3.0.0-rc.1`, `enableHydration()`, and `declarativeTemplate()`.

## Prerequisites

- Rust toolchain 1.93+ (matches the `microsoft-webui` 0.0.12 MSRV)
- Node.js 20+
- npm

## Setup and run

From the project root:

```bash
cd app
npm install
npm run build
cd ../server
cargo run
```

Open <http://127.0.0.1:3000/>. The page should display **"Hello world"** inside a `<hello-world>` custom element.

Use view-source to inspect the server-rendered output. You should see FAST 3.x hydration markers such as `<!--fe:b-->` and `data-fe="N"`, plus a server-injected `<f-template name="hello-world">` before `</body>`.

## Project layout

```text
hello-world-webui-fast/
├── .github/workflows/ci.yml           # GitHub Actions: build + Playwright
├── Cargo.toml                         # workspace root, members = ["server"]
├── server/
│   ├── Cargo.toml                     # binary "hello-world-webui-fast-server"
│   └── src/main.rs                    # actix-web entrypoint, port 3000
├── app/
│   ├── package.json                   # @microsoft/fast-element@3.0.0-rc.1 + esbuild
│   ├── tsconfig.json                  # TypeScript compiler settings
│   ├── data/state.json                # { "greeting": "Hello world" }
│   └── src/
│       ├── index.html                 # host page for the rendered component
│       ├── index.ts                   # enableHydration() + dynamic import
│       └── hello-world/
│           ├── hello-world.ts         # custom element definition
│           └── hello-world.html       # WebUI declarative template
├── tests/
│   ├── package.json                   # @playwright/test
│   ├── playwright.config.ts           # webServer launches `cargo run`
│   └── hello-world.spec.ts            # end-to-end tests
├── DESIGN.md                          # architecture deep-dive
└── README.md                          # getting-started guide
```

## How it works (short version)

The Rust server loads the app template and state, builds the WebUI protocol once at startup, and serves the rendered page through actix-web (the same framework the upstream WebUI examples use). WebUI processes the declarative template, and the `fast-v3` plugin emits FAST-compatible `<f-template>` markup and hydration metadata. The browser loads the bundled TypeScript, enables FAST hydration, and registers the `<hello-world>` element with `declarativeTemplate()`. FAST then hydrates the pre-rendered shadow DOM instead of rebuilding it. See [DESIGN.md](./DESIGN.md) for the full architecture breakdown.

## Development tips

- Re-run `npm run build` in `app/` after editing any `.ts` file.
- Restart `cargo run` after editing any `.rs` file or any HTML/state file under `app/src` or `app/data`; the protocol is built once at startup.
- Run `RUST_BACKTRACE=1 cargo run` to see Rust panic backtraces.

## Testing

End-to-end tests live in `tests/` and use [Playwright](https://playwright.dev/). The Playwright config has a `webServer` block that launches `cargo run` in `server/` automatically before the tests run.

First time setup:

```bash
cd app && npm install && npm run build
cd ../tests && npm install
npx playwright install chromium
```

Run the tests:

```bash
cd tests
npx playwright test
```

The tests:

1. Verify that the rendered `<hello-world>` element shows the greeting from `app/data/state.json`.
2. Click the **Press me** button and assert the resulting `alert` says `button pressed!`.

Playwright's HTML report is written to `tests/playwright-report/`. View it with `npx playwright show-report` from `tests/`.

## Continuous integration

`.github/workflows/ci.yml` runs on every push to `main` and on every pull request. It installs Node and Rust toolchains, caches the cargo build, builds the client bundle and Rust server, installs Playwright (with its Chromium browser), and runs the tests. The Playwright HTML report is uploaded as a workflow artifact.

## Troubleshooting

- If `cargo build` fails on the first invocation due to network issues, re-run it; cargo retries are not automatic.
- If `app/dist/index.js` is missing, the page will load HTML but the component will not hydrate. Run `npm run build` in `app/`.
- If you see "Address already in use" on port 3000, another process is bound to it. Stop that process or change the port in `server/src/main.rs`.

## Where to learn more

- [microsoft/webui](https://github.com/microsoft/webui)
- [FAST declarative templates docs](https://github.com/microsoft/fast/tree/releases/fast-element-v3/sites/website/src/docs/3.x/declarative-templates)
- [DESIGN.md](./DESIGN.md) for this project's architecture deep-dive
