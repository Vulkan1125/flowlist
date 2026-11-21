//! Settings → About: the version on this machine, where the library lives,
//! and a real update check against the project's GitHub releases.
//!
//! The check shells out to `curl`, which every Mac has, so the app stays free
//! of an HTTP dependency. Installing downloads the release zip, stages the
//! `.app`, swaps it in beside the running bundle, and offers a restart. When
//! the app is not running from a bundle (a `target/release/flowlist` binary),
//! it falls back to opening the release page.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, div, prelude::*};

use super::icons::Icon;
use super::{FlowlistApp, icons, widgets};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum UpdateState {
    Idle,
    Checking,
    UpToDate {
        current: String,
    },
    Available {
        current: String,
        version: String,
        release_url: String,
        asset: Option<String>,
    },
    Installing,
    Ready {
        version: String,
        bundle: PathBuf,
    },
    Failed(String),
}

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(serde::Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

impl FlowlistApp {
    pub(super) fn render_about_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let (status, color) = match &self.update {
            UpdateState::Idle => (
                "Check GitHub for a newer build.".to_string(),
                theme.text_faint,
            ),
            UpdateState::Checking => ("Checking GitHub…".to_string(), theme.text_faint),
            UpdateState::UpToDate { current } => (
                format!("Flowlist {current} is the latest version."),
                theme.success,
            ),
            UpdateState::Available {
                current, version, ..
            } => (
                format!("Flowlist {version} is available; you have {current}."),
                theme.accent,
            ),
            UpdateState::Installing => (
                "Downloading and installing the update…".to_string(),
                theme.text_faint,
            ),
            UpdateState::Ready { version, .. } => (
                format!("Flowlist {version} is installed. Restart to use it."),
                theme.success,
            ),
            UpdateState::Failed(message) => (message.clone(), theme.danger),
        };
        let busy = matches!(self.update, UpdateState::Checking | UpdateState::Installing);
        let mut actions = div().flex().items_center().gap_2();
        if !busy {
            actions = actions.child(
                div()
                    .id("update-check")
                    .child(widgets::button(theme, "Check for updates", false))
                    .on_click(cx.listener(|this, _, _, cx| this.check_for_updates(cx))),
            );
        }
        if let UpdateState::Available {
            release_url, asset, ..
        } = &self.update
        {
            if let Some(asset) = asset {
                let asset = asset.clone();
                actions = actions.child(
                    div()
                        .id("update-install")
                        .child(widgets::button(theme, "Install update", true))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.install_update(asset.clone(), cx)
                        })),
                );
            }
            let release_url = release_url.clone();
            actions = actions.child(
                div()
                    .id("update-release")
                    .child(widgets::button(
                        theme,
                        if asset.is_some() {
                            "Release notes"
                        } else {
                            "Open download page"
                        },
                        false,
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        open_url(&release_url);
                        this.toast = Some(format!("Opening {release_url}"));
                        cx.notify();
                    })),
            );
        }
        if let UpdateState::Ready { bundle, .. } = &self.update {
            let bundle = bundle.clone();
            actions = actions.child(
                div()
                    .id("update-restart")
                    .child(widgets::button(theme, "Restart Flowlist", true))
                    .on_click(cx.listener(move |_this, _, _, cx| {
                        let _ = std::process::Command::new("open").arg(&bundle).spawn();
                        cx.quit();
                    })),
            );
        }
        let updates = self.settings_card(
            "Updates",
            Some("The check reads the latest published GitHub release; nothing is sent back."),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_sm().text_color(color).child(status))
                .child(actions),
        );
        let links = self.settings_card(
            "Links",
            None,
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .id("about-github")
                        .child(widgets::button(theme, "GitHub", false))
                        .on_click(cx.listener(|this, _, _, cx| {
                            open_url(REPOSITORY);
                            this.toast = Some("Opening the repository".to_string());
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .id("about-wiki")
                        .child(widgets::button(theme, "Wiki", false))
                        .on_click(cx.listener(|this, _, _, cx| {
                            open_url(&format!("{REPOSITORY}/wiki"));
                            this.toast = Some("Opening the wiki".to_string());
                            cx.notify();
                        })),
                ),
        );
        let library = self.store.paths().root.display().to_string();
        let about = self.settings_card(
            "Flowlist",
            None,
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(icons::icon(Icon::Info, 14.0, theme.text_faint))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text)
                                .child(format!("Version {CURRENT_VERSION}")),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child("Local-first: the library stays on this machine."),
                )
                .child(
                    div()
                        .mt_1()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child(format!("Library: {library}")),
                ),
        );
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(about)
            .child(updates)
            .child(links)
            .into_any_element()
    }

    pub(super) fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if matches!(self.update, UpdateState::Checking | UpdateState::Installing) {
            return;
        }
        self.update = UpdateState::Checking;
        cx.notify();
        let slot: Arc<Mutex<Option<UpdateState>>> = Arc::new(Mutex::new(None));
        let worker = slot.clone();
        std::thread::spawn(move || {
            let state = match fetch_latest() {
                Ok(Some((version, release_url, asset))) => UpdateState::Available {
                    current: CURRENT_VERSION.to_string(),
                    version,
                    release_url,
                    asset,
                },
                Ok(None) => UpdateState::UpToDate {
                    current: CURRENT_VERSION.to_string(),
                },
                Err(message) => UpdateState::Failed(message),
            };
            if let Ok(mut slot) = worker.lock() {
                *slot = Some(state);
            }
        });
        Self::poll_update_worker(slot, cx);
    }

    fn install_update(&mut self, asset: String, cx: &mut Context<Self>) {
        if matches!(self.update, UpdateState::Checking | UpdateState::Installing) {
            return;
        }
        let Some(bundle) = current_bundle() else {
            self.update = UpdateState::Failed(
                "This build is not inside Flowlist.app; download the release and replace it by hand."
                    .to_string(),
            );
            cx.notify();
            return;
        };
        self.update = UpdateState::Installing;
        cx.notify();
        let version = match &self.update {
            UpdateState::Available { version, .. } => version.clone(),
            _ => String::new(),
        };
        let slot: Arc<Mutex<Option<UpdateState>>> = Arc::new(Mutex::new(None));
        let worker = slot.clone();
        std::thread::spawn(move || {
            let state = match install(&asset, &bundle) {
                Ok(bundle) => UpdateState::Ready { version, bundle },
                Err(message) => UpdateState::Failed(message),
            };
            if let Ok(mut slot) = worker.lock() {
                *slot = Some(state);
            }
        });
        Self::poll_update_worker(slot, cx);
    }

    /// Network and disk work happens on a plain thread; the window polls the
    /// shared slot so the frame never blocks on `curl` or `ditto`.
    fn poll_update_worker(slot: Arc<Mutex<Option<UpdateState>>>, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            for _ in 0..1500 {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                let state = slot.lock().ok().and_then(|mut slot| slot.take());
                if let Some(state) = state {
                    let ready = matches!(state, UpdateState::Ready { .. });
                    this.update(cx, |this, cx| {
                        this.update = state;
                        if ready {
                            this.toast = Some(
                                "Update installed — restart Flowlist from Settings → About."
                                    .to_string(),
                            );
                        }
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            }
            this.update(cx, |this, cx| {
                this.update = UpdateState::Failed("The update timed out.".to_string());
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

fn open_url(url: &str) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(url).spawn();
}

/// `Ok(Some(..))` when a newer release exists, `Ok(None)` when this build is
/// current, and `Err` with a sentence the sheet can show when the check failed.
fn fetch_latest() -> Result<Option<(String, String, Option<String>)>, String> {
    let api = api_url().ok_or_else(|| "The repository URL is not a GitHub link.".to_string())?;
    let output = std::process::Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--max-time",
            "15",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            &format!("User-Agent: Flowlist/{CURRENT_VERSION}"),
            "-w",
            "\n%{http_code}",
            &api,
        ])
        .output()
        .map_err(|error| format!("Could not run curl: {error}"))?;
    if !output.status.success() && output.stdout.is_empty() {
        return Err("Could not reach GitHub. Check the connection and try again.".to_string());
    }
    let body = String::from_utf8_lossy(&output.stdout);
    let (json, code) = body
        .rsplit_once('\n')
        .map(|(json, code)| (json, code.trim()))
        .unwrap_or((body.as_ref(), ""));
    match code {
        "200" => {}
        "404" => return Err("No release has been published yet.".to_string()),
        "" => {
            return Err("Could not reach GitHub. Check the connection and try again.".to_string());
        }
        other => return Err(format!("GitHub answered with HTTP {other}.")),
    }
    let release: Release = serde_json::from_str(json)
        .map_err(|_| "GitHub returned a release the app could not read.".to_string())?;
    if !is_newer(&release.tag_name, CURRENT_VERSION) {
        return Ok(None);
    }
    // Prefer a zip: the app bundle travels as one, and `ditto` unpacks it
    // without asking the user anything.
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name.to_ascii_lowercase().ends_with(".zip"))
        .map(|asset| asset.browser_download_url.clone());
    let release_url = if release.html_url.is_empty() {
        format!("{REPOSITORY}/releases")
    } else {
        release.html_url
    };
    Ok(Some((release.tag_name, release_url, asset)))
}

fn api_url() -> Option<String> {
    let path = REPOSITORY.strip_prefix("https://github.com/")?;
    let path = path.trim_end_matches('/');
    Some(format!(
        "https://api.github.com/repos/{path}/releases/latest"
    ))
}

/// Numeric dotted versions only; a prerelease suffix compares on its base
/// (`0.2.0-beta.1` is newer than `0.1.0`). Unreadable tags never count as an
/// update, so a bad release name cannot trigger an install.
fn version_key(value: &str) -> Option<Vec<u64>> {
    let value = value.trim().trim_start_matches(['v', 'V']);
    let base = value.split(['-', '+']).next().unwrap_or(value);
    let mut parts: Vec<u64> = base
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<_>>()?;
    while parts.len() < 3 {
        parts.push(0);
    }
    Some(parts)
}

fn is_newer(latest: &str, current: &str) -> bool {
    matches!((version_key(latest), version_key(current)), (Some(latest), Some(current)) if latest > current)
}

/// The `.app` this process is running from, if any. A plain binary in
/// `target/release` has no bundle to replace.
fn current_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(Path::to_path_buf)
}

fn install(asset: &str, bundle: &Path) -> Result<PathBuf, String> {
    let work = std::env::temp_dir().join(format!("flowlist-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)
        .map_err(|error| format!("Could not create a download folder: {error}"))?;
    let zip = work.join("update.zip");
    let status = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "300", "-o"])
        .arg(&zip)
        .arg(asset)
        .status()
        .map_err(|error| format!("Could not run curl: {error}"))?;
    if !status.success() {
        return Err("The update download failed. Try again from the release page.".to_string());
    }
    let result = unpack_and_swap(&zip, bundle);
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// Unpack a downloaded release zip and swap its `.app` in beside the running
/// bundle. Split from the download so the swap itself can be tested with a
/// local archive.
fn unpack_and_swap(zip: &Path, bundle: &Path) -> Result<PathBuf, String> {
    let parent = bundle
        .parent()
        .ok_or_else(|| "The app bundle has no parent folder.".to_string())?;
    let downloads = zip.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let extracted = downloads.join("extracted");
    let _ = std::fs::remove_dir_all(&extracted);
    std::fs::create_dir_all(&extracted)
        .map_err(|error| format!("Could not unpack the update: {error}"))?;
    let status = std::process::Command::new("ditto")
        .args(["-x", "-k"])
        .arg(zip)
        .arg(&extracted)
        .status()
        .map_err(|error| format!("Could not unpack the update: {error}"))?;
    if !status.success() {
        return Err("The update archive could not be unpacked.".to_string());
    }
    let app = find_app(&extracted, 2)
        .ok_or_else(|| "The update archive held no Flowlist.app.".to_string())?;
    let executable = app.join("Contents/MacOS/flowlist");
    if !executable.is_file() {
        return Err("The downloaded app is missing its executable.".to_string());
    }
    let staged = parent.join("Flowlist.app.updating");
    let backup = parent.join("Flowlist.app.previous");
    let _ = std::fs::remove_dir_all(&staged);
    let _ = std::fs::remove_dir_all(&backup);
    let status = std::process::Command::new("ditto")
        .arg(&app)
        .arg(&staged)
        .status()
        .map_err(|error| format!("Could not stage the update: {error}"))?;
    if !status.success() {
        return Err("Could not stage the downloaded app.".to_string());
    }
    std::fs::rename(bundle, &backup)
        .map_err(|error| format!("Could not move the running app aside: {error}"))?;
    if let Err(error) = std::fs::rename(&staged, bundle) {
        let _ = std::fs::rename(&backup, bundle);
        return Err(format!("Could not put the update in place: {error}"));
    }
    let _ = std::fs::remove_dir_all(&backup);
    Ok(bundle.to_path_buf())
}

/// Find the first `.app` in an unpacked archive. Release zips usually put
/// `Flowlist.app` at the root, but a wrapper folder is allowed.
fn find_app(directory: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let mut folders = Vec::new();
    for entry in std::fs::read_dir(directory).ok()?.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "app") {
            return Some(path);
        }
        if path.is_dir() {
            folders.push(path);
        }
    }
    folders
        .into_iter()
        .find_map(|folder| find_app(&folder, depth - 1))
}

