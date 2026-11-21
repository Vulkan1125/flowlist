//! Mobile access: a small HTTP server over the same library as the GUI, CLI
//! and MCP server.
//!
//! The Android app is a client: it caches a snapshot, queues operations while
//! offline and replays them here. Every operation is a [`Command`], so the
//! phone gets the same validation, repeats, undo history and file locking as
//! every other surface. The protocol is deliberately plain JSON over HTTP;
//! there is no cloud and no account.

pub mod agent;

use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use flowlist_core::command::Command;
use flowlist_core::paths::DataPaths;
use flowlist_core::store::Store;

pub const DEFAULT_PORT: u16 = 7777;
pub const DEFAULT_BIND: &str = "0.0.0.0";
const MAX_BODY: usize = 2 * 1024 * 1024;
const MAX_UPLOAD: usize = 40 * 1024 * 1024;
const RECENT_OPS: usize = 500;
const PAIR_TTL: Duration = Duration::from_secs(10 * 60);
const PAIR_ATTEMPTS: usize = 10;
const PAIR_WINDOW: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// Files

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub token: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MobileFile {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub devices: Vec<Device>,
    /// Operation ids already applied. Replaying an id is a no-op, so a phone
    /// that retries after a timeout cannot capture the same task twice.
    #[serde(default)]
    pub recent_ops: Vec<String>,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

impl Default for MobileFile {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            devices: Vec::new(),
            recent_ops: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServeInfo {
    pub pid: u32,
    pub port: u16,
    pub started_at: String,
    pub pairing_code: String,
    pub pairing_expires_at: i64,
    pub version: String,
}

pub fn mobile_path(root: &Path) -> PathBuf {
    root.join("mobile.json")
}

pub fn info_path(root: &Path) -> PathBuf {
    root.join("serve.json")
}

pub fn load_mobile(root: &Path) -> MobileFile {
    std::fs::read_to_string(mobile_path(root))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_mobile(root: &Path, file: &MobileFile) -> Result<()> {
    std::fs::create_dir_all(root)?;
    let path = mobile_path(root);
    let text = serde_json::to_string_pretty(file)?;
    std::fs::write(&path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// A reachable server, if one is running for this library.
pub fn running(root: &Path) -> Option<ServeInfo> {
    let text = std::fs::read_to_string(info_path(root)).ok()?;
    let info: ServeInfo = serde_json::from_str(&text).ok()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], info.port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(250)).ok()?;
    Some(info)
}

/// The addresses a phone can use. Tailscale appears as a `utun` interface, so
/// it is included without any special handling.
pub fn addresses() -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(output) = Process::new("/sbin/ifconfig").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("inet ") else {
                continue;
            };
            let Some(address) = rest.split_whitespace().next() else {
                continue;
            };
            if address == "127.0.0.1" || address.starts_with("169.254.") {
                continue;
            }
            found.push(address.to_string());
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Server

#[derive(Clone, Debug)]
pub struct Options {
    pub port: u16,
    pub bind: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            bind: DEFAULT_BIND.to_string(),
        }
    }
}

struct Shared {
    paths: DataPaths,
    port: u16,
    started_at: String,
    pairing: Mutex<Pairing>,
}

struct Pairing {
    code: String,
    expires: Instant,
    attempts: Vec<Instant>,
}

pub struct Running {
    root: PathBuf,
    addr: SocketAddr,
    info: ServeInfo,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Running {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn info(&self) -> &ServeInfo {
        &self.info
    }

    pub fn wait(&self) {
        while !self.stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(250));
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_file(info_path(&self.root));
    }
}

pub fn start(paths: &DataPaths, options: Options) -> Result<Running> {
    paths.ensure()?;
    // Fail early on a corrupt library rather than serving surprises.
    Store::open(paths.clone()).context("opening the library")?;

    let port = options.port;
    let listener = TcpListener::bind((options.bind.as_str(), port))
        .with_context(|| format!("binding {}:{port}", options.bind))?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;

    let pairing = Mutex::new(Pairing {
        code: new_code(),
        expires: Instant::now() + PAIR_TTL,
        attempts: Vec::new(),
    });
    let started_at = now().to_rfc3339();
    let shared = Arc::new(Shared {
        paths: paths.clone(),
        port: addr.port(),
        started_at: started_at.clone(),
        pairing,
    });

    let info = ServeInfo {
        pid: std::process::id(),
        port: addr.port(),
        started_at,
        pairing_code: shared.pairing_code(),
        pairing_expires_at: expiry_epoch(PAIR_TTL),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    write_info(&paths.root, &info)?;

    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let thread_shared = shared.clone();
    let thread = thread::Builder::new()
        .name("flowlist-serve".to_string())
        .spawn(move || accept_loop(listener, thread_shared, thread_stop))
        .context("starting the server thread")?;

    Ok(Running {
        root: paths.root.clone(),
        addr,
        info,
        stop,
        thread: Some(thread),
    })
}

fn accept_loop(listener: TcpListener, shared: Arc<Shared>, stop: Arc<AtomicBool>) {
    let mut last_tick = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        if last_tick.elapsed() >= Duration::from_secs(1) {
            shared.pairing_code();
            last_tick = Instant::now();
        }
        match listener.accept() {
            Ok((stream, peer)) => {
                let shared = shared.clone();
                thread::spawn(move || {
                    let _ = handle(stream, peer, shared);
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    }
}

fn write_info(root: &Path, info: &ServeInfo) -> Result<()> {
    let text = serde_json::to_string_pretty(info)?;
    std::fs::write(info_path(root), text)?;
    Ok(())
}

impl Shared {
    fn pairing_code(&self) -> String {
        let (code, expires, rotated) = {
            let mut pairing = self.pairing.lock().expect("pairing lock");
            let mut rotated = false;
            if pairing.expires <= Instant::now() {
                pairing.code = new_code();
                pairing.expires = Instant::now() + PAIR_TTL;
                rotated = true;
            }
            (pairing.code.clone(), pairing.expires, rotated)
        };
        if rotated {
            self.persist_pairing(&code, expires);
        }
        code
    }

    /// The pairing code is also shown in Settings, which reads `serve.json`
    /// rather than talking to this process. Write it whenever it changes.
    fn persist_pairing(&self, code: &str, expires: Instant) {
        let remaining = expires.saturating_duration_since(Instant::now());
        let info = ServeInfo {
            pid: std::process::id(),
            port: self.port,
            started_at: self.started_at.clone(),
            pairing_code: code.to_string(),
            pairing_expires_at: expiry_epoch(remaining),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        let _ = write_info(&self.paths.root, &info);
    }

    fn rotate_pairing(&self) {
        let (code, expires) = {
            let mut pairing = self.pairing.lock().expect("pairing lock");
            pairing.code = new_code();
            pairing.expires = Instant::now() + PAIR_TTL;
            (pairing.code.clone(), pairing.expires)
        };
        self.persist_pairing(&code, expires);
    }

    fn pairing_seconds(&self) -> u64 {
        let pairing = self.pairing.lock().expect("pairing lock");
        pairing
            .expires
            .saturating_duration_since(Instant::now())
            .as_secs()
    }

    fn allow_attempt(&self) -> bool {
        let mut pairing = self.pairing.lock().expect("pairing lock");
        let now = Instant::now();
        pairing
            .attempts
            .retain(|attempt| now.duration_since(*attempt) < PAIR_WINDOW);
        if pairing.attempts.len() >= PAIR_ATTEMPTS {
            return false;
        }
        pairing.attempts.push(now);
        true
    }

    fn check_code(&self, code: &str) -> bool {
        let actual = self.pairing_code();
        constant_eq(actual.as_bytes(), code.trim().as_bytes())
    }
}

fn expiry_epoch(ttl: Duration) -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        + ttl.as_secs() as i64
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

// ---------------------------------------------------------------------------
// HTTP

struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn query(&self, key: &str) -> Option<String> {
        let (_, query) = self.path.split_once('?')?;
        for pair in query.split('&') {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            if name == key {
                return Some(percent_decode(value));
            }
        }
        None
    }

    fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, String> {
        serde_json::from_slice(&self.body).map_err(|error| format!("invalid JSON: {error}"))
    }
}

struct Response {
    status: u16,
    body: Vec<u8>,
    content_type: &'static str,
}

impl Response {
    fn json(status: u16, value: impl Serialize) -> Self {
        Self {
            status,
            body: serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec()),
            content_type: "application/json",
        }
    }

    fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(status, json!({ "error": message.into() }))
    }

    fn text(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            body: message.into().into_bytes(),
            content_type: "text/plain; charset=utf-8",
        }
    }

    fn bytes(status: u16, body: Vec<u8>, content_type: &'static str) -> Self {
        Self {
            status,
            body,
            content_type,
        }
    }
}

fn handle(mut stream: TcpStream, peer: SocketAddr, shared: Arc<Shared>) -> Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    let Some(request) = read_request(&mut stream)? else {
        return Ok(());
    };
    let response = route(&request, peer, &shared);
    write_response(&mut stream, response)?;
    Ok(())
}

fn read_request(stream: &mut TcpStream) -> Result<Option<Request>> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        match stream.read(&mut chunk) {
            Ok(0) => return Ok(None),
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(position) = find_bytes(&buffer, b"\r\n\r\n") {
                    break position;
                }
                if buffer.len() > 64 * 1024 {
                    return Ok(None);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    };

    let head = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let Some(request_line) = lines.next() else {
        return Ok(None);
    };
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = if path.starts_with("/api/v1/files") {
        MAX_UPLOAD
    } else {
        MAX_BODY
    };
    if length > limit {
        return Ok(None);
    }

    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < length {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => body.extend_from_slice(&chunk[..read]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
    body.truncate(length);

    Ok(Some(Request {
        method,
        path,
        headers,
        body,
    }))
}

fn write_response(stream: &mut TcpStream, response: Response) -> Result<()> {
    let reason = match response.status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        423 => "Locked",
        429 => "Too Many Requests",
        _ => "Server Error",
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()?;
    Ok(())
}

fn route(request: &Request, peer: SocketAddr, shared: &Shared) -> Response {
    let path = request.path.split('?').next().unwrap_or("/");
    match (request.method.as_str(), path) {
        ("GET", "/") => Response::text(200, "Flowlist mobile server. Try GET /api/v1/info\n"),
        ("GET", "/api/v1/info") => {
            let revision = Store::open(shared.paths.clone())
                .map(|store| store.revision())
                .unwrap_or(0);
            Response::json(
                200,
                json!({
                    "name": "Flowlist",
                    "version": env!("CARGO_PKG_VERSION"),
                    "schema_version": flowlist_core::persist::SCHEMA_VERSION,
                    "revision": revision,
                    "port": shared.port,
                }),
            )
        }
        // Only a process on this Mac may read the current pairing code.
        ("GET", "/api/v1/pair") => {
            if !peer.ip().is_loopback() {
                return Response::error(404, "not found");
            }
            let code = shared.pairing_code();
            Response::json(
                200,
                json!({
                    "code": code,
                    "expires_in": shared.pairing_seconds(),
                    "port": shared.port,
                    "addresses": addresses(),
                }),
            )
        }
        ("POST", "/api/v1/pair") => pair(request, shared),
        ("POST", "/api/v1/unpair") => {
            let token = match authorize(request, shared) {
                Ok(token) => token,
                Err(response) => return response,
            };
            unpair(shared, &token)
        }
        ("GET", "/api/v1/snapshot") => {
            if let Err(response) = authorize(request, shared) {
                return response;
            }
            snapshot_response(shared)
        }
        ("POST", "/api/v1/ops") => {
            if let Err(response) = authorize(request, shared) {
                return response;
            }
            ops(request, shared)
        }
        ("GET", "/api/v1/events") => {
            if let Err(response) = authorize(request, shared) {
                return response;
            }
            let since = request
                .query("since")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0);
            events(shared, since)
        }
        ("POST", "/api/v1/files") => {
            if let Err(response) = authorize(request, shared) {
                return response;
            }
            upload_file(request, shared)
        }
        ("GET", "/api/v1/files") => {
            if let Err(response) = authorize(request, shared) {
                return response;
            }
            download_file(request, shared)
        }
        ("OPTIONS", _) => Response::text(204, ""),
        _ => Response::error(404, "not found"),
    }
}

fn upload_file(request: &Request, shared: &Shared) -> Response {
    if request.body.is_empty() {
        return Response::error(400, "empty file");
    }
    let raw = request.query("name").unwrap_or_else(|| "attachment".to_string());
    let name = safe_file_name(&raw);
    let dir = shared.paths.root.join("attachments");
    if let Err(error) = std::fs::create_dir_all(&dir) {
        return Response::error(500, format!("could not create the attachments folder: {error}"));
    }
    let mut path = dir.join(&name);
    if path.exists() {
        let stem = path
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "attachment".to_string());
        let extension = path
            .extension()
            .map(|value| format!(".{}", value.to_string_lossy()))
            .unwrap_or_default();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0);
        path = dir.join(format!("{stem}-{stamp}{extension}"));
    }
    if let Err(error) = std::fs::write(&path, &request.body) {
        return Response::error(500, format!("could not save the file: {error}"));
    }
    Response::json(
        200,
        json!({
            "path": path.to_string_lossy(),
            "name": name,
            "size": request.body.len(),
        }),
    )
}

