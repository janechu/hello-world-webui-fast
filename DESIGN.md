# Hello World WebUI FAST Design

## Overview

`hello-world-webui-fast` is a minimal Rust + TypeScript application that server-renders a single FAST 3.x custom element displaying `Hello world`.

The Rust side is a small actix-web HTTP server. At startup, it uses `microsoft-webui` 0.0.12, whose library name is `webui`, to compile `app/src/index.html` and component templates into a `WebUIProtocol`. The build uses `Plugin::FastV3`, which selects the FAST 3.x parser plugin.

The TypeScript side defines a single FAST Element component, `<hello-world>`, using `@microsoft/fast-element@3.0.0-rc.1`. The component template is authored in WebUI declarative syntax and transformed by the `fast-v3` parser and hydration plugins into FAST 3.x-compatible `<f-template>` markup with hydration markers such as `<!--fe:b-->`, `<!--fe:/b-->`, and `data-fe="N"`.

At request time, the server walks the compiled protocol with `WebUIHandler` and `FastV3HydrationPlugin`, producing HTML that includes declarative shadow DOM and FAST hydration markers. In the browser, `enableHydration()` and `declarativeTemplate()` allow FAST 3.x to attach reactive bindings to the existing server-rendered DOM without rebuilding it.

## File Layout

```text
hello-world-webui-fast/
├── .github/
│   └── workflows/ci.yml               # CI: build + Playwright tests
├── .gitignore
├── Cargo.toml                         # workspace root, members = ["server"]
├── server/
│   ├── Cargo.toml                     # binary "hello-world-webui-fast-server"
│   └── src/main.rs                    # actix-web entrypoint
├── app/
│   ├── package.json                   # @microsoft/fast-element@3.0.0-rc.1 + esbuild
│   ├── tsconfig.json
│   ├── data/state.json                # { "greeting": "Hello world" }
│   └── src/
│       ├── index.html                 # <hello-world greeting="{{greeting}}"></hello-world>
│       ├── index.ts                   # enableHydration + dynamic import of component
│       └── hello-world/
│           ├── hello-world.ts         # FASTElement w/ @attr greeting + handleButtonPress()
│           └── hello-world.html       # <h1>{{greeting}}</h1> + <button @click="{...}">
├── tests/
│   ├── package.json                   # @playwright/test
│   ├── playwright.config.ts           # webServer launches `cargo run`
│   └── hello-world.spec.ts            # end-to-end tests
└── README.md
```

Important paths:

