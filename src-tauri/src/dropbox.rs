//! Dropbox sync for the content directory.
//!
//! Mirrors the GitHub flow in `git.rs`: log in once, point at a folder, then
//! push/pull on demand (plus an auto-push when the app closes). Auth is OAuth2
//! with PKCE — no client secret is baked into the binary — and the long-lived
//! refresh token is kept in the OS keyring, never in `config.yaml`.
//!
//! Files are compared by Dropbox's own `content_hash`, so a push or pull only
//! moves the files that actually differ. Only `.md` files under the content
//! directory participate, matching what the rest of the app treats as writing.

use super::error::DropboxError;
use super::fs::discover_files;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write as IoWrite};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Public Dropbox app key. With PKCE there is no client secret, so this is safe
/// to ship in the binary (same reasoning as `GITHUB_CLIENT_ID` in `git.rs`).
/// Set at build time via `DROPBOX_APP_KEY=... cargo build`.
const DROPBOX_APP_KEY: &str = match option_env!("DROPBOX_APP_KEY") {
    Some(key) => key,
    None => "",
};

/// Dropbox requires redirect URIs to be registered exactly, port included, so
/// the loopback listener uses a fixed port rather than an ephemeral one.
/// Register `http://localhost:53682/` on the app's console page.
const REDIRECT_PORT: u16 = 53682;
const REDIRECT_URI: &str = "http://localhost:53682/";

const KEYRING_SERVICE: &str = "memoire";
const KEYRING_USER: &str = "dropbox_refresh_token";

/// How long the loopback listener waits for the browser to come back.
const AUTH_TIMEOUT: Duration = Duration::from_secs(300);