fn download_file(request: &Request, shared: &Shared) -> Response {
    let Some(requested) = request.query("path") else {
        return Response::error(400, "missing path");
    };
    let Ok(target) = std::fs::canonicalize(&requested) else {
        return Response::error(404, "file not found");
    };
    let upload_dir = std::fs::canonicalize(shared.paths.root.join("attachments")).ok();
    let in_upload_dir = upload_dir
        .as_ref()
        .is_some_and(|dir| target.starts_with(dir));
    let listed = attachment_paths(shared)
        .iter()
        .filter_map(|path| std::fs::canonicalize(path).ok())
        .any(|allowed| allowed == target);
    if !in_upload_dir && !listed {
        return Response::error(404, "file not found");
    }
    match std::fs::read(&target) {
        Ok(bytes) => Response::bytes(200, bytes, mime_for(&target)),
        Err(error) => Response::error(500, format!("could not read the file: {error}")),
    }
}

fn attachment_paths(shared: &Shared) -> Vec<PathBuf> {
    let Ok(store) = Store::open(shared.paths.clone()) else {
        return Vec::new();
    };
    let db = &store.data().db;
    let mut paths = Vec::new();
    for task in db.tasks.values() {
        paths.extend(task.attachments.iter().map(PathBuf::from));
    }
    for project in db.projects.values() {
        paths.extend(project.attachments.iter().map(PathBuf::from));
    }
    paths
}

