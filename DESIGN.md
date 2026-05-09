# Hello World WebUI FAST Design

## Overview

`hello-world-webui-fast` is a minimal Rust + TypeScript application that server-renders a single FAST 3 custom element displaying `Hello world`.

The Rust side is a small Axum HTTP server. At startup, it uses `microsoft-webui` 0.0.12, whose library name is `webui`, to compile `app/src/index.html` and component templates into a `WebUIProtocol`. The build uses `Plugin::FastV3`, which selects the FAST 3 parser plugin.

The TypeScript side defines a single FAST Element component, `<hello-world>`, using `@microsoft/fast-element@3.0.0-rc.1`. The component template is authored in WebUI declarative syntax and transformed by the `fast-v3` parser and hydration plugins into FAST 3-compatible `<f-template>` markup with hydration markers such as `<!--fe:b-->`, `<!--fe:/b-->`, and `data-fe="N"`.

At request time, the server walks the compiled protocol with `WebUIHandler` and `FastV3HydrationPlugin`, producing HTML that includes declarative shadow DOM and FAST hydration markers. In the browser, `enableHydration()` and `declarativeTemplate()` allow FAST 3 to attach reactive bindings to the existing server-rendered DOM without rebuilding it.

## File Layout

```text
hello-world-webui-fast/
├── .gitignore
├── Cargo.toml                         # workspace root, members = ["server"]
├── server/
│   ├── Cargo.toml                     # binary "hello-world-webui-fast-server"
│   └── src/main.rs                    # axum entrypoint
├── app/
│   ├── package.json                   # @microsoft/fast-element@3.0.0-rc.1 + esbuild
│   ├── tsconfig.json
│   ├── data/state.json                # { "greeting": "Hello world" }
│   └── src/
│       ├── index.html                 # <hello-world greeting="{{greeting}}"></hello-world>
│       ├── index.ts                   # enableHydration + dynamic import of component
│       └── hello-world/
│           ├── hello-world.ts         # FASTElement w/ @attr greeting + declarativeTemplate()
│           └── hello-world.html       # <template shadowrootmode="open"><h1>{{greeting}}</h1></template>
└── README.md
```

Important paths:

- `server/src/main.rs` contains the Axum server, startup build step, request handler, and `StringWriter` implementation.
- `app/src/index.html` is the server-rendered entry document and hosts `<hello-world greeting="{{greeting}}"></hello-world>`.
- `app/src/index.ts` enables FAST hydration and dynamically imports the component definition.
- `app/src/hello-world/hello-world.ts` defines the `HelloWorld` FAST element with an `@attr greeting` property and `declarativeTemplate()`.
- `app/src/hello-world/hello-world.html` contains the WebUI declarative template for the component shadow DOM.
- `app/data/state.json` provides the render state: `{ "greeting": "Hello world" }`.

## Rust Architecture

The server is a single Axum binary named `hello-world-webui-fast-server`, defined under `server/` and launched with `cargo run` from that directory.

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

The server implements the `webui_handler::ResponseWriter` trait with a `StringWriter` so rendered HTML can be captured into a `String` and returned from the Axum route as `text/html; charset=utf-8`.

## TypeScript Architecture

The client application is intentionally small:

- `app/package.json` depends on `@microsoft/fast-element@3.0.0-rc.1` and `esbuild`.
- `app/src/index.ts` imports `enableHydration()` from `@microsoft/fast-element/hydration.js` and then dynamically imports the component module.
- `app/src/hello-world/hello-world.ts` defines and registers the `<hello-world>` custom element.
- `app/src/hello-world/hello-world.html` provides the WebUI declarative template.

Conceptually, `app/src/index.ts` performs:

```ts
import { enableHydration } from "@microsoft/fast-element/hydration.js";

await enableHydration();
await import("./hello-world/hello-world.js");
```

The component definition uses FAST Element and `declarativeTemplate()`:

```ts
import { FASTElement, attr, customElement } from "@microsoft/fast-element";
import { declarativeTemplate } from "@microsoft/fast-element/hydration.js";

@customElement({
  name: "hello-world",
  template: declarativeTemplate(),
})
export class HelloWorld extends FASTElement {
  @attr greeting = "";
}
```

The template is authored in WebUI declarative syntax, not directly in FAST's final `<f-template>` form:

```html
<template shadowrootmode="open">
  <h1>{{greeting}}</h1>
</template>
```

`FastV3ParserPlugin` converts WebUI declarative syntax into FAST 3 template syntax. In this minimal project there are no `<if>` or `<for>` directives, but the same pipeline would convert:

- `<if condition="...">` into `<f-when>`
- `<for each="...">` into `<f-repeat>`
- `{{...}}` bindings into FAST-compatible template bindings and hydration markers

## Web Layer

The HTTP layer uses Axum 0.8.

There is one dynamic route:

- `GET /` → `render_root`

The route handler:

1. Receives `State<AppState>`.
2. Creates a `StringWriter`.
3. Constructs a per-request `WebUIHandler` with `FastV3HydrationPlugin`.
4. Calls `handler.handle(...)` with the compiled `WebUIProtocol`, render state, `RenderOptions`, and writer.
5. Returns the resulting HTML buffer as `text/html; charset=utf-8`.

