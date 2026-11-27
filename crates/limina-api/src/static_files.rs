//! Serves the built web app (`STATIC_DIR`) next to the API, for one process
//! behind a proxy. Every path that is not a file and does not look like one
//! gets `index.html` (the app's client-side routes, such as `/laboratory`).
//! Paths under `/v1/` are never answered with the page: an unknown API route
//! stays a JSON 404.
use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::http::{header, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Router;

/// The page every client-side route is answered with.
pub const INDEX: &str = "index.html";

/// The router with the web app behind it.
pub fn with_static_dir(router: Router, dir: PathBuf) -> Router {
    router.fallback(move |method: Method, uri: Uri| {
        let dir = dir.clone();
        async move { serve(&dir, &method, uri.path()).await }
    })
}

/// `STATIC_DIR` when it is set to a folder that holds `index.html`.
pub fn static_dir_from_env() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var("STATIC_DIR").ok()?.trim());
    dir.join(INDEX).is_file().then_some(dir)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        "pdf" => "application/pdf",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// The request path as a relative path made of plain names only; `None` for
/// anything that could leave the folder (`..`, a drive, a backslash, a NUL).
pub fn safe_relative(path: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        if part == ".." || part.contains(['\\', '\0', ':']) {
            return None;
        }
        out.push(part);
    }
    out.components()
        .all(|c| matches!(c, Component::Normal(_)))
        .then_some(out)
}

fn not_found() -> Response {
    let body = r#"{"error":{"code":"NotFound","message":"no such route"}}"#;
    let mut r = Response::new(Body::from(body));
    *r.status_mut() = StatusCode::NOT_FOUND;
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    r
}

fn ok(file: &Path, bytes: Vec<u8>, head: bool) -> Response {
    let is_index = file.file_name().is_some_and(|n| n == INDEX);
    let mut r = Response::new(if head {
        Body::empty()
    } else {
        Body::from(bytes)
    });
    let h = r.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(file)),
    );
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if is_index {
            "no-cache"
        } else {
            "public, max-age=3600"
        }),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    r
}

/// A file of `dir` that exists, is a regular file and stays inside `dir` once links are resolved.
async fn read_inside(dir: &Path, relative: &Path) -> Option<(PathBuf, Vec<u8>)> {
    let root = tokio::fs::canonicalize(dir).await.ok()?;
    let file = tokio::fs::canonicalize(dir.join(relative)).await.ok()?;
    if !file.starts_with(&root) || !tokio::fs::metadata(&file).await.ok()?.is_file() {
        return None;
    }
    let bytes = tokio::fs::read(&file).await.ok()?;
    Some((file, bytes))
}

async fn serve(dir: &Path, method: &Method, path: &str) -> Response {
    if path == "/v1" || path.starts_with("/v1/") {
        return not_found();
    }
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let head = method == Method::HEAD;
    let Some(relative) = safe_relative(path) else {
        return not_found();
    };
    if !relative.as_os_str().is_empty() {
        if let Some((file, bytes)) = read_inside(dir, &relative).await {
            return ok(&file, bytes, head);
        }
        // A missing file (it has an extension) is a 404, not the page.
        if relative.extension().is_some() {
            return not_found();
        }
    }
    match read_inside(dir, Path::new(INDEX)).await {
        Some((file, bytes)) => ok(&file, bytes, head),
        None => not_found(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_names_stay_inside_the_folder() {
        assert_eq!(safe_relative("/"), Some(PathBuf::new()));
        assert_eq!(
            safe_relative("/a/b.js"),
            Some(PathBuf::from("a").join("b.js"))
        );
        for bad in [
            "/../secret",
            "/a/../../b",
            "/a\\b",
            "/C:/windows",
            "/a/\0b",
            "/..",
            "/a/..",
        ] {
            assert_eq!(safe_relative(bad), None, "{bad}");
        }
    }

    #[test]
    fn files_get_their_type() {
        assert_eq!(
            content_type(Path::new("main-ABC.js")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            content_type(Path::new("styles.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            content_type(Path::new("x.unknown")),
            "application/octet-stream"
        );
    }
}