fn safe_file_name(raw: &str) -> String {
    let base = Path::new(raw)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "attachment".to_string());
    let mut cleaned: String = base
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ' ') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    cleaned = cleaned.trim_matches('.').trim().to_string();
    if cleaned.is_empty() {
        "attachment".to_string()
    } else {
        cleaned.chars().take(120).collect()
    }
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain; charset=utf-8",
        "json" => "application/json",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

fn authorize(request: &Request, shared: &Shared) -> Result<String, Response> {
    let token = request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(token) = token else {
        return Err(Response::error(401, "missing bearer token"));
    };
    let mobile = load_mobile(&shared.paths.root);
    let known = mobile
        .devices
        .iter()
        .any(|device| constant_eq(device.token.as_bytes(), token.as_bytes()));
    if known {
        Ok(token.to_string())
    } else {
        Err(Response::error(401, "unknown token; pair again"))
    }
}

fn unpair(shared: &Shared, token: &str) -> Response {
    let mut mobile = load_mobile(&shared.paths.root);
    let before = mobile.devices.len();
    mobile
        .devices
        .retain(|device| !constant_eq(device.token.as_bytes(), token.as_bytes()));
    if let Err(error) = save_mobile(&shared.paths.root, &mobile) {
        return Response::error(500, format!("could not save the device list: {error}"));
    }
    Response::json(
        200,
        json!({
            "unpaired": before != mobile.devices.len(),
            "devices": mobile.devices.len(),
        }),
    )
}

