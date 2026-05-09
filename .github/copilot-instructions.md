# Copilot instructions — `hello-world-webui-fast`

This file gives Copilot the context it needs to be productive in this repository on its first touch. Read it before making changes; pair it with [`README.md`](../README.md) (getting started) and [`DESIGN.md`](../DESIGN.md) (architecture deep-dive).

## What this project is

A minimal Rust + TypeScript demo that server-renders a single FAST 3.x custom element (`<hello-world>`) using the [`microsoft-webui`](https://crates.io/crates/microsoft-webui) crate's `fast-v3` plugin pipeline. Every layer of the stack is intentionally as small as possible — its job is to demonstrate the smallest useful integration between Rust SSR and FAST 3.x client hydration, not to be a starter template loaded with extras.

If you find yourself adding a dependency, a layer of abstraction, or a configuration knob, double-check that it is actually required by the requested change. Brevity is a feature here.

## Repository layout

```
hello-world-webui-fast/
├── .github/workflows/ci.yml           # build + Playwright on push / PR
├── Cargo.toml                         # workspace root, members = ["server"]
├── server/                            # actix-web binary (port 3000)
│   ├── Cargo.toml
│   └── src/main.rs
├── app/                               # browser bundle (esbuild + FAST 3.x)
│   ├── package.json
│   ├── tsconfig.json
│   ├── data/state.json                # render state passed to the protocol
│   └── src/
│       ├── index.html                 # entry document
│       ├── index.ts                   # enableHydration() + dynamic import
│       └── hello-world/
│           ├── hello-world.ts
│           └── hello-world.html       # WebUI declarative template
├── tests/                             # Playwright end-to-end tests
│   ├── package.json
│   ├── playwright.config.ts
│   └── hello-world.spec.ts
├── DESIGN.md
└── README.md
```

## Tech stack and pinned versions

| Layer            | Choice                                  | Why                                                                                |
| ---------------- | --------------------------------------- | ---------------------------------------------------------------------------------- |
| Rust HTTP        | `actix-web = "4"`                       | The framework every WebUI example server uses; aligns with WebUI's house style.    |
| WebUI            | `microsoft-webui = "0.0.12"`            | First published release exposing `Plugin::FastV3`. Do not downgrade.               |
| WebUI handler    | `microsoft-webui-handler = "0.0.12"`    | Provides `RenderOptions` and `FastV3HydrationPlugin` (not re-exported by `webui`). |
| FAST runtime     | `@microsoft/fast-element@3.0.0-rc.1`    | The only published FAST 3.x release on npm. Targets the FAST 3.x hydration format.     |
| Client bundler   | `esbuild ^0.25`                         | Matches WebUI examples (`hello-world`, `todo-fast`). Fast, single ESM bundle.      |
| End-to-end tests | `@playwright/test ^1.59`                | Drives a real Chromium browser against `cargo run`.                                |

Other Rust deps in `server/Cargo.toml`: `serde_json = "1"`, `anyhow = "1"`. Keep this list to five entries unless there is a strong reason — `axum`, `tower-http`, `tokio`, and `mime_guess` were intentionally removed during a refactor (see commit `13f45ac`).

## Build, run, and test

The canonical local flow:

```sh
# one-time
cd app && npm install
cd ../tests && npm install && npx playwright install chromium

# every iteration
cd app && npm run build           # rebuild the bundle when *.ts changes
cd ../server && cargo run         # serves on http://127.0.0.1:3000

# tests (will spawn the server itself via webServer)
cd tests && npx playwright test
```

Things to remember:

- The Rust server **pre-loads `app/dist/index.js` into memory at startup**. If the file is missing or stale, restart `cargo run`. There is no file watcher.
- The protocol is also **built once at startup** from `../app/src` and `../app/data/state.json`. Restart `cargo run` after editing any `.html`, `.json`, or `.rs` source.
- `Cargo.lock` and `package-lock.json` are gitignored — both regenerate on first build.
- `tests/playwright.config.ts` has a `webServer` block that runs `cargo run --quiet` in `../server`, so when running tests you do **not** need a separate server in another terminal. Locally, `reuseExistingServer` is on so iteration is fast.

## House conventions

### License header (required)

Every `.rs` and `.ts` file starts with:

```rust
// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
```

(Then a blank line, then the file's content.)

### Commit trailer (required)

Every commit must include this trailer:

```
Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

### Code style

- Prefer the `microsoft-webui` re-exports (`use webui::{ResponseWriter, WebUIHandler, …}`) over reaching into `webui_handler` directly. `RenderOptions` and `FastV3HydrationPlugin` are the only items that must come from `webui_handler::` because they are not re-exported.
- The `webui` and `webui_handler` *library* names differ from the *crate* names (`microsoft-webui` and `microsoft-webui-handler`). Use the lib names in `use` statements; use the crate names in `Cargo.toml`.
- TypeScript: `experimentalDecorators: true` and `useDefineForClassFields: false` are required for FAST's `@attr` decorator. Do not remove them from `app/tsconfig.json`.
- Custom elements register via `MyElement.define({ name, template: declarativeTemplate() })` — **not** the `@customElement` decorator. Keep registration in the same module that defines the class.
- `declarativeTemplate` is imported from `@microsoft/fast-element/declarative.js`. `enableHydration` is imported from `@microsoft/fast-element/hydration.js`. These two modules are easy to mix up; double-check.
- Comments only when something needs clarification — not as narration. The repo prefers terse code.

### WebUI declarative-template syntax

The component's `.html` file is authored in WebUI declarative syntax. The `fast-v3` parser plugin transforms it at server-build time:

| Syntax                            | Where evaluated                | Use                                                            |
| --------------------------------- | ------------------------------ | -------------------------------------------------------------- |
| `{{expr}}`                        | Server (and client on hydrate) | Content / attribute bindings against the render state.         |
| `{expr}` (single braces)          | Client only                    | Event handlers and attribute directives.                       |
| `@event="{handler()}"`            | Client only                    | Event binding. **Parentheses on the method name are required.** |
| `?attr="{{cond}}"`                | Server / client                | Boolean attribute binding.                                     |
| `<if condition="…">`              | Server                         | Compiled to `<f-when>` in the FAST template.                   |
| `<for each="x in xs">`            | Server                         | Compiled to `<f-repeat>`.                                      |

Event-binding gotcha: omitting the parens (`@click="{handler}"` vs `@click="{handler()}"`) silently breaks the binding. The renderer still allocates a `data-fe="N"` slot but FAST will not wire up the listener.

### Server-rendered output you should expect

For `<button @click="{handleButtonPress()}">Press me</button>` inside a component template, the rendered HTML contains:

```html
<button data-fe="1">Press me</button>
```

…inside the declarative shadow DOM, plus a server-injected `<f-template name="…">` block before `</body>` that retains the original `@click` expression. The host element (e.g. `<hello-world greeting="Hello world">`) does **not** carry a `data-fe` attribute itself unless its own attributes have client-only bindings.

## Tests

`tests/hello-world.spec.ts` is the canonical example for new tests. Key things to mirror:

- Read fixture state from `app/data/state.json` at module load and assert the rendered DOM against it. Do not hard-code the value — that defeats the test.
- Playwright **pierces shadow DOM by default**, so `page.locator("hello-world h1")` works on declarative shadow content. No `>>> ` or `pierce=` is needed.
- Before clicking an event-bound element, wait for hydration to complete:
  ```ts
  await page.waitForFunction(() => customElements.get("hello-world") !== undefined);
  await page.evaluate(() => new Promise<void>(r =>
      requestAnimationFrame(() => requestAnimationFrame(() => r()))));
  ```
  Without this, the click can fire before FAST has wired the handler and the test will be flaky.
- Capture `alert`, `confirm`, and `prompt` with `page.on("dialog", …)` **before** triggering the action that fires the dialog; the listener must already be attached when the event fires.
- For dialog assertions, push messages into an array and use `expect.poll(() => arr.at(-1)).toBe(...)` so the assertion retries until the dialog event has propagated.

## CI

`.github/workflows/ci.yml` is one `ubuntu-latest` job called `build-and-test`. Steps in order:

1. `actions/checkout@v4`
2. `actions/setup-node@v4` (Node 20)
3. `dtolnay/rust-toolchain@stable`
4. `Swatinem/rust-cache@v2` keyed on `server -> target`
5. `npm install` + `npm run build` in `app/`
6. `cargo build` in `server/`
7. `npm install` in `tests/`
8. `npx playwright install --with-deps chromium` in `tests/`
9. `npx playwright test` (with `CI=true`) in `tests/`
10. `actions/upload-artifact@v4` for `tests/playwright-report/` (`if: ${{ !cancelled() }}`, 14-day retention)

When adding new tests or new build steps, prefer extending this single job rather than introducing matrix builds — this is a demo project. If you do need a matrix, keep it minimal (e.g. ubuntu + macos) and reuse the same step list.

## Common pitfalls

- **Forgetting to rebuild the client bundle.** If `app/dist/index.js` is missing or stale, the server still renders HTML but the page will not hydrate, and Playwright tests for the click handler will fail. Run `npm run build` in `app/`.
- **Port 3000 already in use.** A leftover `cargo run` will block both manual smoke tests and Playwright's `webServer`. Use `lsof -nP -i tcp:3000 -sTCP:LISTEN -t | xargs kill -9` to free the port. Do not switch ports — both the server, `playwright.config.ts`, and the docs all assume 3000.
- **Mixing up `declarative.js` and `hydration.js` imports.** `declarativeTemplate` lives in `@microsoft/fast-element/declarative.js`; `enableHydration` lives in `@microsoft/fast-element/hydration.js`. The TypeScript compiler will not catch a swap because both are valid module specifiers.
- **Adding `axum`, `tokio`, `tower-http`, or `mime_guess`.** These were deliberately removed during the actix-web refactor (commit `13f45ac`). actix-web 4 supplies its own runtime and the static asset is a single pre-loaded `web::Bytes`. Do not re-introduce them without justification.
- **Reaching for FAST 1.x or 2.x APIs.** This project targets FAST 3.x (`@microsoft/fast-element@3.0.0-rc.1`). Tutorials and Stack Overflow answers about earlier versions often do not apply (e.g. `@customElement` decorator → `MyElement.define({...})`).

## Adding a new component

1. Create `app/src/<name>/<name>.ts` and `app/src/<name>/<name>.html`. Use `hello-world` as the template.
2. Author the template in WebUI declarative syntax (`{{...}}`, `@event="{handler()}"`, etc.).
3. Add the host element to `app/src/index.html` (e.g. `<my-thing prop="{{prop}}"></my-thing>`).
4. Add the field to `app/data/state.json` if the component reads from state.
5. Import the new component module from `app/src/index.ts` (or rely on the existing dynamic-import pattern if the component is the page's main element).
6. Run `npm run build` in `app/`, then restart `cargo run` so the protocol picks up the new template.
7. Add a Playwright spec to `tests/` mirroring `hello-world.spec.ts`.

## Reference repositories on disk

These are local clones used during development as references. They are **not** dependencies — the project depends on `microsoft-webui` 0.0.12 from crates.io and `@microsoft/fast-element@3.0.0-rc.1` from npm. Use them when you need to look something up:

- `../update-hydration-markers-3-rc-1/` — clone of `microsoft/webui` with `Plugin::FastV3` support. Useful examples: `examples/integration/ssr-performance-showdown/`, `examples/app/commerce/server/`, `examples/app/hello-world/`.
- `../add-declarative-docs/` — clone of `microsoft/fast` containing `sites/website/src/docs/3.x/declarative-templates/{overview,defining-elements,f-templates,server-rendering}.md`. The authoritative docs for the declarative-template syntax used in this repo.

If those clones are not available locally, fall back to:

- <https://github.com/microsoft/webui>
- <https://github.com/microsoft/fast/tree/releases/fast-element-v3>
- <https://crates.io/crates/microsoft-webui> (extract the `.crate` to inspect the public API of 0.0.12 directly).
