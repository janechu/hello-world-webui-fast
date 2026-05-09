// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Minimal actix-web server that renders a FAST 3 component using the
//! WebUI `fast-v3` parser + hydration plugins.
//!
//! At startup it builds the WebUI protocol from `../app/src`, reads the
//! initial state from `../app/data/state.json`, and pre-loads the bundled
//! client script `../app/dist/index.js` into memory. On each GET request
//! it renders HTML with `FastV3HydrationPlugin`.

use std::path::PathBuf;

use actix_web::web::Bytes;
use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::{Context, Result};
use serde_json::Value;
use webui::{
    build, BuildOptions, CssStrategy, DomStrategy, Plugin, ResponseWriter, WebUIHandler,
    WebUIProtocol,
};
use webui_handler::plugin::fast_v3::FastV3HydrationPlugin;
use webui_handler::RenderOptions;

/// Shared application state injected into actix handlers.
struct AppState {
    protocol: WebUIProtocol,
    state: Value,
    index_js: Bytes,
}

/// In-memory `ResponseWriter` that accumulates rendered HTML into a `String`.
struct StringWriter(String);

impl ResponseWriter for StringWriter {
    fn write(&mut self, content: &str) -> webui_handler::Result<()> {
        self.0.push_str(content);
        Ok(())
    }

    fn end(&mut self) -> webui_handler::Result<()> {
        Ok(())
    }
}

async fn render_root(state: web::Data<AppState>) -> HttpResponse {
    let mut writer = StringWriter(String::with_capacity(4096));
    let handler = WebUIHandler::with_plugin(|| Box::new(FastV3HydrationPlugin::new()));
    let opts = RenderOptions::new("index.html", "/");
    if let Err(err) = handler.handle(&state.protocol, &state.state, &opts, &mut writer) {
        return HttpResponse::InternalServerError().body(format!("render failed: {err}"));
    }
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(writer.0)
}

async fn serve_index_js(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/javascript; charset=utf-8")
        .body(state.index_js.clone())
}

fn main() -> Result<()> {
    let app_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("workspace root not found")?
        .join("app");
    let src_dir = app_dir.join("src");
    let state_path = app_dir.join("data").join("state.json");
    let index_js_path = app_dir.join("dist").join("index.js");

    let result = build(BuildOptions {
        app_dir: src_dir,
        entry: "index.html".to_string(),
        css: CssStrategy::Link,
        dom: DomStrategy::Shadow,
        plugin: Some(Plugin::FastV3),
        components: Vec::new(),
    })
    .context("Failed to build WebUI protocol with fast-v3 plugin")?;

    let state_bytes = std::fs::read(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let state: Value =
        serde_json::from_slice(&state_bytes).context("Failed to parse state.json")?;

    let index_js = std::fs::read(&index_js_path).with_context(|| {
        format!(
            "Failed to read {}. Did you run `npm run build` in app/?",
            index_js_path.display()
        )
    })?;

    let app_state = web::Data::new(AppState {
        protocol: result.protocol,
        state,
        index_js: Bytes::from(index_js),
    });

    println!("Listening on http://127.0.0.1:3000");

    actix_web::rt::System::new().block_on(async move {
        HttpServer::new(move || {
            App::new()
                .app_data(app_state.clone())
                .route("/", web::get().to(render_root))
                .route("/dist/index.js", web::get().to(serve_index_js))
        })
        .bind("127.0.0.1:3000")
        .context("Failed to bind 127.0.0.1:3000")?
        .run()
        .await
        .context("Server error")
    })?;

    Ok(())
}
