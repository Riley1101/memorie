use super::error::GitError;
use git2::{Cred, PushOptions, RemoteCallbacks, Repository, Signature};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Public GitHub OAuth App client id (device flow, no client secret needed).
/// Register one at https://github.com/settings/developers with "Enable Device Flow" checked.
/// Set at build time via `GITHUB_CLIENT_ID=... cargo build` / `.cargo/config.toml` env,
/// since this ends up baked into the compiled binary either way (device-flow client ids
/// are not secret — gh CLI and VSCode ship theirs the same way).
const GITHUB_CLIENT_ID: &str = match option_env!("GIT_CLIENT_ID") {
    Some(id) => id,
    None => "Iv23licYPOLsngZ7NztG",
};

const KEYRING_SERVICE: &str = "memoire";
const KEYRING_USER: &str = "github_token";

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DevicePollResult {
    Pending,
    SlowDown,
    Success { user: GitHubUser },
    Expired,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

fn token_entry() -> Result<Entry, GitError> {
    Ok(Entry::new(KEYRING_SERVICE, KEYRING_USER)?)
}

pub fn save_token(token: &str) -> Result<(), GitError> {
    token_entry()?.set_password(token)?;
    Ok(())
}

pub fn load_token() -> Result<Option<String>, GitError> {
    match token_entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn clear_token() -> Result<(), GitError> {
    match token_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub async fn start_device_flow() -> Result<DeviceCode, GitError> {
    let client = reqwest::Client::new();
    let res: serde_json::Value = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", GITHUB_CLIENT_ID), ("scope", "repo")])
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = res.get("error").and_then(|v| v.as_str()) {
        return Err(GitError::DeviceFlow(err.to_string()));
    }

    Ok(DeviceCode {
        device_code: res["device_code"].as_str().unwrap_or_default().to_string(),
        user_code: res["user_code"].as_str().unwrap_or_default().to_string(),
        verification_uri: res["verification_uri"].as_str().unwrap_or_default().to_string(),
        interval: res["interval"].as_u64().unwrap_or(5),
        expires_in: res["expires_in"].as_u64().unwrap_or(900),
    })
}

pub async fn poll_device_flow(device_code: &str) -> Result<DevicePollResult, GitError> {
    let client = reqwest::Client::new();
    let res: serde_json::Value = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", GITHUB_CLIENT_ID),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await?
        .json()
        .await?;

    if let Some(token) = res.get("access_token").and_then(|v| v.as_str()) {
        save_token(token)?;
        let user = fetch_github_user(token).await?;
        return Ok(DevicePollResult::Success { user });
    }

    match res.get("error").and_then(|v| v.as_str()) {
        Some("authorization_pending") => Ok(DevicePollResult::Pending),
        Some("slow_down") => Ok(DevicePollResult::SlowDown),
        Some("expired_token") => Ok(DevicePollResult::Expired),
        Some(other) => Ok(DevicePollResult::Error { message: other.to_string() }),
        None => Ok(DevicePollResult::Error { message: "Unknown device flow response".to_string() }),
    }
}

pub async fn fetch_github_user(token: &str) -> Result<GitHubUser, GitError> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "memoire-app")
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(GitError::GitHubApi(format!("{status}: {body}")));
    }

    let res: serde_json::Value = response.json().await?;

    let login = res["login"].as_str().unwrap_or_default().to_string();
    if login.is_empty() {
        return Err(GitError::GitHubApi("response missing login".to_string()));
    }

    Ok(GitHubUser {
        login,
        name: res["name"].as_str().map(|s| s.to_string()),
        avatar_url: res["avatar_url"].as_str().map(|s| s.to_string()),
    })
}

pub fn ensure_repo(path: &Path) -> Result<Repository, GitError> {
    match Repository::open(path) {
        Ok(repo) => Ok(repo),
        Err(_) => Ok(Repository::init(path)?),
    }
}

