//! `upgrade` command semantics (seeds-d54c step 4, seeds-bdcb):
//! in-process self-update from the GitHub Releases.
//!
//! Deliberately NOT the `self_update` crate: the decided contract
//! (d54c round 2) demands SHASUMS.txt verification ON TOP OF TLS, and
//! `self_update` does not expose its download for verification - the
//! outcome requirement wins over the named means (README DEVIATIONS).
//! The stack stays minimal and blocking: ureq + rustls(ring),
//! webpki-roots, sha2, flate2(miniz_oxide), tar - all behind the
//! `upgrade` feature so the core stays offline-pure without it.
//!
//! UX mirrors the reference surface: `--check` never installs and
//! exits 1 when outdated (envelope still reports success:true - the
//! check succeeded; the exit code is the machine signal),
//! `--json` carries {current, latest, upToDate}. Guards classify the
//! running binary's install class first: image-baked (/usr, /opt),
//! cargo-installed (~/.cargo/bin), mise-managed - those report the
//! right upgrade channel instead of self-replacing.

use std::io::Read;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{CommandOutcome, envelope_pretty, push_line};

/// The release channel (operator decision: GitHub Releases on
/// denkhaus/seeds).
const REPO: &str = "denkhaus/seeds";

/// Numeric dotted-version ordering: `0.3.0` vs `0.3.1`. Suffixes
/// (pre-release tags) compare lexicographically as a tiebreak.
fn version_key(version: &str) -> Vec<u64> {
    version
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

/// Whether `current` is at least `latest` (a locally newer build
/// counts as current, not outdated).
#[must_use]
fn same_version(current: &str, latest: &str) -> bool {
    version_key(current) >= version_key(latest)
}

/// `seeds upgrade`: self-update from the releases.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UpgradeInput {
    /// Whether to only check for a newer release without installing.
    pub check: bool,
    /// Whether output is the JSON envelope.
    pub json:  bool,
}

/// Runs `upgrade`: resolve the latest release, then check or perform.
#[must_use = "the outcome only takes effect when the caller prints it"]
pub fn upgrade(input: &UpgradeInput) -> CommandOutcome {
    let current = env!("CARGO_PKG_VERSION").to_owned();
    let latest = match latest_version() {
        Ok(latest) => latest,
        Err(error) => return transport_error(&error, input.json),
    };
    let up_to_date = same_version(&current, &latest);

    if input.check {
        return check_outcome(&current, &latest, up_to_date, input.json);
    }
    if up_to_date {
        return report_update("unchanged", &current, &latest, None, input.json);
    }

    // Guards classify the running binary BEFORE any download: managed
    // installs upgrade through their own channel.
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            return transport_error(
                &format!("cannot locate the running binary: {error}"),
                input.json,
            );
        }
    };
    match classify_install(&exe) {
        InstallClass::SelfManaged => match perform_update(&exe, &current, &latest) {
            Ok(()) => report_update("updated", &current, &latest, Some(&exe), input.json),
            Err(error) => transport_error(&error, input.json),
        },
        class => guard_outcome(class, &exe, input.json),
    }
}

// ---------------------------------------------------------------------------
// release discovery + download
// ---------------------------------------------------------------------------

/// One shared agent: the `rustls` feature gives TLS with Mozilla's
/// root set by default; a seeds User-Agent is required by the GitHub
/// API.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(concat!("seeds/", env!("CARGO_PKG_VERSION")))
        .build()
        .new_agent()
}

/// The latest release's version (v-prefix stripped).
fn latest_version() -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let response = agent()
        .get(&url)
        .call()
        .map_err(|error| format!("cannot reach {url}: {error} (offline?)"))?;
    let mut body = response.into_body();
    let body = body
        .read_to_string()
        .map_err(|error| format!("cannot read the release listing: {error}"))?;
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| format!("malformed release listing: {error}"))?;
    let tag = value["tag_name"]
        .as_str()
        .ok_or_else(|| "release listing carries no tag_name".to_owned())?;
    Ok(tag.strip_prefix('v').unwrap_or(tag).to_owned())
}

fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = agent()
        .get(url)
        .call()
        .map_err(|error| format!("cannot reach {url}: {error} (offline?)"))?;
    let mut response_body = response.into_body();
    let bytes = response_body
        .read_to_vec()
        .map_err(|error| format!("cannot download {url}: {error}"))?;
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// update mechanics
// ---------------------------------------------------------------------------

