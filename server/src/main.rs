// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

//! Minimal axum server that renders a FAST 3 component using the WebUI
//! `fast-v3` parser + hydration plugins.
//!
//! At startup it builds the WebUI protocol from `../app/src` and reads the
//! initial state from `../app/data/state.json`. On each GET request it
//! renders HTML with `FastV3HydrationPlugin`. The bundled client script
//! lives at `../app/dist/index.js` and is served as a static file.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde_json::Value;
use tower_http::services::ServeDir;
use webui::{build, BuildOptions, CssStrategy, DomStrategy, Plugin, WebUIProtocol};
use webui_handler::plugin::fast_v3::FastV3HydrationPlugin;
use webui_handler::{RenderOptions, ResponseWriter, WebUIHandler};

/// Shared application state held by axum.
struct AppState {
    protocol: WebUIProtocol,
    state: Value,
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

#[tokio::main]
async fn main() -> Result<()> {
    let app_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("workspace root not found")?
        .join("app");
    let src_dir = app_dir.join("src");
    let state_path = app_dir.join("data").join("state.json");
    let dist_dir = app_dir.join("dist");

    // Build the protocol once with the fast-v3 parser plugin.
    let result = build(BuildOptions {
        app_dir: src_dir,
        entry: "index.html".to_string(),
        css: CssStrategy::Link,
        dom: DomStrategy::Shadow,
        plugin: Some(Plugin::FastV3),
        components: Vec::new(),
    })
    .context("Failed to build WebUI protocol with fast-v3 plugin")?;

    // Load the initial state JSON.
    let state_bytes = std::fs::read(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let state: Value =
        serde_json::from_slice(&state_bytes).context("Failed to parse state.json")?;

    let app_state = Arc::new(AppState {
        protocol: result.protocol,
        state,
    });

    let app = Router::new()
        .route("/", get(render_root))
        .nest_service("/dist", ServeDir::new(dist_dir))
        .with_state(app_state);

    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    println!("Listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn render_root(State(app): State<Arc<AppState>>) -> Response {
    let mut writer = StringWriter(String::with_capacity(4096));
    let handler = WebUIHandler::with_plugin(|| Box::new(FastV3HydrationPlugin::new()));
    let opts = RenderOptions::new("index.html", "/");
    if let Err(err) = handler.handle(&app.protocol, &app.state, &opts, &mut writer) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("render failed: {err}"),
        )
            .into_response();
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(writer.0))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