fn pair(request: &Request, shared: &Shared) -> Response {
    if !shared.allow_attempt() {
        return Response::error(429, "too many pairing attempts; wait a minute");
    }
    let body: Value = match request.json() {
        Ok(value) => value,
        Err(error) => return Response::error(400, error),
    };
    let code = body.get("code").and_then(Value::as_str).unwrap_or_default();
    if !shared.check_code(code) {
        return Response::error(401, "that pairing code is wrong or expired");
    }
    let name = body
        .get("device_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Android")
        .chars()
        .take(60)
        .collect::<String>();

    let device = Device {
        id: new_ulid(),
        name,
        token: random_token(),
        created_at: now().to_rfc3339(),
    };
    let mut mobile = load_mobile(&shared.paths.root);
    mobile.devices.push(device.clone());
    if let Err(error) = save_mobile(&shared.paths.root, &mobile) {
        return Response::error(500, format!("could not save the device: {error}"));
    }
    shared.rotate_pairing();
    Response::json(
        200,
        json!({
            "token": device.token,
            "device_id": device.id,
            "device_name": device.name,
            "port": shared.port,
            "revision": Store::open(shared.paths.clone()).map(|store| store.revision()).unwrap_or(0),
        }),
    )
}

fn snapshot_response(shared: &Shared) -> Response {
    match live_snapshot(shared) {
        Ok(value) => Response::json(200, value),
        Err(error) => Response::error(500, format!("{error}")),
    }
}