/// Dropbox hashes files in 4 MiB blocks.
const HASH_BLOCK: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropboxAccount {
    pub account_id: String,
    pub name: String,
    pub email: Option<String>,
    pub photo_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropboxAuthStart {
    pub auth_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AuthPollResult {
    Pending,
    Success { account: DropboxAccount },
    Expired,
    Error { message: String },
}

/// One file that differs between the content directory and the Dropbox folder.
#[derive(Debug, Serialize, Deserialize)]
pub struct DropboxFileStatus {
    pub path: String,
    /// `local_new`, `local_modified`, `remote_new`, `remote_modified` or
    /// `conflict`.
    pub status: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SyncResult {
    pub uploaded: u32,
    pub downloaded: u32,
    /// Files changed on both sides. A push skips them; a pull saves Dropbox's
    /// version beside the local one.
    pub conflicts: u32,
    /// Files deleted on one side since the last sync, removed from the other.
    pub deleted: u32,
}

/// The in-flight login: the loopback listener's result channel plus the PKCE
/// verifier the token exchange needs. Only one login runs at a time.
struct PendingAuth {
    receiver: Receiver<Result<String, String>>,
    verifier: String,
    started: Instant,
    /// Tells the listener thread to stop and free the port.
    cancelled: Arc<AtomicBool>,
}

/// However the login ends — cancelled, replaced by a new one, finished — the
/// listener thread is told to stop, so the fixed port is free for a retry.
impl Drop for PendingAuth {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

static PENDING_AUTH: Mutex<Option<PendingAuth>> = Mutex::new(None);

/// Cached access token and the instant it stops being usable. Access tokens are
/// short-lived; the refresh token in the keyring is what actually persists.
static ACCESS_TOKEN: Mutex<Option<(String, Instant)>> = Mutex::new(None);

/**
 *  Token storage
 */

fn token_entry() -> Result<Entry, DropboxError> {
    Ok(Entry::new(KEYRING_SERVICE, KEYRING_USER)?)
}

pub fn save_refresh_token(token: &str) -> Result<(), DropboxError> {
    token_entry()?.set_password(token)?;
    Ok(())
}

pub fn load_refresh_token() -> Result<Option<String>, DropboxError> {
    match token_entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn clear_refresh_token() -> Result<(), DropboxError> {
    if let Ok(mut cached) = ACCESS_TOKEN.lock() {
        *cached = None;
    }
    // The next account may have a same-named folder with different contents.
    clear_sync_state();
    match token_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub fn is_logged_in() -> bool {
    matches!(load_refresh_token(), Ok(Some(_)))
}

/**
 *  OAuth2 with PKCE
 */

fn app_key() -> Result<&'static str, DropboxError> {
    if DROPBOX_APP_KEY.is_empty() {
        return Err(DropboxError::NoAppKey);
    }
    Ok(DROPBOX_APP_KEY)
}

/// Base64url without padding, as PKCE and Dropbox expect.
fn base64_url_nopad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        let indices = [n >> 18 & 63, n >> 12 & 63, n >> 6 & 63, n & 63];
        for (i, index) in indices.iter().enumerate() {
            // 1 input byte yields 2 output chars, 2 bytes yield 3, 3 bytes yield 4.
            if i <= chunk.len() {
                out.push(ALPHABET[*index as usize] as char);
            }
        }
    }
    out
}

/// A high-entropy PKCE code verifier built from two v4 UUIDs (128 random bits
/// each), rendered as unreserved characters.
fn new_code_verifier() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn code_challenge(verifier: &str) -> String {
    base64_url_nopad(&Sha256::digest(verifier.as_bytes()))
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Starts the loopback listener and returns the URL the browser should open.
/// The caller opens it; `poll_login` then waits for the redirect to land.
pub fn start_login() -> Result<DropboxAuthStart, DropboxError> {
    let app_key = app_key()?;

    // Stop any earlier login first; its listener holds the port we need.
    PENDING_AUTH.lock().expect("auth lock poisoned").take();
    let listener = bind_loopback()?;

    let verifier = new_code_verifier();
    let challenge = code_challenge(&verifier);
    let state = uuid::Uuid::new_v4().simple().to_string();

    let (sender, receiver) = mpsc::channel();
    let expected_state = state.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let thread_cancelled = cancelled.clone();

    std::thread::spawn(move || {
        let result = wait_for_redirect(listener, &expected_state, &thread_cancelled);
        let _ = sender.send(result);
    });

    *PENDING_AUTH.lock().expect("auth lock poisoned") = Some(PendingAuth {
        receiver,
        verifier,
        started: Instant::now(),
        cancelled,
    });

    let auth_url = format!(
        "https://www.dropbox.com/oauth2/authorize?client_id={}&response_type=code\
         &code_challenge={}&code_challenge_method=S256&token_access_type=offline\
         &redirect_uri={}&state={}",
        percent_encode(app_key),
        percent_encode(&challenge),
        percent_encode(REDIRECT_URI),
        percent_encode(&state),
    );

    Ok(DropboxAuthStart { auth_url })
}

/// Binds the fixed redirect port. A just-cancelled login's listener notices
/// within one accept tick and releases the port, so retry briefly before
/// reporting it as taken.
fn bind_loopback() -> Result<TcpListener, DropboxError> {
    let mut attempts = 0;
    loop {
        match TcpListener::bind(("127.0.0.1", REDIRECT_PORT)) {
            Ok(listener) => return Ok(listener),
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse && attempts < 10 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(DropboxError::Loopback(format!(
                    "port {REDIRECT_PORT} unavailable: {e}"
                )))
            }
        }
    }
}

/// Serves loopback requests until one carries the authorization code. Browsers
/// also ask for `/favicon.ico` and the like, so anything without a code is
/// answered and ignored.
fn wait_for_redirect(
    listener: TcpListener,
    expected_state: &str,
    cancelled: &AtomicBool,
) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("could not configure loopback listener: {e}"))?;

    let deadline = Instant::now() + AUTH_TIMEOUT;

    loop {
        if cancelled.load(Ordering::SeqCst) {
            return Err("cancelled".to_string());
        }
        if Instant::now() > deadline {
            return Err("timeout".to_string());
        }

        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(e) => return Err(format!("loopback accept failed: {e}")),
        };

        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

        let mut request_line = String::new();
        if BufReader::new(&stream)
            .read_line(&mut request_line)
            .is_err()
        {
            continue;
        }

        let target = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("/")
            .to_string();
        let params = query_params(&target);

        if let Some(error) = params.get("error") {
            respond(&mut stream, "Login failed. You can close this window.");
            return Err(error.clone());
        }

        let Some(code) = params.get("code") else {
            respond(&mut stream, "Waiting for Dropbox…");
            continue;
        };

        if params.get("state").map(String::as_str) != Some(expected_state) {
            respond(&mut stream, "Login failed. You can close this window.");
            return Err(
                "state mismatch — the login response did not match this request".to_string(),
            );
        }

        respond(
            &mut stream,
            "Memoire is connected to Dropbox. You can close this window.",
        );
        return Ok(code.clone());
    }
}