Static client assets are served from `app/dist/` using `tower_http::services::ServeDir`, mounted at `/dist`. The bundled browser module is loaded as `/dist/index.js`.

A representative route flow looks like:

```rust
use axum::{extract::State, response::Html};
use webui::WebUIHandler;
use webui_handler::{RenderOptions, ResponseWriter};
use webui_handler::plugin::fast_v3::FastV3HydrationPlugin;

async fn render_root(State(state): State<AppState>) -> Html<String> {
    let mut writer = StringWriter::default();
    let options = RenderOptions::new("index.html", "/");
    let handler = WebUIHandler::with_plugin(|| Box::new(FastV3HydrationPlugin::new()));

    handler
        .handle(&state.protocol, &state.render_state, &options, &mut writer)
        .expect("render root document");

    Html(writer.into_string())
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

1. Axum dispatches the request to `render_root`.
2. `render_root` creates a new `StringWriter`.
3. The handler is constructed with the FAST 3 hydration plugin:

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

While walking protocol fragments, `FastV3HydrationPlugin` emits FAST 3 hydration markers:

- Attribute bindings on elements with bindings receive `data-fe="N"`, where `N` is the binding count.
- Content bindings such as `{{greeting}}` are resolved against `app/data/state.json` and wrapped with `<!--fe:b-->` and `<!--fe:/b-->` markers.
- At `</body>`, an artifact emitted earlier by the parser plugin injects an `<f-template name="hello-world">` block containing the FAST-converted component template.

For this project, the rendered component template is minimal. Because `app/src/hello-world/hello-world.html` contains only a content binding, the emitted FAST template is conceptually:

```html
<f-template name="hello-world">
  <template>
    <h1><!--fe:b-->Hello world<!--fe:/b--></h1>
  </template>
</f-template>
```

The complete response contains:

- The host element from `app/src/index.html`:

  ```html
  <hello-world greeting="Hello world" data-fe="1"></hello-world>
  ```

- Declarative shadow DOM for the component.
- FAST 3 hydration markers in the shadow DOM.
- The `<f-template name="hello-world">` artifact before `</body>`.
- A script reference to `/dist/index.js`.

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
   - `data-fe="N"` attribute binding counters.
8. FAST re-attaches reactive bindings to the existing nodes without creating duplicate DOM.
9. `$fastController.isPrerendered` resolves to `true`.
10. Later property changes, such as `helloWorld.greeting = "Hi"`, update only the marker-bound text node.

The key property of this pipeline is that the server-rendered DOM becomes the live FAST view. The client runtime hydrates it instead of replacing it.

## Design Choices

### `microsoft-webui` 0.0.12 from crates.io

Version 0.0.12 is used because it is the first published release exposing `Plugin::FastV3`. The earlier 0.0.11 release only exposed `Plugin::Fast` and `Plugin::FastV2`. Using the crates.io release keeps this project self-contained and avoids path dependencies on other local working clones.

### `@microsoft/fast-element@3.0.0-rc.1`

`@microsoft/fast-element@3.0.0-rc.1` is the only published 3.x release of FAST Element on npm. It is the runtime targeted by `Plugin::FastV3`, which emits the compact FAST 3 hydration marker format using `fe:b` content markers and `data-fe="N"` attribute counters.

### WebUI declarative syntax

The component template in `app/src/hello-world/hello-world.html` is intentionally authored in WebUI declarative syntax:

```html
<h1>{{greeting}}</h1>
```

This keeps the source template framework-neutral at authoring time. The `FastV3ParserPlugin` performs the FAST-specific conversion, including support for transforming WebUI directives such as `<if>` and `<for>` into FAST 3 `<f-when>` and `<f-repeat>` markup.

### Axum

Axum is a good fit because the WebUI Rust handler documentation includes an Axum example, and Axum 0.8 keeps the server small. It also integrates cleanly with `tower-http`'s `ServeDir` for serving `app/dist/index.js`.

### esbuild

The project uses esbuild because WebUI examples such as `hello-world` and `todo-fast` use it for client bundling. It is fast, has minimal configuration, and emits a single ESM bundle suitable for this small demo.

### Declarative shadow DOM

`DomStrategy::Shadow` and `<template shadowrootmode="open">` allow the server to send shadow DOM in the initial HTML response. Browsers that support declarative shadow DOM can attach that shadow tree during parse, allowing FAST hydration to reuse it directly.

## Request Flow

```text
Browser GET /
        │
        ▼
axum router → render_root(State<AppState>)
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

The project demonstrates the smallest useful integration between Rust server rendering and FAST 3 client hydration:

- Rust compiles WebUI declarative templates with `Plugin::FastV3`.
- Axum serves the rendered HTML and bundled ESM client.
- `FastV3HydrationPlugin` emits FAST 3-compatible hydration markers.
- The browser loads `enableHydration()` before registering `<hello-world>`.
- `declarativeTemplate()` connects the server-emitted `<f-template name="hello-world">` to the FAST element definition.
- FAST reuses the server-rendered declarative shadow DOM and re-attaches bindings in place.

The result is a minimal server-rendered custom element that displays `Hello world` immediately in the HTML response and becomes reactive once the FAST 3 runtime hydrates it in the browser.