fn live_snapshot(shared: &Shared) -> Result<Value> {
    let store = Store::open(shared.paths.clone())?;
    Ok(json!({
        "revision": store.revision(),
        "schema_version": flowlist_core::persist::SCHEMA_VERSION,
        "generated_at": now().to_rfc3339(),
        "db": store.data().db,
    }))
}

#[derive(Deserialize)]
struct OpRequest {
    id: String,
    /// Quick-entry text, parsed here so the phone gets the same date, project
    /// and tag rules as the Mac. Either `text` or `command` is required.
    #[serde(default)]
    text: Option<String>,
    /// The id the phone already shows for an offline capture.
    #[serde(default)]
    task_id: Option<flowlist_core::ids::TaskId>,
    #[serde(default)]
    command: Option<Command>,
}

#[derive(Deserialize)]
struct OpsRequest {
    #[serde(default)]
    ops: Vec<OpRequest>,
    #[serde(default)]
    base_revision: Option<u64>,
    #[serde(default)]
    snapshot: Option<bool>,
}

fn ops(request: &Request, shared: &Shared) -> Response {
    let body: OpsRequest = match request.json() {
        Ok(value) => value,
        Err(error) => return Response::error(400, error),
    };
    if body.ops.len() > 200 {
        return Response::error(400, "send at most 200 operations at a time");
    }

    let mut store = match Store::open(shared.paths.clone()) {
        Ok(store) => store,
        Err(error) => return Response::error(500, format!("{error}")),
    };
    let mut mobile = load_mobile(&shared.paths.root);
    let mut seen: HashSet<String> = mobile.recent_ops.iter().cloned().collect();
    let stale = body
        .base_revision
        .is_some_and(|base| base != store.revision());
    let mut results = Vec::new();
    let mut changed = false;

    for op in body.ops {
        let id = op.id.trim().to_string();
        if id.is_empty() {
            results.push(json!({ "id": "", "ok": false, "error": "operation id is empty" }));
            continue;
        }
        if seen.contains(&id) {
            results.push(json!({ "id": id, "ok": true, "duplicate": true }));
            continue;
        }
        let command = match (op.text, op.command) {
            (Some(text), _) => match flowlist_core::parse::parse_quick(&text, now()) {
                Ok(entry) => {
                    let mut task = entry.into_new_task();
                    task.id = op.task_id;
                    Command::CreateTask(task)
                }
                Err(error) => {
                    results.push(json!({
                        "id": id,
                        "ok": false,
                        "error": error.to_string(),
                    }));
                    continue;
                }
            },
            (None, Some(command)) => command,
            (None, None) => {
                results.push(json!({
                    "id": id,
                    "ok": false,
                    "error": "an operation needs text or a command",
                }));
                continue;
            }
        };
        match store.execute(command) {
            Ok(outcome) => {
                changed = true;
                seen.insert(id.clone());
                mobile.recent_ops.push(id.clone());
                results.push(json!({ "id": id, "ok": true, "outcome": outcome }));
            }
            Err(error) => {
                results.push(json!({ "id": id, "ok": false, "error": error.to_string() }));
            }
        }
    }

    if mobile.recent_ops.len() > RECENT_OPS {
        let extra = mobile.recent_ops.len() - RECENT_OPS;
        mobile.recent_ops.drain(..extra);
    }
    if changed && let Err(error) = save_mobile(&shared.paths.root, &mobile) {
        return Response::error(500, format!("could not record the operations: {error}"));
    }

    let revision = store.revision();
    let db = if body.snapshot.unwrap_or(true) {
        Some(store.data().db.clone())
    } else {
        None
    };
    Response::json(
        200,
        json!({
            "revision": revision,
            "stale": stale,
            "results": results,
            "db": db,
        }),
    )
}