fn respond(stream: &mut std::net::TcpStream, message: &str) {
    let body = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Memoire</title>\
         <body style=\"font-family:system-ui;padding:3rem;text-align:center\">{message}</body>"
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn query_params(target: &str) -> HashMap<String, String> {
    let Some((_, query)) = target.split_once('?') else {
        return HashMap::new();
    };

    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_string(), percent_decode(value)))
        .collect()
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => match u8::from_str_radix(&value[i + 1..i + 3], 16) {
                Ok(byte) => {
                    out.push(byte);
                    i += 3;
                }
                Err(_) => {
                    out.push(bytes[i]);
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// Checks whether the browser has come back yet. Returns `Pending` while the
/// listener is still waiting, and on success stores the refresh token.
pub async fn poll_login() -> Result<AuthPollResult, DropboxError> {
    let (code, verifier) = {
        let mut pending = PENDING_AUTH.lock().expect("auth lock poisoned");
        let Some(auth) = pending.as_mut() else {
            return Ok(AuthPollResult::Error {
                message: "No Dropbox login in progress".to_string(),
            });
        };

        match auth.receiver.try_recv() {
            Ok(Ok(code)) => {
                let verifier = auth.verifier.clone();
                *pending = None;
                (code, verifier)
            }
            Ok(Err(message)) => {
                *pending = None;
                return Ok(if message == "timeout" {
                    AuthPollResult::Expired
                } else {
                    AuthPollResult::Error { message }
                });
            }
            Err(TryRecvError::Empty) => {
                if auth.started.elapsed() > AUTH_TIMEOUT {
                    *pending = None;
                    return Ok(AuthPollResult::Expired);
                }
                return Ok(AuthPollResult::Pending);
            }
            Err(TryRecvError::Disconnected) => {
                *pending = None;
                return Ok(AuthPollResult::Error {
                    message: "Dropbox login was interrupted".to_string(),
                });
            }
        }
    };

    let token = exchange_code(&code, &verifier).await?;
    save_refresh_token(&token.refresh_token)?;
    cache_access_token(&token.access_token, token.expires_in);

    let account = fetch_account(&token.access_token).await?;
    Ok(AuthPollResult::Success { account })
}

pub fn cancel_login() {
    *PENDING_AUTH.lock().expect("auth lock poisoned") = None;
}

struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

async fn exchange_code(code: &str, verifier: &str) -> Result<TokenResponse, DropboxError> {
    let app_key = app_key()?;
    let client = reqwest::Client::new();

    let res: serde_json::Value = client
        .post("https://api.dropboxapi.com/oauth2/token")
        .form(&[
            ("code", code),
            ("grant_type", "authorization_code"),
            ("client_id", app_key),
            ("code_verifier", verifier),
            ("redirect_uri", REDIRECT_URI),
        ])
        .send()
        .await?
        .json()
        .await?;

    if let Some(error) = res.get("error_description").and_then(|v| v.as_str()) {
        return Err(DropboxError::Auth(error.to_string()));
    }

    let refresh_token = res["refresh_token"]
        .as_str()
        .ok_or_else(|| DropboxError::Auth("response had no refresh token".to_string()))?
        .to_string();

    Ok(TokenResponse {
        access_token: res["access_token"].as_str().unwrap_or_default().to_string(),
        refresh_token,
        expires_in: res["expires_in"].as_u64().unwrap_or(14400),
    })
}

fn cache_access_token(token: &str, expires_in: u64) {
    // Expire a minute early so a token never dies mid-request.
    let ttl = Duration::from_secs(expires_in.saturating_sub(60).max(60));
    if let Ok(mut cached) = ACCESS_TOKEN.lock() {
        *cached = Some((token.to_string(), Instant::now() + ttl));
    }
}

/// Returns a usable access token, refreshing it from the stored refresh token
/// when the cached one is missing or stale.
async fn access_token() -> Result<String, DropboxError> {
    if let Ok(cached) = ACCESS_TOKEN.lock() {
        if let Some((token, expires_at)) = cached.as_ref() {
            if Instant::now() < *expires_at {
                return Ok(token.clone());
            }
        }
    }

    let refresh_token = load_refresh_token()?.ok_or(DropboxError::NotLoggedIn)?;
    let app_key = app_key()?;

    let res: serde_json::Value = reqwest::Client::new()
        .post("https://api.dropboxapi.com/oauth2/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", app_key),
        ])
        .send()
        .await?
        .json()
        .await?;

    let Some(token) = res["access_token"].as_str() else {
        let message = res["error_description"]
            .as_str()
            .unwrap_or("could not refresh the Dropbox session")
            .to_string();
        return Err(DropboxError::Auth(message));
    };

    cache_access_token(token, res["expires_in"].as_u64().unwrap_or(14400));
    Ok(token.to_string())
}