/// The release target triple for the RUNNING binary. A glibc-host
/// binary is replaced by the musl-static artifact on purpose: the
/// catalog is musl-only and static runs everywhere.
fn target_triple() -> Result<String, String> {
    let os = match std::env::consts::OS {
        "linux" => "unknown-linux-musl",
        "macos" => "apple-darwin",
        other => {
            return Err(format!(
                "unsupported OS '{other}' (supported: linux, macos)"
            ));
        }
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => {
            return Err(format!(
                "unsupported architecture '{other}' (supported: x86_64, aarch64)"
            ));
        }
    };
    Ok(format!("{arch}-{os}"))
}

/// Downloads, SHASUM-verifies, extracts, and installs the release
/// binary over `exe` (write-beside + atomic rename).
fn perform_update(exe: &Path, current: &str, latest: &str) -> Result<(), String> {
    let target = target_triple()?;
    let asset = format!("seeds-{latest}-{target}.tar.gz");
    let base = format!("https://github.com/{REPO}/releases/download/v{latest}");

    let sums = String::from_utf8(http_get_bytes(&format!("{base}/SHASUMS.txt"))?)
        .map_err(|_| "SHASUMS.txt is not valid UTF-8".to_owned())?;
    let expected = sums
        .lines()
        .find(|line| line.ends_with(&asset))
        .and_then(|line| line.split_whitespace().next())
        .ok_or_else(|| format!("{asset} is not listed in the release's SHASUMS.txt"))?
        .to_owned();

    let bytes = http_get_bytes(&format!("{base}/{asset}"))?;
    let digest = hex(&Sha256::digest(&bytes));
    if digest != expected {
        return Err(format!(
            "checksum mismatch for {asset} (expected {expected}, got {digest})"
        ));
    }

    let binary = extract_root_binary(&bytes, "seeds")?;
    install_over(exe, &binary)?;
    let _ = (current, latest);
    Ok(())
}