fn events(shared: &Shared, since: u64) -> Response {
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        let revision = Store::open(shared.paths.clone())
            .map(|store| store.revision())
            .unwrap_or(0);
        if revision != since {
            return snapshot_response(shared);
        }
        if Instant::now() >= deadline {
            return Response::json(200, json!({ "revision": since, "timeout": true }));
        }
        thread::sleep(Duration::from_millis(400));
    }
}

// ---------------------------------------------------------------------------
// Small helpers

fn new_code() -> String {
    let bytes = random_bytes(4);
    let value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 1_000_000;
    format!("{value:06}")
}

pub fn new_ulid() -> String {
    flowlist_core::ids::TaskId::new().to_string()
}

fn random_token() -> String {
    random_bytes(32)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn random_bytes(count: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; count];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        let _ = file.read_exact(&mut bytes);
    }
    bytes
}

fn constant_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(if bytes[index] == b'+' {
            b' '
        } else {
            bytes[index]
        });
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn parse_options(port: Option<u16>, bind: Option<String>) -> Options {
    Options {
        port: port.unwrap_or(DEFAULT_PORT),
        bind: bind.unwrap_or_else(|| DEFAULT_BIND.to_string()),
    }
}

pub fn print_addresses(port: u16) {
    for address in addresses() {
        println!("  http://{address}:{port}");
    }
}