/**
 *  Dropbox API
 */

async fn fetch_account(token: &str) -> Result<DropboxAccount, DropboxError> {
    let response = reqwest::Client::new()
        .post("https://api.dropboxapi.com/2/users/get_current_account")
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(DropboxError::Api(format!("{status}: {body}")));
    }

    let res: serde_json::Value = response.json().await?;

    Ok(DropboxAccount {
        account_id: res["account_id"].as_str().unwrap_or_default().to_string(),
        name: res["name"]["display_name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        email: res["email"].as_str().map(|s| s.to_string()),
        photo_url: res["profile_photo_url"].as_str().map(|s| s.to_string()),
    })
}

/// The logged-in account, or `None` when there is no stored session.
pub async fn current_account() -> Result<Option<DropboxAccount>, DropboxError> {
    if !is_logged_in() {
        return Ok(None);
    }
    let token = access_token().await?;
    Ok(Some(fetch_account(&token).await?))
}

async fn rpc(endpoint: &str, body: serde_json::Value) -> Result<serde_json::Value, DropboxError> {
    let token = access_token().await?;

    let response = reqwest::Client::new()
        .post(format!("https://api.dropboxapi.com/2/{endpoint}"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(DropboxError::Api(format!("{status}: {text}")));
    }

    Ok(response.json().await?)
}

/// `Dropbox-API-Arg` is an HTTP header, so every non-ASCII character has to be
/// escaped — file names with accents or emoji otherwise make the request invalid.
fn ascii_escape_json(value: &serde_json::Value) -> String {
    let mut out = String::new();

    for c in value.to_string().chars() {
        if c.is_ascii() {
            out.push(c);
            continue;
        }

        let mut units = [0u16; 2];
        for unit in c.encode_utf16(&mut units) {
            out.push_str(&format!("\\u{unit:04x}"));
        }
    }

    out
}

async fn upload(remote_path: &str, contents: Vec<u8>) -> Result<(), DropboxError> {
    let token = access_token().await?;
    let arg = serde_json::json!({
        "path": remote_path,
        "mode": "overwrite",
        "autorename": false,
        "mute": true,
    });

    let response = reqwest::Client::new()
        .post("https://content.dropboxapi.com/2/files/upload")
        .header("Authorization", format!("Bearer {token}"))
        .header("Dropbox-API-Arg", ascii_escape_json(&arg))
        .header("Content-Type", "application/octet-stream")
        .body(contents)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(DropboxError::Api(format!(
            "upload {remote_path} — {status}: {text}"
        )));
    }

    Ok(())
}

async fn download(remote_path: &str) -> Result<Vec<u8>, DropboxError> {
    let token = access_token().await?;
    let arg = serde_json::json!({ "path": remote_path });

    let response = reqwest::Client::new()
        .post("https://content.dropboxapi.com/2/files/download")
        .header("Authorization", format!("Bearer {token}"))
        .header("Dropbox-API-Arg", ascii_escape_json(&arg))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(DropboxError::Api(format!(
            "download {remote_path} — {status}: {text}"
        )));
    }

    Ok(response.bytes().await?.to_vec())
}

/**
 *  Sync
 */

/// Normalises a user-supplied folder into a Dropbox path: `Memoire`,
/// `/Memoire` and `/Memoire/` all become `/Memoire`. The account root is `""`.
pub fn normalize_folder(folder: &str) -> String {
    let trimmed = folder.trim().trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
}

async fn delete_remote(remote_path: &str) -> Result<(), DropboxError> {
    rpc("files/delete_v2", serde_json::json!({ "path": remote_path })).await?;
    Ok(())
}

fn remote_path(folder: &str, relative: &str) -> String {
    format!("{}/{}", normalize_folder(folder), relative)
}

