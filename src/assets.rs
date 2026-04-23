//! `asset://` custom protocol handler. Serves a local directory of static
//! assets (the Vite `dist/` output) with basic mime detection.
//!
//! Also serves allow-listed local files under the reserved prefix
//! `__file/<percent-encoded-absolute-path>`. The frontend builds such URLs
//! via [`window.__shell_asset_url(path)`][js] and the Rust helper
//! [`crate::asset_url_from_file`]. Files are only served when their
//! canonical path is in the [`crate::FsState`] allow-list; otherwise 403.
//!
//! [js]: crate#frontend-bridge

use std::path::{Path, PathBuf};
use std::sync::Arc;

use wry::http::Response;

use crate::FsState;

pub(crate) fn serve(root: &Path, fs: Option<&Arc<FsState>>, req_path: &str) -> Response<Vec<u8>> {
    let rel = req_path.trim_start_matches('/');

    if let Some(encoded) = rel.strip_prefix("__file/") {
        return serve_sandboxed_file(fs, encoded);
    }

    let rel = if rel.is_empty() { "index.html" } else { rel };
    let path = root.join(rel);
    let path = if path.is_dir() {
        path.join("index.html")
    } else {
        path
    };

    // Refuse any path that escapes `root` after resolution. Guards against
    // `asset://localhost/../../etc/passwd` style traversal.
    if let Ok(canonical) = std::fs::canonicalize(&path) {
        if let Ok(canonical_root) = std::fs::canonicalize(root) {
            if !canonical.starts_with(&canonical_root) {
                return not_found();
            }
        }
    }

    match std::fs::read(&path) {
        Ok(body) => Response::builder()
            .header("Content-Type", mime_for(&path))
            .header("Access-Control-Allow-Origin", "*")
            .body(body)
            .unwrap(),
        Err(_) => not_found(),
    }
}

fn serve_sandboxed_file(fs: Option<&Arc<FsState>>, encoded: &str) -> Response<Vec<u8>> {
    let Some(fs) = fs else {
        return forbidden();
    };
    let Ok(decoded) = percent_decode(encoded) else {
        return forbidden();
    };
    let Ok(canonical) = crate::is_path_allowed(&decoded, &fs.allowed_paths, &fs.allowed_dirs)
    else {
        return forbidden();
    };
    let path = PathBuf::from(canonical);
    match std::fs::read(&path) {
        Ok(body) => Response::builder()
            .header("Content-Type", mime_for(&path))
            .header("Access-Control-Allow-Origin", "*")
            .body(body)
            .unwrap(),
        Err(_) => not_found(),
    }
}

fn percent_decode(s: &str) -> Result<String, ()> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' {
            if i + 2 >= bytes.len() {
                return Err(());
            }
            let hi = hex_digit(bytes[i + 1])?;
            let lo = hex_digit(bytes[i + 2])?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
}

fn hex_digit(b: u8) -> Result<u8, ()> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(()),
    }
}