pub fn require_port(port: u16) -> Result<u16> {
    if port == 0 {
        bail!("port cannot be 0");
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flowlist_core::command::{NewTask, Outcome};

    #[test]
    fn commands_round_trip_as_json() {
        let command = Command::CreateTask(NewTask::new("Buy stamps"));
        let text = serde_json::to_string(&command).unwrap();
        assert!(text.contains("create_task"), "{text}");
        let restored: Command = serde_json::from_str(&text).unwrap();
        assert_eq!(restored.label(), "Add task “Buy stamps”");
    }

    #[test]
    fn a_client_id_survives_a_command_round_trip() {
        let mut task = NewTask::new("Offline capture");
        let id = flowlist_core::ids::TaskId::new();
        task.id = Some(id.clone());
        let text = serde_json::to_string(&Command::CreateTask(task)).unwrap();
        let restored: Command = serde_json::from_str(&text).unwrap();
        match restored {
            Command::CreateTask(task) => assert_eq!(task.id, Some(id)),
            other => panic!("unexpected command {other:?}"),
        }
    }

    #[test]
    fn outcomes_serialize_for_the_phone() {
        let mut outcome = Outcome::new("Add task");
        outcome.created.push("abc".to_string());
        let text = serde_json::to_string(&outcome).unwrap();
        assert!(text.contains("created"));
    }

    #[test]
    fn pairing_codes_are_six_digits() {
        for _ in 0..50 {
            let code = new_code();
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|character| character.is_ascii_digit()));
        }
    }

    #[test]
    fn tokens_are_long_and_unique() {
        let first = random_token();
        let second = random_token();
        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
    }

    fn http(method: &str, url: &str, body: Option<&str>, token: Option<&str>) -> (u16, String) {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let rest = url.strip_prefix("http://").expect("http url");
        let (host, path) = rest.split_once('/').expect("path");
        let mut stream = TcpStream::connect(host).expect("connect");
        let body = body.unwrap_or("");
        let mut request = format!(
            "{method} /{path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\nContent-Type: application/json\r\n",
            body.len()
        );
        if let Some(token) = token {
            request.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        request.push_str("\r\n");
        request.push_str(body);
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let body = response
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.to_string())
            .unwrap_or_default();
        (status, body)
    }

    #[test]
    fn a_phone_can_pair_capture_and_retry_safely() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path());
        let running = start(
            &paths,
            Options {
                port: 0,
                bind: "127.0.0.1".to_string(),
            },
        )
        .unwrap();
        let base = format!("http://{}", running.addr());

        let (status, body) = http("GET", &format!("{base}/api/v1/pair"), None, None);
        assert_eq!(status, 200, "{body}");
        let code = serde_json::from_str::<Value>(&body).unwrap()["code"]
            .as_str()
            .unwrap()
            .to_string();

        let (status, body) = http(
            "POST",
            &format!("{base}/api/v1/pair"),
            Some(&format!(r#"{{"code":"{code}","device_name":"Test"}}"#)),
            None,
        );
        assert_eq!(status, 200, "{body}");
        let token = serde_json::from_str::<Value>(&body).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_string();

        let (status, _) = http(
            "GET",
            &format!("{base}/api/v1/snapshot"),
            None,
            Some("nope"),
        );
        assert_eq!(status, 401);

        let task_id = flowlist_core::ids::TaskId::new().to_string();
        let capture = format!(
            r#"{{"ops":[{{"id":"op-1","text":"Buy stamps due:tomorrow","task_id":"{task_id}"}}],"snapshot":true}}"#
        );
        let (status, body) = http(
            "POST",
            &format!("{base}/api/v1/ops"),
            Some(&capture),
            Some(&token),
        );
        assert_eq!(status, 200, "{body}");
        let value: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["results"][0]["ok"], true, "{value}");
        assert!(value["db"]["tasks"][&task_id].is_object(), "{value}");

        let (status, body) = http(
            "POST",
            &format!("{base}/api/v1/ops"),
            Some(&capture),
            Some(&token),
        );
        assert_eq!(status, 200, "{body}");
        let value: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["results"][0]["duplicate"], true, "{value}");
        let tasks = value["db"]["tasks"].as_object().unwrap();
        assert_eq!(tasks.len(), 1, "a retry must not duplicate the capture");

        let (status, body) = http(
            "POST",
            &format!("{base}/api/v1/unpair"),
            Some("{}"),
            Some(&token),
        );
        assert_eq!(status, 200, "{body}");
        let (status, _) = http(
            "GET",
            &format!("{base}/api/v1/snapshot"),
            None,
            Some(&token),
        );
        assert_eq!(status, 401, "an unpaired token must stop working");
    }

    #[test]
    fn a_bearer_token_must_match() {
        assert!(constant_eq(b"abc", b"abc"));
        assert!(!constant_eq(b"abc", b"abd"));
        assert!(!constant_eq(b"abc", b"abcd"));
    }
}