/// Dropbox's content hash: SHA-256 of the concatenated SHA-256 digests of each
/// 4 MiB block. <https://www.dropbox.com/developers/reference/content-hash>
fn content_hash(bytes: &[u8]) -> String {
    let mut concatenated = Vec::new();
    for block in bytes.chunks(HASH_BLOCK) {
        concatenated.extend_from_slice(&Sha256::digest(block));
    }
    hex::encode(Sha256::digest(&concatenated))
}

/// Relative path -> (absolute path, content hash) for every `.md` file under
/// the content directory.
fn local_index(content_dir: &Path) -> Result<HashMap<String, (PathBuf, String)>, DropboxError> {
    let mut index = HashMap::new();

    for file in discover_files(content_dir)? {
        let bytes = std::fs::read(&file.path)?;
        index.insert(file.name.clone(), (file.path, content_hash(&bytes)));
    }

    Ok(index)
}

/// Relative path -> (Dropbox path, content hash) for every `.md` file in the
/// configured folder. A folder that does not exist yet reads as empty.
async fn remote_index(folder: &str) -> Result<HashMap<String, (String, String)>, DropboxError> {
    let root = normalize_folder(folder);
    let mut index = HashMap::new();

    let mut page = match rpc(
        "files/list_folder",
        serde_json::json!({ "path": root, "recursive": true }),
    )
    .await
    {
        Ok(page) => page,
        // The folder is created on first upload, so "not found" just means empty.
        Err(DropboxError::Api(ref message)) if message.contains("path/not_found") => {
            return Ok(index)
        }
        Err(e) => return Err(e),
    };

    loop {
        for entry in page["entries"].as_array().into_iter().flatten() {
            if entry[".tag"].as_str() != Some("file") {
                continue;
            }

            let path = entry["path_display"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let hash = entry["content_hash"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            let Some(relative) = path.strip_prefix(&format!("{root}/")) else {
                continue;
            };

            if !relative.ends_with(".md") {
                continue;
            }

            index.insert(relative.to_string(), (path, hash));
        }

        if !page["has_more"].as_bool().unwrap_or(false) {
            break;
        }

        let cursor = page["cursor"].as_str().unwrap_or_default().to_string();
        page = rpc(
            "files/list_folder/continue",
            serde_json::json!({ "cursor": cursor }),
        )
        .await?;
    }

    Ok(index)
}

/// How one file compares across the content directory, Dropbox, and the last
/// sync. Without the last-synced hash a difference can't say which side moved,
/// so a push or pull would blindly overwrite the other side's edits.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Change {
    InSync,
    LocalNew,
    LocalModified,
    RemoteNew,
    RemoteModified,
    /// Deleted locally; Dropbox still has the last-synced version.
    LocalDeleted,
    /// Deleted in Dropbox; the local file is still the last-synced version.
    RemoteDeleted,
    /// Both sides changed since the last sync (or they differ and were never
    /// synced). Neither push nor pull overwrites it.
    Conflict,
}

impl Change {
    fn as_str(self) -> &'static str {
        match self {
            Change::InSync => "in_sync",
            Change::LocalNew => "local_new",
            Change::LocalModified => "local_modified",
            Change::RemoteNew => "remote_new",
            Change::RemoteModified => "remote_modified",
            Change::LocalDeleted => "local_deleted",
            Change::RemoteDeleted => "remote_deleted",
            Change::Conflict => "conflict",
        }
    }
}

fn classify(local: Option<&str>, remote: Option<&str>, base: Option<&str>) -> Change {
    match (local, remote) {
        (Some(l), Some(r)) if l == r => Change::InSync,
        // Missing on one side but the other still matches the last sync: it
        // was deleted, not created. Anything else — never synced, or edited
        // since — is new, so a delete never wins over an edit.
        (Some(l), None) if base == Some(l) => Change::RemoteDeleted,
        (None, Some(r)) if base == Some(r) => Change::LocalDeleted,
        (Some(_), None) => Change::LocalNew,
        (None, Some(_)) => Change::RemoteNew,
        (Some(l), Some(r)) => match base {
            Some(b) if b == r => Change::LocalModified,
            Some(b) if b == l => Change::RemoteModified,
            _ => Change::Conflict,
        },
        (None, None) => Change::InSync,
    }
}