fn forbidden() -> Response<Vec<u8>> {
    Response::builder()
        .status(403)
        .body(b"forbidden".to_vec())
        .unwrap()
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(404)
        .body(b"not found".to_vec())
        .unwrap()
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "wasm" => "application/wasm",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

pub(crate) fn default_root() -> PathBuf {
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> tempfile::TempDir {
        let home = std::env::var("HOME").expect("HOME");
        let base = PathBuf::from(home).join(".fude-test-scratch");
        std::fs::create_dir_all(&base).unwrap();
        tempfile::TempDir::new_in(&base).unwrap()
    }

    #[test]
    fn mime_detection_covers_common_types() {
        assert_eq!(mime_for(Path::new("x.html")), "text/html; charset=utf-8");
        assert_eq!(
            mime_for(Path::new("x.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            mime_for(Path::new("x.mjs")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(mime_for(Path::new("x.css")), "text/css; charset=utf-8");
        assert_eq!(mime_for(Path::new("x.json")), "application/json");
        assert_eq!(mime_for(Path::new("x.svg")), "image/svg+xml");
        assert_eq!(mime_for(Path::new("x.wasm")), "application/wasm");
        assert_eq!(mime_for(Path::new("x.woff2")), "font/woff2");
        assert_eq!(
            mime_for(Path::new("x.unknownext")),
            "application/octet-stream"
        );
        assert_eq!(
            mime_for(Path::new("no-extension")),
            "application/octet-stream"
        );
    }

    #[test]
    fn serves_index_html_for_root() {
        let dir = scratch();
        std::fs::write(dir.path().join("index.html"), b"<!doctype html>hi").unwrap();
        let resp = serve(dir.path(), None, "/");
        assert_eq!(resp.status(), 200);
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/html"));
        assert_eq!(resp.body(), b"<!doctype html>hi");
    }

    #[test]
    fn serves_index_html_for_directory_path() {
        let dir = scratch();
        let sub = dir.path().join("docs");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("index.html"), b"docs index").unwrap();
        let resp = serve(dir.path(), None, "/docs");
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), b"docs index");
    }

    #[test]
    fn serves_existing_file_with_detected_mime() {
        let dir = scratch();
        std::fs::write(dir.path().join("app.js"), b"console.log(1)").unwrap();
        let resp = serve(dir.path(), None, "/app.js");
        assert_eq!(resp.status(), 200);
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("application/javascript"));
    }

    #[test]
    fn returns_404_for_missing_file() {
        let dir = scratch();
        let resp = serve(dir.path(), None, "/does-not-exist.js");
        assert_eq!(resp.status(), 404);
    }

    #[test]
    fn rejects_path_traversal() {
        // Outside-root file that *does* exist should not leak through `..`
        // after canonicalization.
        let dir = scratch();
        std::fs::write(dir.path().join("index.html"), b"ok").unwrap();
        let sibling_dir = scratch();
        let secret = sibling_dir.path().join("secret.txt");
        std::fs::write(&secret, b"TOPSECRET").unwrap();

        // Relative path that tries to escape the root.
        let rel = format!(
            "/../{}/secret.txt",
            sibling_dir.path().file_name().unwrap().to_string_lossy()
        );
        let resp = serve(dir.path(), None, &rel);
        // Either 404 (traversal blocked) or the body doesn't match secret.
        assert_ne!(resp.body(), b"TOPSECRET");
        assert!(resp.status() == 404 || resp.body() != b"TOPSECRET");
    }

    #[test]
    fn adds_cors_header() {
        let dir = scratch();
        std::fs::write(dir.path().join("a.json"), b"{}").unwrap();
        let resp = serve(dir.path(), None, "/a.json");
        assert_eq!(
            resp.headers().get("Access-Control-Allow-Origin").unwrap(),
            "*"
        );
    }

    // -- percent_decode --

    #[test]
    fn percent_decode_plain_ascii() {
        assert_eq!(percent_decode("hello").unwrap(), "hello");
    }

    #[test]
    fn percent_decode_spaces_and_slashes() {
        assert_eq!(
            percent_decode("%2FUsers%2Falice%2Fhello%20world.md").unwrap(),
            "/Users/alice/hello world.md"
        );
    }

    #[test]
    fn percent_decode_lowercase_hex() {
        assert_eq!(percent_decode("%2f%2a").unwrap(), "/*");
    }

    #[test]
    fn percent_decode_rejects_incomplete_escape() {
        assert!(percent_decode("%2").is_err());
        assert!(percent_decode("foo%").is_err());
    }

    #[test]
    fn percent_decode_rejects_bad_hex() {
        assert!(percent_decode("%ZZ").is_err());
        assert!(percent_decode("%2G").is_err());
    }

    // -- __file/ protocol --

    use crate::{new_list, safe_lock};

    fn fs_with_allowed_path(p: &Path) -> std::sync::Arc<FsState> {
        let fs = std::sync::Arc::new(FsState {
            allowed_paths: new_list(),
            allowed_dirs: new_list(),
        });
        let canonical = std::fs::canonicalize(p)
            .unwrap()
            .to_string_lossy()
            .to_string();
        safe_lock(&fs.allowed_paths).push(canonical);
        fs
    }

    #[test]
    fn file_protocol_requires_fs_state() {
        let dir = scratch();
        let resp = serve(dir.path(), None, "/__file/%2Fetc%2Fpasswd");
        assert_eq!(resp.status(), 403);
    }

    #[test]
    fn file_protocol_rejects_non_allow_listed() {
        let dir = scratch();
        let other = scratch();
        let secret = other.path().join("secret.md");
        std::fs::write(&secret, b"nope").unwrap();
        let fs = std::sync::Arc::new(FsState {
            allowed_paths: new_list(),
            allowed_dirs: new_list(),
        });
        let encoded = percent_encode(&secret.to_string_lossy());
        let resp = serve(dir.path(), Some(&fs), &format!("/__file/{}", encoded));
        assert_eq!(resp.status(), 403);
    }

    #[test]
    fn file_protocol_serves_allow_listed_file() {
        let dir = scratch();
        let target_dir = scratch();
        let target = target_dir.path().join("hello.md");
        std::fs::write(&target, b"# hello").unwrap();
        let fs = fs_with_allowed_path(&target);
        let encoded = percent_encode(&target.to_string_lossy());
        let resp = serve(dir.path(), Some(&fs), &format!("/__file/{}", encoded));
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), b"# hello");
        assert!(
            resp.headers()
                .get("Content-Type")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("application/octet-stream")
                || resp
                    .headers()
                    .get("Content-Type")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("text/")
        );
    }

    #[test]
    fn file_protocol_detects_mime_from_extension() {
        let dir = scratch();
        let target_dir = scratch();
        let target = target_dir.path().join("pic.png");
        std::fs::write(&target, b"\x89PNG\r\n\x1a\n").unwrap();
        let fs = fs_with_allowed_path(&target);
        let encoded = percent_encode(&target.to_string_lossy());
        let resp = serve(dir.path(), Some(&fs), &format!("/__file/{}", encoded));
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers()
                .get("Content-Type")
                .unwrap()
                .to_str()
                .unwrap(),
            "image/png"
        );
    }

    // minimal percent-encoder for tests only
    fn percent_encode(s: &str) -> String {
        let mut out = String::new();
        for b in s.as_bytes() {
            match *b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    out.push(*b as char)
                }
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }
}
