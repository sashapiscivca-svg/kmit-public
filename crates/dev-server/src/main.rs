//! Настільний сервер для перевірки інтерфейсу в браузері. Слухає лише 127.0.0.1, без залежностей поза
//! стандартною бібліотекою. Статичні файли інтерфейсу читає з теки (`--ui`), а `POST /api/<команда>` передає
//! ядру застосунку (`app-core`). Це не продуктова оболонка: продукт на телефоні працює через Tauri.

use app_core::{App, Ctx};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;
use storage::crypto::KdfParams;

const MAX_BODY: usize = 64 * 1024 * 1024;

struct Opts {
    port: u16,
    data: PathBuf,
    ui: PathBuf,
}

fn parse_args() -> Opts {
    let mut o = Opts {
        port: 8765,
        data: PathBuf::from("dev-data"),
        ui: PathBuf::from("app/ui"),
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--port" => o.port = it.next().and_then(|v| v.parse().ok()).unwrap_or(o.port),
            "--data" => o.data = it.next().map(PathBuf::from).unwrap_or(o.data),
            "--ui" => o.ui = it.next().map(PathBuf::from).unwrap_or(o.ui),
            other => eprintln!("невідомий параметр {other}"),
        }
    }
    o
}

fn kdf() -> KdfParams {
    #[cfg(feature = "fast-kdf")]
    {
        KdfParams::fast_for_tests()
    }
    #[cfg(not(feature = "fast-kdf"))]
    {
        KdfParams::production()
    }
}

struct Request {
    method: String,
    path: String,
    host: String,
    origin: Option<String>,
    json: bool,
    body: Vec<u8>,
}

fn read_request(s: &mut TcpStream) -> Option<Request> {
    s.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let head_end = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        if buf.len() > 64 * 1024 {
            return None;
        }
        let n = s.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let (mut host, mut len) = (String::new(), 0usize);
    let (mut origin, mut json) = (None, false);
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "host" => host = v.trim().to_string(),
                "content-length" => len = v.trim().parse().unwrap_or(0),
                "origin" => origin = Some(v.trim().to_string()),
                "content-type" => json = v.trim().starts_with("application/json"),
                _ => {}
            }
        }
    }
    if len > MAX_BODY {
        return None;
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < len {
        let n = s.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    Some(Request {
        method,
        path,
        host,
        origin,
        json,
        body,
    })
}

fn respond(s: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
         Content-Security-Policy: default-src 'self'; style-src 'self'; script-src 'self'; img-src 'self' data:; \
         connect-src 'self'; object-src 'none'; frame-ancestors 'none'\r\n\
         X-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body);
}

fn ctype_of(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

fn serve_static(s: &mut TcpStream, ui: &Path, path: &str) {
    let rel = path.split(['?', '#']).next().unwrap_or("/");
    let rel = if rel == "/" { "/index.html" } else { rel };
    let clean: Vec<&str> = rel.split('/').filter(|c| !c.is_empty()).collect();
    if clean
        .iter()
        .any(|c| *c == ".." || c.contains('\\') || c.starts_with('.'))
    {
        return respond(s, "400 Bad Request", "text/plain", b"bad path");
    }
    let mut file = ui.to_path_buf();
    clean.iter().for_each(|c| file.push(c));
    match std::fs::read(&file) {
        Ok(bytes) => respond(s, "200 OK", ctype_of(&file), &bytes),
        Err(_) => respond(s, "404 Not Found", "text/plain", b"not found"),
    }
}

fn handle_api(app: &mut App, name: &str, body: &[u8]) -> Value {
    let v: Value = serde_json::from_slice(body).unwrap_or_else(|_| json!({}));
    let args = v.get("args").cloned().unwrap_or_else(|| json!({}));
    let ctx = Ctx::from_json(v.get("ctx").unwrap_or(&Value::Null));
    match app.call(name, &args, &ctx) {
        Ok(v) => v,
        Err(e) => e.to_json(),
    }
}

/// Запит до API приймається лише від нашої сторінки: заголовок `Origin` (якщо є) має вказувати на наш хост,
/// а тіло має бути JSON (чужа сторінка не може надіслати такий запит без попереднього погодження, якого сервер не дає).
fn origin_ok(origin: &Option<String>, port: u16) -> bool {
    match origin {
        None => true,
        Some(o) => {
            o == &format!("http://127.0.0.1:{port}") || o == &format!("http://localhost:{port}")
        }
    }
}

fn host_ok(host: &str) -> bool {
    let h = host.rsplit_once(':').map_or(host, |(h, _)| h);
    h == "127.0.0.1" || h == "localhost"
}

fn main() {
    let o = parse_args();
    std::fs::create_dir_all(&o.data).expect("не вдалося створити теку даних");
    let mut app = App::new(&o.data, kdf(), None);
    let listener = TcpListener::bind(("127.0.0.1", o.port)).expect("порт зайнятий");
    eprintln!(
        "Відкрий у браузері: http://127.0.0.1:{}/  (дані: {}, інтерфейс: {})",
        o.port,
        o.data.display(),
        o.ui.display()
    );
    for stream in listener.incoming() {
        let Ok(mut s) = stream else { continue };
        let Some(req) = read_request(&mut s) else {
            continue;
        };
        if !host_ok(&req.host) {
            respond(&mut s, "403 Forbidden", "text/plain", b"forbidden host");
            continue;
        }
        if let Some(cmd) = req.path.strip_prefix("/api/") {
            if !req.json || !origin_ok(&req.origin, o.port) {
                respond(&mut s, "403 Forbidden", "text/plain", b"forbidden origin");
                continue;
            }
            if req.method != "POST" {
                respond(&mut s, "405 Method Not Allowed", "text/plain", b"POST only");
                continue;
            }
            let out = handle_api(&mut app, cmd, &req.body);
            respond(
                &mut s,
                "200 OK",
                "application/json; charset=utf-8",
                out.to_string().as_bytes(),
            );
        } else if cfg!(feature = "e2e") && req.path == "/__peek" {
            #[cfg(feature = "e2e")]
            {
                let a = app.peek_analog_answer().or_else(|| app.peek_answer());
                respond(
                    &mut s,
                    "200 OK",
                    "application/json",
                    json!({"answer": a}).to_string().as_bytes(),
                );
            }
        } else if req.method == "GET" {
            serve_static(&mut s, &o.ui, &req.path);
        } else {
            respond(&mut s, "405 Method Not Allowed", "text/plain", b"GET only");
        }
    }
}