#[cfg(test)]
mod tests {
    use super::{is_newer, version_key};

    #[cfg(target_os = "macos")]
    #[test]
    fn a_downloaded_bundle_replaces_the_running_one() {
        use std::fs;

        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("Flowlist.app");
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        fs::write(bundle.join("Contents/MacOS/flowlist"), "old").unwrap();
        fs::write(bundle.join("Contents/Info.plist"), "old").unwrap();

        let source = root.path().join("source");
        let app = source.join("Flowlist.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/MacOS/flowlist"), "new").unwrap();
        fs::write(app.join("Contents/Info.plist"), "new").unwrap();
        let zip = root.path().join("update.zip");
        let status = std::process::Command::new("ditto")
            .args(["-c", "-k"])
            .arg(&source)
            .arg(&zip)
            .status()
            .unwrap();
        assert!(status.success());

        let installed = super::unpack_and_swap(&zip, &bundle).unwrap();
        assert_eq!(installed, bundle);
        assert_eq!(
            fs::read_to_string(bundle.join("Contents/MacOS/flowlist")).unwrap(),
            "new"
        );
        assert!(!root.path().join("Flowlist.app.previous").exists());
        assert!(!root.path().join("Flowlist.app.updating").exists());
    }

    #[test]
    fn versions_compare_numerically_and_ignore_the_v_prefix() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(is_newer("v0.2.0-beta.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("release", "0.1.0"));
        assert_eq!(version_key("v1.2"), Some(vec![1, 2, 0]));
    }
}