- `server/src/main.rs` contains the actix-web server, startup build step, request handlers, and `StringWriter` implementation.
- `app/src/index.html` is the server-rendered entry document and hosts `<hello-world greeting="{{greeting}}"></hello-world>`.
- `app/src/index.ts` enables FAST hydration and dynamically imports the component definition.
- `app/src/hello-world/hello-world.ts` defines the `HelloWorld` FAST element with an `@attr greeting` property and a `handleButtonPress()` method that calls `alert("button pressed!")`. Registration happens via `HelloWorld.define({ name, template: declarativeTemplate() })`.
- `app/src/hello-world/hello-world.html` contains the WebUI declarative template for the component shadow DOM, including a content binding (`{{greeting}}`) and an event binding (`@click="{handleButtonPress()}"`).
- `app/data/state.json` provides the render state: `{ "greeting": "Hello world" }`.
- `tests/` contains a Playwright suite that drives a real Chromium browser against `cargo run`. See [Testing](#testing).
- `.github/workflows/ci.yml` builds the project and runs the Playwright suite on every push to `main` and every pull request. See [Continuous Integration](#continuous-integration).

## Rust Architecture

The server is a single actix-web binary named `hello-world-webui-fast-server`, defined under `server/` and launched with `cargo run` from that directory.

At startup, `server/src/main.rs` calls `webui::build` to compile the WebUI entry document and component templates into a `WebUIProtocol`. The relevant API surface from `microsoft-webui` 0.0.12 is:

- `webui::build(...)`
- `webui::BuildOptions`
- `webui::CssStrategy::Link`
- `webui::DomStrategy::Shadow`
- `webui::Plugin::FastV3`
- `webui::WebUIProtocol`
- `webui::WebUIHandler::with_plugin(...)`

A representative startup build call is:

```rust
use webui::{build, BuildOptions, CssStrategy, DomStrategy, Plugin};

let build_result = build(BuildOptions {
    app_dir: "../app".into(),
    entry: "src/index.html".into(),
    css: CssStrategy::Link,
    dom: DomStrategy::Shadow,
    plugin: Some(Plugin::FastV3),
    components: vec![],
})?;

let protocol = build_result.protocol;
```

The protocol is stored in application state, typically in an `Arc<AppState>`, together with the parsed `app/data/state.json` value. This makes the compiled protocol reusable across requests while keeping per-request rendering isolated.

The render-time hydration plugin comes from `microsoft-webui-handler` 0.0.12, whose library name is `webui_handler`:

```rust
use webui::WebUIHandler;
use webui_handler::plugin::fast_v3::FastV3HydrationPlugin;

let handler = WebUIHandler::with_plugin(|| Box::new(FastV3HydrationPlugin::new()));
```

The server implements the `webui_handler::ResponseWriter` trait with a `StringWriter` so rendered HTML can be captured into a `String` and returned from the actix-web handler as `text/html; charset=utf-8`.

## TypeScript Architecture

The client application is intentionally small:

- `app/package.json` depends on `@microsoft/fast-element@3.0.0-rc.1` and `esbuild`.
- `app/src/index.ts` imports `enableHydration()` from `@microsoft/fast-element/hydration.js` and then dynamically imports the component module.
- `app/src/hello-world/hello-world.ts` defines and registers the `<hello-world>` custom element.
- `app/src/hello-world/hello-world.html` provides the WebUI declarative template.

Conceptually, `app/src/index.ts` performs:

```ts
import { enableHydration } from "@microsoft/fast-element/hydration.js";

enableHydration();
void import("./hello-world/hello-world.js");
```

The component definition uses FAST Element with `declarativeTemplate()` from `@microsoft/fast-element/declarative.js` and registers via the static `define()` method:

```ts
import { attr, FASTElement } from "@microsoft/fast-element";
import { declarativeTemplate } from "@microsoft/fast-element/declarative.js";

export class HelloWorld extends FASTElement {
    @attr greeting: string = "Hello world";

    handleButtonPress(): void {
        alert("button pressed!");
    }
}

void HelloWorld.define({
    name: "hello-world",
    template: declarativeTemplate(),
});
```

The template is authored in WebUI declarative syntax, not directly in FAST's final `<f-template>` form:

```html
<template shadowrootmode="open">
  <h1>{{greeting}}</h1>
  <button @click="{handleButtonPress()}">Press me</button>
</template>
```

`FastV3ParserPlugin` converts WebUI declarative syntax into FAST 3.x template syntax. In this minimal project there are no `<if>` or `<for>` directives, but the same pipeline supports:

- `<if condition="...">` into `<f-when>`
- `<for each="...">` into `<f-repeat>`
- `{{...}}` content / attribute bindings → FAST template bindings + hydration markers (`<!--fe:b-->`, `data-fe="N"`)
- `@event="{handler()}"` event bindings (single curly braces, client-only) → FAST event bindings; the server strips the inline expression from the rendered HTML but allocates a hydration slot on the element via `data-fe="N"`

## Web Layer

The HTTP layer uses actix-web 4. actix-web is what the upstream WebUI repository itself uses (it is a workspace dependency in the WebUI project, and every example server under `examples/` uses actix-web), so the server code stays close to the WebUI house style.

There are two routes:

- `GET /` → `render_root` (server-rendered HTML)
- `GET /dist/index.js` → `serve_index_js` (the bundled browser module)

The HTML route handler:

1. Receives `web::Data<AppState>`.
2. Creates a `StringWriter`.
3. Constructs a per-request `WebUIHandler` with `FastV3HydrationPlugin`.
4. Calls `handler.handle(...)` with the compiled `WebUIProtocol`, render state, `RenderOptions`, and writer.
5. Returns the resulting HTML buffer as `text/html; charset=utf-8`.

The `index.js` bundle is read from `app/dist/index.js` once at startup, stored as an `actix_web::web::Bytes` in `AppState`, and cloned cheaply into responses (the same pre-load pattern used by `commerce/server/src/frontend.rs` upstream). This avoids a `tower-http` dependency and keeps the static-asset path inline with the WebUI house style.

A representative route flow looks like:

```rust
use actix_web::{web, HttpResponse};
use webui::{ResponseWriter, WebUIHandler};
use webui_handler::RenderOptions;
use webui_handler::plugin::fast_v3::FastV3HydrationPlugin;

async fn render_root(state: web::Data<AppState>) -> HttpResponse {
    let mut writer = StringWriter::default();
    let options = RenderOptions::new("index.html", "/");
    let handler = WebUIHandler::with_plugin(|| Box::new(FastV3HydrationPlugin::new()));

    if let Err(err) = handler.handle(&state.protocol, &state.render_state, &options, &mut writer) {
        return HttpResponse::InternalServerError().body(format!("render failed: {err}"));
    }

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(writer.into_string())
}
```

## Build Pipeline

The build pipeline has three stages: install client dependencies, bundle the browser module, and start the Rust server.

1. Install JavaScript dependencies:

   ```sh
   cd app
   npm install
   ```

   This installs `@microsoft/fast-element@3.0.0-rc.1` and `esbuild`.

2. Bundle the client entry point:

   ```sh
   npm run build
   ```

   The package script runs:

   ```sh
   esbuild src/index.ts --bundle --outfile=dist/index.js --format=esm --sourcemap
   ```

   The output is a single ESM module at `app/dist/index.js` containing FAST Element and the `<hello-world>` registration path.

3. Start the server:

   ```sh
   cd server
   cargo run
   ```

   During `main()`, the server calls `webui::build` with `Plugin::FastV3` against `../app/src/index.html` and the discovered `hello-world/hello-world.html` template. The output is a `BuildResult` whose `protocol` field is the compiled `WebUIProtocol` used for all subsequent requests.

## Render Pipeline

Every request to `GET /` follows the same render path.

1. actix-web dispatches the request to `render_root`.
2. `render_root` creates a new `StringWriter`.
3. The handler is constructed with the FAST 3.x hydration plugin:

   ```rust
   let handler = WebUIHandler::with_plugin(|| Box::new(FastV3HydrationPlugin::new()));
   ```

4. Per-request options identify the entry and route path:

   ```rust
   let options = RenderOptions::new("index.html", "/");
   ```

5. The handler walks the compiled protocol:

   ```rust
   handler.handle(&protocol, &state, &options, &mut writer)?;
   ```

While walking protocol fragments, `FastV3HydrationPlugin` emits FAST 3.x hydration markers:

- Attribute bindings on elements with bindings receive `data-fe="N"`, where `N` is the binding count.
- Content bindings such as `{{greeting}}` are resolved against `app/data/state.json` and wrapped with `<!--fe:b-->` and `<!--fe:/b-->` markers.
- At `</body>`, an artifact emitted earlier by the parser plugin injects an `<f-template name="hello-world">` block containing the FAST-converted component template.

For this project, the rendered component template includes a content binding and an event binding. Conceptually, the emitted FAST template is:

```html
<f-template name="hello-world">
  <template>
    <h1>{{greeting}}</h1>
    <button @click="{handleButtonPress()}">Press me</button>
  </template>
</f-template>
```

The complete response contains:

- The host element from `app/src/index.html` with the resolved attribute:

  ```html
  <hello-world greeting="Hello world">…</hello-world>
  ```

- Declarative shadow DOM for the component, with FAST 3.x hydration markers:

  ```html
  <template shadowrootmode="open">
    <h1><!--fe:b-->Hello world<!--fe:/b--></h1>
    <button data-fe="1">Press me</button>
  </template>
  ```

  `data-fe="1"` on the `<button>` reserves a single binding slot for the client-only `@click` handler — the inline `{handleButtonPress()}` expression is stripped from the rendered HTML.

- The `<f-template name="hello-world">` artifact before `</body>` (with the `@click` expression preserved so the browser can wire up the handler).
- A `window.__webui` script element carrying the inventory and the JSON state.
- A `<script type="module" src="/dist/index.js">` reference.

## Hydration Pipeline

Hydration is performed in the browser by FAST Element 3.

1. The browser parses the server response. The `<hello-world>` host element has a resolved `greeting="Hello world"` attribute and pre-rendered shadow DOM via `<template shadowrootmode="open">`.
2. `/dist/index.js` loads.
3. `enableHydration()` from `@microsoft/fast-element/hydration.js` activates the FAST hydration path so element controllers reuse server-rendered DOM.
4. The dynamic component import registers the `HelloWorld` class.
5. `declarativeTemplate()` waits for the matching server-emitted `<f-template name="hello-world">`.
6. FAST parses the `<f-template>` into a `ViewTemplate` and associates it with the `<hello-world>` definition.
7. FAST walks the existing declarative shadow DOM and locates:
   - `<!--fe:b-->` and `<!--fe:/b-->` content markers.
   - `data-fe="N"` attribute binding counters (e.g. on the `<button>` for the `@click` handler).
8. FAST re-attaches reactive bindings to the existing nodes without creating duplicate DOM. The `@click` handler on the `<button>` is wired up to `HelloWorld.handleButtonPress`, which calls `alert("button pressed!")`.
9. `$fastController.isPrerendered` resolves to `true`.
10. Later property changes, such as `helloWorld.greeting = "Hi"`, update only the marker-bound text node.

The key property of this pipeline is that the server-rendered DOM becomes the live FAST view. The client runtime hydrates it instead of replacing it.

## Testing

End-to-end tests live in `tests/` and use [Playwright](https://playwright.dev/). The suite drives a real Chromium browser against the actual Rust server, so it exercises the full pipeline — server-side rendering → declarative shadow DOM → client bundle → `enableHydration()` → `declarativeTemplate()` → FAST event-binding wire-up — exactly as a user would experience it.

### Configuration

`tests/playwright.config.ts` declares a `webServer` block that launches `cargo run --quiet` in `../server/` and waits for `http://127.0.0.1:3000/` before any test runs. Locally, `reuseExistingServer` is on so iteration is fast; in CI it is off so each run gets a clean server. The base URL is set so tests can use relative paths (e.g. `page.goto("/")`).

The Chromium project from `devices["Desktop Chrome"]` is the only browser configured. Under CI (`process.env.CI`), the config switches to a single worker, two retries, and the `github` + `html` reporters.

### Tests

`tests/hello-world.spec.ts` contains two tests:

1. **Greeting reflects `state.json`.** The spec reads `app/data/state.json` synchronously at load time, navigates to `/`, and asserts that the `<h1>` inside the `<hello-world>` declarative shadow DOM has text equal to `state.greeting`. Playwright's selectors pierce shadow DOM, so `page.locator("hello-world h1")` finds the text whether the shadow root is open or not.
2. **Button click fires the alert.** The spec subscribes a `page.on("dialog")` listener that records the message and dismisses the dialog. It then waits for `customElements.get("hello-world")` to be defined and yields two `requestAnimationFrame` ticks so FAST hydration has wired the `@click` handler. Finally it clicks `hello-world button` and uses `expect.poll` to assert the most recent recorded dialog message equals `"button pressed!"`.

The waitForFunction + double-RAF pattern is necessary because the click handler is set up only after the dynamic import resolves and FAST hydration walks the `data-fe="1"` slot on the `<button>`. Clicking before that point would fall on a plain `<button>` with no listener.

### Running locally

```sh
cd app && npm install && npm run build
cd ../tests && npm install
npx playwright install chromium
npx playwright test
```

`npx playwright show-report` opens the HTML report.

## Continuous Integration

`.github/workflows/ci.yml` runs on every push to `main`, every pull request, and on manual `workflow_dispatch`. It is a single `ubuntu-latest` job called `build-and-test` with these steps:

1. **Checkout** (`actions/checkout@v4`).
2. **Set up Node.js** 20 (`actions/setup-node@v4`).
3. **Set up Rust toolchain** stable (`dtolnay/rust-toolchain@stable`).
4. **Cache the cargo build** (`Swatinem/rust-cache@v2`, scoped to the `server -> target` workspace).
5. **`npm install`** in `app/`, then **`npm run build`** to produce `app/dist/index.js`.
6. **`cargo build`** in `server/` to compile the binary ahead of test runs.
7. **`npm install`** in `tests/`.
8. **`npx playwright install --with-deps chromium`** in `tests/`.
9. **`npx playwright test`** in `tests/` with `CI=true` set, which activates the retry / single-worker / GitHub reporter behavior in `playwright.config.ts`.
10. **Upload the Playwright HTML report** (`actions/upload-artifact@v4`) from `tests/playwright-report/` whenever the job is not cancelled. The artifact is retained for 14 days.

The Playwright `webServer` block in the config is what actually runs the Rust binary during the test step, so there is no separate "start server" step in the workflow.

## Design Choices

### `microsoft-webui` 0.0.12 from crates.io

Version 0.0.12 is used because it is the first published release exposing `Plugin::FastV3`. The earlier 0.0.11 release only exposed `Plugin::Fast` and `Plugin::FastV2`. Using the crates.io release keeps this project self-contained and avoids path dependencies on other local working clones.

### `@microsoft/fast-element@3.0.0-rc.1`

`@microsoft/fast-element@3.0.0-rc.1` is the only published 3.x release of FAST Element on npm. It is the runtime targeted by `Plugin::FastV3`, which emits the compact FAST 3.x hydration marker format using `fe:b` content markers and `data-fe="N"` attribute counters.

### WebUI declarative syntax

The component template in `app/src/hello-world/hello-world.html` is intentionally authored in WebUI declarative syntax:

```html
<h1>{{greeting}}</h1>
```

This keeps the source template framework-neutral at authoring time. The `FastV3ParserPlugin` performs the FAST-specific conversion, including support for transforming WebUI directives such as `<if>` and `<for>` into FAST 3.x `<f-when>` and `<f-repeat>` markup.

### actix-web

actix-web is the framework the upstream WebUI project itself uses — it is in WebUI's workspace `[workspace.dependencies]` and is the framework chosen by every example server (`examples/integration/ssr-performance-showdown`, `examples/app/commerce/server`, `examples/app/contact-book-manager/server`, `examples/app/routes/server`, `examples/demo/server`). Picking actix-web here keeps the dependency graph aligned with WebUI's own ecosystem and avoids pulling in a parallel HTTP framework like axum plus its tower middleware stack.

For this minimal example the server doesn't need static-file serving abstractions like `tower-http::ServeDir`. The single `index.js` asset is read once at startup, stored as `actix_web::web::Bytes`, and served with a tiny inline handler — exactly how `commerce/server/src/frontend.rs` caches its assets.

### esbuild

The project uses esbuild because WebUI examples such as `hello-world` and `todo-fast` use it for client bundling. It is fast, has minimal configuration, and emits a single ESM bundle suitable for this small demo.

### Declarative shadow DOM

`DomStrategy::Shadow` and `<template shadowrootmode="open">` allow the server to send shadow DOM in the initial HTML response. Browsers that support declarative shadow DOM can attach that shadow tree during parse, allowing FAST hydration to reuse it directly.

## Request Flow

```text
Browser GET /
        │
        ▼
actix-web router → render_root(web::Data<AppState>)
        │
        ▼
WebUIHandler::with_plugin(FastV3HydrationPlugin::new())
        │
        ▼
handler.handle(&protocol, &state, &RenderOptions::new("index.html","/"), &mut StringWriter)
        │
        │ — emits HTML with declarative shadow DOM, fe:b content markers, data-fe attrs
        │ — appends <f-template name="hello-world"> ... </f-template> before </body>
        ▼
text/html response

Browser parses HTML, expands declarative shadow DOM
        │
        ▼
/dist/index.js loads
        │
        ▼
enableHydration()  →  declarativeTemplate() resolves <f-template>  →  HelloWorld registered
        │
        ▼
FAST hydrates existing shadow DOM via fe:b / data-fe markers (no DOM rebuild)
```

## End-to-End Summary

The project demonstrates a small but complete integration between Rust server rendering and FAST 3.x client hydration:

- Rust compiles WebUI declarative templates with `Plugin::FastV3`.
- actix-web serves the rendered HTML and the pre-loaded ESM client bundle.
- `FastV3HydrationPlugin` emits FAST 3.x-compatible hydration markers, including `data-fe="N"` slots on elements with client-only event bindings.
- The browser loads `enableHydration()` before registering `<hello-world>`.
- `declarativeTemplate()` connects the server-emitted `<f-template name="hello-world">` to the FAST element definition.
- FAST reuses the server-rendered declarative shadow DOM and re-attaches bindings in place — including wiring the `@click` handler on the `<button>` to `handleButtonPress()`, which calls `alert("button pressed!")`.
- A Playwright suite under `tests/` and a GitHub Actions workflow under `.github/workflows/ci.yml` run the full pipeline end-to-end on every change.

The result is a minimal server-rendered custom element that displays `Hello world` and an interactive **Press me** button immediately in the HTML response, and becomes reactive once the FAST 3.x runtime hydrates it in the browser.