pub fn status(path: &Path) -> Result<Vec<GitFileStatus>, GitError> {
    let repo = ensure_repo(path)?;
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    Ok(statuses
        .iter()
        .filter_map(|entry| {
            let path = entry.path()?.to_string();
            let flags = entry.status();
            let status = if flags.is_wt_new() || flags.is_index_new() {
                "added"
            } else if flags.is_wt_deleted() || flags.is_index_deleted() {
                "deleted"
            } else if flags.is_wt_renamed() || flags.is_index_renamed() {
                "renamed"
            } else {
                "modified"
            };
            Some(GitFileStatus { path, status: status.to_string() })
        })
        .collect())
}

pub fn set_remote(path: &Path, owner_repo: &str) -> Result<(), GitError> {
    let repo = ensure_repo(path)?;
    let url = format!("https://github.com/{owner_repo}.git");

    if repo.find_remote("origin").is_ok() {
        repo.remote_set_url("origin", &url)?;
    } else {
        repo.remote("origin", &url)?;
    }
    Ok(())
}

fn remote_callbacks(token: &str) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    let token = token.to_string();
    callbacks.credentials(move |_url, _username_from_url, _allowed| {
        Cred::userpass_plaintext("x-access-token", &token)
    });
    callbacks
}

/// Fetches `origin` and fast-forwards (or, for a brand-new local repo, checks out)
/// the working directory to match. Used to pull an existing GitHub repo's content
/// into `content_directory` — e.g. importing writing that already lives on GitHub.
pub fn pull(path: &Path, token: &str) -> Result<(), GitError> {
    let repo = ensure_repo(path)?;

    {
        let mut remote = repo.find_remote("origin")?;
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(remote_callbacks(token));
        remote.fetch(&[] as &[&str], Some(&mut fetch_opts), None)?;
    }

    let branch = match repo.head().ok().and_then(|h| h.shorthand().map(String::from)) {
        Some(name) if repo.find_reference(&format!("refs/remotes/origin/{name}")).is_ok() => name,
        _ => ["main", "master"]
            .into_iter()
            .find(|candidate| {
                repo.find_reference(&format!("refs/remotes/origin/{candidate}")).is_ok()
            })
            .map(String::from)
            .ok_or(GitError::NoRemoteBranch)?,
    };

    let remote_ref = repo.find_reference(&format!("refs/remotes/origin/{branch}"))?;
    let fetch_commit = remote_ref.peel_to_commit()?;

    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();

    if repo.head().is_err() {
        // Fresh/empty local repo: adopt the remote branch directly.
        repo.branch(&branch, &fetch_commit, true)?;
        repo.set_head(&format!("refs/heads/{branch}"))?;
        repo.checkout_head(Some(&mut checkout))?;
        return Ok(());
    }

    let annotated = repo.find_annotated_commit(fetch_commit.id())?;
    let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    if analysis.is_up_to_date() {
        return Ok(());
    }

    if analysis.is_fast_forward() {
        let branch_ref_name = format!("refs/heads/{branch}");
        let mut branch_ref = repo
            .find_reference(&branch_ref_name)
            .or_else(|_| repo.reference(&branch_ref_name, fetch_commit.id(), true, "fast-forward import"))?;
        branch_ref.set_target(fetch_commit.id(), "fast-forward import")?;
        repo.set_head(&branch_ref_name)?;
        repo.checkout_head(Some(&mut checkout))?;
        return Ok(());
    }

    Err(GitError::DivergedHistory)
}

pub fn commit_and_push(
    path: &Path,
    message: &str,
    token: &str,
    author_name: &str,
    author_email: &str,
) -> Result<(), GitError> {
    let repo = ensure_repo(path)?;

    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let signature = Signature::now(author_name, author_email)?;

    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)?;

    let mut remote = repo.find_remote("origin")?;
    let branch = repo.head()?.shorthand().unwrap_or("main").to_string();
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");

    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(remote_callbacks(token));
    remote.push(&[&refspec], Some(&mut push_opts))?;

    Ok(())
}