/// Content hashes as of the last push or pull, per relative path. Tied to the
/// content directory and folder it was recorded for; a change to either starts
/// from scratch rather than trusting hashes from a different pairing.
#[derive(Debug, Default, Serialize, Deserialize)]
struct SyncState {
    content_dir: PathBuf,
    folder: String,
    files: HashMap<String, String>,
}

fn sync_state_path() -> Result<PathBuf, DropboxError> {
    Ok(super::utils::get_app_dir()?.join("dropbox_sync.json"))
}

fn load_base(content_dir: &Path, folder: &str) -> HashMap<String, String> {
    let Ok(path) = sync_state_path() else {
        return HashMap::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    match serde_json::from_str::<SyncState>(&text) {
        Ok(state) if state.content_dir == content_dir && state.folder == normalize_folder(folder) => {
            state.files
        }
        _ => HashMap::new(),
    }
}

fn save_base(
    content_dir: &Path,
    folder: &str,
    files: HashMap<String, String>,
) -> Result<(), DropboxError> {
    let state = SyncState {
        content_dir: content_dir.to_path_buf(),
        folder: normalize_folder(folder),
        files,
    };
    let text = serde_json::to_string(&state)
        .map_err(|e| DropboxError::Api(format!("could not record sync state: {e}")))?;
    std::fs::write(sync_state_path()?, text)?;
    Ok(())
}

fn clear_sync_state() {
    if let Ok(path) = sync_state_path() {
        let _ = std::fs::remove_file(path);
    }
}

/// Both indexes plus the last-synced hashes, trimmed to files that still
/// exist on at least one side.
async fn sync_inputs(
    content_dir: &Path,
    folder: &str,
) -> Result<
    (
        HashMap<String, (PathBuf, String)>,
        HashMap<String, (String, String)>,
        HashMap<String, String>,
    ),
    DropboxError,
> {
    let local = local_index(content_dir)?;
    let remote = remote_index(folder).await?;
    let mut base = load_base(content_dir, folder);
    base.retain(|relative, _| local.contains_key(relative) || remote.contains_key(relative));
    Ok((local, remote, base))
}

/// Every file that differs between the content directory and Dropbox, so the
/// UI can show what a push or pull would move — and what it won't touch.
pub async fn status(
    content_dir: &Path,
    folder: &str,
) -> Result<Vec<DropboxFileStatus>, DropboxError> {
    let (local, remote, base) = sync_inputs(content_dir, folder).await?;

    let mut paths: Vec<&String> = local.keys().chain(remote.keys()).collect();
    paths.sort();
    paths.dedup();

    Ok(paths
        .into_iter()
        .filter_map(|relative| {
            let change = classify(
                local.get(relative).map(|(_, hash)| hash.as_str()),
                remote.get(relative).map(|(_, hash)| hash.as_str()),
                base.get(relative).map(String::as_str),
            );
            (change != Change::InSync).then(|| DropboxFileStatus {
                path: relative.clone(),
                status: change.as_str().to_string(),
            })
        })
        .collect())
}

/// Uploads a local file and returns the hash of exactly what was sent, which
/// may be newer than what the index saw if the file was saved in between.
async fn upload_file(path: &Path, remote: &str) -> Result<String, DropboxError> {
    let bytes = std::fs::read(path)?;
    let hash = content_hash(&bytes);
    upload(remote, bytes).await?;
    Ok(hash)
}

/// Downloads a Dropbox file to `destination` and returns the hash of what was
/// written.
async fn download_file(remote: &str, destination: &Path) -> Result<String, DropboxError> {
    let bytes = download(remote).await?;
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(destination, &bytes)?;
    Ok(content_hash(&bytes))
}

/// `Novel/Chapter 1.md` -> `<content>/Novel/Chapter 1 (Dropbox conflict).md`.
fn conflict_copy_path(content_dir: &Path, relative: &str) -> PathBuf {
    let original = content_dir.join(relative);
    let stem = original
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Untitled");
    original.with_file_name(format!("{stem} (Dropbox conflict).md"))
}

/// Uploads local files that are new or changed only locally, and deletes from
/// Dropbox files removed here since the last sync. Files Dropbox changed are
/// left for a pull, and conflicts are counted but never overwritten. A delete
/// only goes through when Dropbox's copy is unchanged, so it never wipes an
/// edit made on the other side.
pub async fn push(content_dir: &Path, folder: &str) -> Result<SyncResult, DropboxError> {
    let (local, remote, mut base) = sync_inputs(content_dir, folder).await?;
    let mut result = SyncResult::default();
    let mut failure = None;

    for (relative, (path, local_hash)) in &local {
        let change = classify(
            Some(local_hash),
            remote.get(relative).map(|(_, hash)| hash.as_str()),
            base.get(relative).map(String::as_str),
        );

        match change {
            Change::InSync => {
                base.insert(relative.clone(), local_hash.clone());
            }
            Change::LocalNew | Change::LocalModified => {
                match upload_file(path, &remote_path(folder, relative)).await {
                    Ok(hash) => {
                        base.insert(relative.clone(), hash);
                        result.uploaded += 1;
                    }
                    Err(e) => {
                        failure = Some(e);
                        break;
                    }
                }
            }
            Change::Conflict => result.conflicts += 1,
            Change::RemoteNew | Change::RemoteModified | Change::RemoteDeleted => {}
            Change::LocalDeleted => {}
        }
    }

    // Files deleted here since the last sync. Dropbox keeps deleted files in
    // its history, so this is recoverable there.
    if failure.is_none() {
        for (relative, (path, remote_hash)) in &remote {
            if local.contains_key(relative) {
                continue;
            }
            if classify(None, Some(remote_hash), base.get(relative).map(String::as_str))
                != Change::LocalDeleted
            {
                continue;
            }
            match delete_remote(path).await {
                Ok(()) => {
                    base.remove(relative);
                    result.deleted += 1;
                }
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
    }

    // Record whatever did transfer, even if a later file failed.
    save_base(content_dir, folder, base)?;
    failure.map_or(Ok(result), Err)
}

/// Downloads Dropbox files that are new or changed only in Dropbox, and
/// removes local files deleted from Dropbox since the last sync. For a
/// conflict the local file is kept and Dropbox's version is saved beside it as
/// `<name> (Dropbox conflict).md`; the next push then uploads both, so neither
/// edit is lost.
pub async fn pull(content_dir: &Path, folder: &str) -> Result<SyncResult, DropboxError> {
    let (local, remote, mut base) = sync_inputs(content_dir, folder).await?;
    let mut result = SyncResult::default();
    let mut failure = None;

    for (relative, (path, remote_hash)) in &remote {
        let change = classify(
            local.get(relative).map(|(_, hash)| hash.as_str()),
            Some(remote_hash),
            base.get(relative).map(String::as_str),
        );

        let destination = match change {
            Change::InSync => {
                base.insert(relative.clone(), remote_hash.clone());
                continue;
            }
            Change::RemoteNew | Change::RemoteModified => content_dir.join(relative),
            Change::Conflict => conflict_copy_path(content_dir, relative),
            Change::LocalNew | Change::LocalModified | Change::LocalDeleted => continue,
            Change::RemoteDeleted => continue,
        };

        match download_file(path, &destination).await {
            Ok(hash) => {
                // For a conflict this marks Dropbox's version as seen, so the
                // local file now reads as the newer edit.
                base.insert(relative.clone(), hash);
                if change == Change::Conflict {
                    result.conflicts += 1;
                } else {
                    result.downloaded += 1;
                }
            }
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }

    // Files deleted in Dropbox since the last sync. Only removed when the
    // local copy is unchanged, so Dropbox's history still has this content.
    if failure.is_none() {
        for (relative, (path, local_hash)) in &local {
            if remote.contains_key(relative) {
                continue;
            }
            if classify(Some(local_hash), None, base.get(relative).map(String::as_str))
                != Change::RemoteDeleted
            {
                continue;
            }
            match std::fs::remove_file(path) {
                Ok(()) => {
                    base.remove(relative);
                    result.deleted += 1;
                }
                Err(e) => {
                    failure = Some(e.into());
                    break;
                }
            }
        }
    }

    save_base(content_dir, folder, base)?;
    failure.map_or(Ok(result), Err)
}

#[cfg(test)]
mod dropbox_tests {
    use super::*;

    #[test]
    fn content_hash_matches_dropbox_reference() {
        // A single short block: SHA-256 of the SHA-256 of the contents.
        let expected = hex::encode(Sha256::digest(Sha256::digest(b"hello")));
        assert_eq!(content_hash(b"hello"), expected);
    }

    #[test]
    fn content_hash_blocks_are_concatenated() {
        let bytes = vec![7u8; HASH_BLOCK + 10];
        let mut concatenated = Vec::new();
        concatenated.extend_from_slice(&Sha256::digest(&bytes[..HASH_BLOCK]));
        concatenated.extend_from_slice(&Sha256::digest(&bytes[HASH_BLOCK..]));

        assert_eq!(
            content_hash(&bytes),
            hex::encode(Sha256::digest(&concatenated))
        );
    }

    #[test]
    fn base64_url_has_no_padding_and_is_url_safe() {
        assert_eq!(base64_url_nopad(b""), "");
        assert_eq!(base64_url_nopad(b"f"), "Zg");
        assert_eq!(base64_url_nopad(b"fo"), "Zm8");
        assert_eq!(base64_url_nopad(b"foo"), "Zm9v");
        assert_eq!(base64_url_nopad(b"foob"), "Zm9vYg");
        assert_eq!(base64_url_nopad(&[251, 255, 190]), "-_--");
    }

    #[test]
    fn code_challenge_is_sha256_of_verifier() {
        let verifier = "abc123";
        assert_eq!(
            code_challenge(verifier),
            base64_url_nopad(&Sha256::digest(verifier.as_bytes()))
        );
    }

    #[test]
    fn folders_normalize_to_a_dropbox_path() {
        assert_eq!(normalize_folder("Memoire"), "/Memoire");
        assert_eq!(normalize_folder("/Memoire/"), "/Memoire");
        assert_eq!(normalize_folder("  /Apps/Memoire  "), "/Apps/Memoire");
        assert_eq!(normalize_folder("/"), "");
        assert_eq!(normalize_folder(""), "");
    }

    #[test]
    fn remote_paths_join_folder_and_relative_path() {
        assert_eq!(
            remote_path("Memoire", "Novel/Chapter 1.md"),
            "/Memoire/Novel/Chapter 1.md"
        );
        assert_eq!(remote_path("", "Note.md"), "/Note.md");
    }

    #[test]
    fn query_params_are_parsed_and_decoded() {
        let params = query_params("/?code=abc%2Fdef&state=xyz");
        assert_eq!(params.get("code").unwrap(), "abc/def");
        assert_eq!(params.get("state").unwrap(), "xyz");
        assert!(query_params("/favicon.ico").is_empty());
    }

    #[test]
    fn classify_uses_last_sync_to_tell_which_side_moved() {
        assert_eq!(classify(Some("a"), Some("a"), None), Change::InSync);
        assert_eq!(classify(Some("a"), None, None), Change::LocalNew);
        assert_eq!(classify(None, Some("a"), None), Change::RemoteNew);
        assert_eq!(classify(Some("b"), Some("a"), Some("a")), Change::LocalModified);
        assert_eq!(classify(Some("a"), Some("b"), Some("a")), Change::RemoteModified);
    }

    #[test]
    fn classify_detects_deletes_only_when_other_side_is_unchanged() {
        assert_eq!(classify(None, Some("a"), Some("a")), Change::LocalDeleted);
        assert_eq!(classify(Some("a"), None, Some("a")), Change::RemoteDeleted);
        // Edited on the other side after the delete: the edit wins.
        assert_eq!(classify(None, Some("b"), Some("a")), Change::RemoteNew);
        assert_eq!(classify(Some("b"), None, Some("a")), Change::LocalNew);
    }

    #[test]
    fn classify_flags_both_sides_changed_as_conflict() {
        assert_eq!(classify(Some("b"), Some("c"), Some("a")), Change::Conflict);
        // Never synced and different: can't tell which is newer.
        assert_eq!(classify(Some("b"), Some("c"), None), Change::Conflict);
    }

    #[test]
    fn conflict_copy_sits_beside_the_original() {
        assert_eq!(
            conflict_copy_path(Path::new("/c"), "Novel/Chapter 1.md"),
            PathBuf::from("/c/Novel/Chapter 1 (Dropbox conflict).md")
        );
    }

    #[test]
    fn api_arg_escapes_non_ascii() {
        let arg = serde_json::json!({ "path": "/Mémoire.md" });
        assert_eq!(ascii_escape_json(&arg), "{\"path\":\"/M\\u00e9moire.md\"}");
    }
}