/// Extracts the archive entry named `name` at the archive root.
fn extract_root_bytes(archive: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let reader = std::io::Cursor::new(archive);
    let gz = flate2::read::GzDecoder::new(reader);
    let mut tar = tar::Archive::new(gz);
    let entries = tar
        .entries()
        .map_err(|error| format!("cannot read the release archive: {error}"))?;
    for entry in entries {
        let mut entry = entry.map_err(|error| format!("damaged archive entry: {error}"))?;
        if entry.path().map_or(true, |path| path != Path::new(name)) {
            continue;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot extract {name}: {error}"))?;
        return Ok(bytes);
    }
    Err(format!("archive has no root entry '{name}'"))
}

fn extract_root_binary(archive: &[u8], name: &str) -> Result<Vec<u8>, String> {
    extract_root_bytes(archive, name)
}

/// Writes the new binary beside the current one, marks it executable,
/// and renames it over the running binary (atomic on Linux and macOS).
fn install_over(exe: &Path, bytes: &[u8]) -> Result<(), String> {
    let staged = exe.with_extension("seeds-new");
    std::fs::write(&staged, bytes)
        .map_err(|error| format!("cannot stage {}: {error}", staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("cannot mark the staged binary executable: {error}"))?;
    }
    std::fs::rename(&staged, exe)
        .map_err(|error| format!("cannot replace {}: {error}", exe.display()))?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

// ---------------------------------------------------------------------------
// install classification (guards)
// ---------------------------------------------------------------------------

/// How the running binary was installed; decides the upgrade channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallClass {
    /// Baked into the toolchain image (/usr, /opt): upgrade the image.
    ImageBaked,
    /// cargo install copy: upgrade through cargo.
    CargoInstall,
    /// mise-managed copy: upgrade through mise.
    Mise,
    /// Self-managed (install.sh, custom prefix): self-update in place.
    SelfManaged,
}

/// Classifies an executable path into its upgrade channel.
#[must_use]
pub fn classify_install(exe: &Path) -> InstallClass {
    let path = exe.to_string_lossy();
    if path.starts_with("/usr/") || path.starts_with("/opt/") {
        InstallClass::ImageBaked
    } else if path.contains("/.cargo/bin/") {
        InstallClass::CargoInstall
    } else if path.contains("/.local/share/mise") {
        InstallClass::Mise
    } else {
        InstallClass::SelfManaged
    }
}

fn guard_outcome(class: InstallClass, exe: &Path, json: bool) -> CommandOutcome {
    let (action, message) = match class {
        InstallClass::ImageBaked => (
            "noop-image-baked",
            format!(
                "no self-update: {} is baked into the toolchain image - rebuild the image instead (just run-images)",
                exe.display()
            ),
        ),
        InstallClass::CargoInstall => (
            "noop-cargo-install",
            "no self-update: cargo-installed copy - upgrade via `cargo install --locked seeds` (checkout: `cargo install --path crates/seeds` or `just install`)".to_owned(),
        ),
        InstallClass::Mise => (
            "noop-mise",
            "no self-update: mise-managed copy - upgrade via `mise upgrade cargo:seeds`".to_owned(),
        ),
        InstallClass::SelfManaged => unreachable!("self-managed installs self-update"),
    };
    if json {
        let stdout = envelope_pretty("upgrade", &[
            ("action", json!(action)),
            ("file", Value::from(exe.display().to_string())),
        ]);
        let mut text = String::new();
        push_line(&mut text, &stdout);
        return CommandOutcome {
            success: true,
            stdout:  text,
            stderr:  String::new(),
        };
    }
    let mut stdout = String::new();
    push_line(&mut stdout, &message);
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

// ---------------------------------------------------------------------------
// reporting
// ---------------------------------------------------------------------------

fn check_outcome(current: &str, latest: &str, up_to_date: bool, json: bool) -> CommandOutcome {
    if json {
        let stdout = envelope_pretty("upgrade", &[
            ("current", json!(current)),
            ("latest", json!(latest)),
            ("upToDate", json!(up_to_date)),
        ]);
        let mut text = String::new();
        push_line(&mut text, &stdout);
        return CommandOutcome {
            // The check itself succeeded even when outdated - the exit
            // code below is the machine signal (decided: --check exits
            // 1 when outdated).
            success: up_to_date,
            stdout:  text,
            stderr:  String::new(),
        };
    }
    let mut stdout = String::new();
    if up_to_date {
        push_line(&mut stdout, &format!("Status: current (v{current})"));
    } else {
        push_line(
            &mut stdout,
            &format!("Status: outdated (current v{current}, latest v{latest})"),
        );
    }
    CommandOutcome {
        success: up_to_date,
        stdout,
        stderr: String::new(),
    }
}

fn report_update(
    action: &str,
    current: &str,
    latest: &str,
    exe: Option<&Path>,
    json: bool,
) -> CommandOutcome {
    if json {
        let mut extra = vec![
            ("action", json!(action)),
            ("current", json!(current)),
            ("latest", json!(latest)),
        ];
        if let Some(exe) = exe {
            extra.push(("file", Value::from(exe.display().to_string())));
        }
        let stdout = envelope_pretty("upgrade", &extra);
        let mut text = String::new();
        push_line(&mut text, &stdout);
        return CommandOutcome {
            success: true,
            stdout:  text,
            stderr:  String::new(),
        };
    }
    let mut stdout = String::new();
    match action {
        "updated" => push_line(
            &mut stdout,
            &format!("✓ Updated to v{latest} (was v{current})"),
        ),
        _ => push_line(&mut stdout, &format!("✓ seeds v{current} is up to date")),
    }
    if let Some(exe) = exe {
        push_line(&mut stdout, &format!("  {}", exe.display()));
    }
    CommandOutcome {
        success: true,
        stdout,
        stderr: String::new(),
    }
}

fn transport_error(message: &str, _json: bool) -> CommandOutcome {
    CommandOutcome {
        success: false,
        stdout:  String::new(),
        stderr:  format!("Error: {message}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ordering_matches_expectations() {
        assert!(same_version("0.3.0", "0.3.0"));
        assert!(same_version("0.4.0", "0.3.0"), "local newer is current");
        assert!(!same_version("0.3.0", "0.3.1"));
        assert!(!same_version("0.3.0", "0.4.0"));
        assert!(!same_version("0.9.0", "0.10.0"), "numeric, not lexical");
    }

    #[test]
    fn install_classes_route_by_path() {
        assert_eq!(
            classify_install(Path::new("/usr/local/bin/seeds")),
            InstallClass::ImageBaked
        );
        assert_eq!(
            classify_install(Path::new("/home/u/.cargo/bin/seeds")),
            InstallClass::CargoInstall
        );
        assert_eq!(
            classify_install(Path::new(
                "/home/u/.local/share/mise/installs/cargo-seeds/0.3.0/bin/seeds"
            )),
            InstallClass::Mise
        );
        assert_eq!(
            classify_install(Path::new("/home/u/.local/bin/seeds")),
            InstallClass::SelfManaged
        );
    }
}
