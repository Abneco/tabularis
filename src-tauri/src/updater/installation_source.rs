const CUSTOM_PACKAGE_MANAGER_SOURCE: Option<&str> = option_env!("PACKAGE_MANAGER_SRC");
const CUSTOM_PACKAGE_MANAGER_NAME: Option<&str> = option_env!("PACKAGE_MANAGER_NAME");

/// Homebrew Cask receipt directories left by `brew install --cask tabularis`.
const HOMEBREW_CASK_PATHS: &[&str] = &[
    "/opt/homebrew/Caskroom/tabularis",
    "/usr/local/Caskroom/tabularis",
];

/// WinGet package id prefix for Tabularis (`Debba.Tabularis` / `Debba.Tabularis_…`).
const WINGET_PACKAGE_ID_PREFIX: &str = "Debba.Tabularis";

fn custom_package_manager_name(source: Option<&str>, name: Option<&str>) -> Option<String> {
    let source = source?.trim();
    let name = name?.trim();

    if source.is_empty() || name.is_empty() {
        return None;
    }

    Some(name.to_string())
}

/// Returns the display name of the package manager responsible for updates.
/// Custom package metadata is embedded at compile time so installed builds do
/// not depend on environment variables being present when the app is launched.
pub(super) fn detect_installation_source() -> Option<String> {
    detect_installation_source_with(CUSTOM_PACKAGE_MANAGER_SOURCE, CUSTOM_PACKAGE_MANAGER_NAME)
}

fn detect_installation_source_with(
    custom_source: Option<&str>,
    custom_name: Option<&str>,
) -> Option<String> {
    if let Some(name) = custom_package_manager_name(custom_source, custom_name) {
        return Some(name);
    }

    #[cfg(target_os = "linux")]
    {
        // Snap sets the SNAP env var when running inside a snap sandbox.
        if std::env::var("SNAP").is_ok() {
            return Some("snap".to_string());
        }

        // Flatpak sets FLATPAK_ID when running inside a Flatpak sandbox.
        if std::env::var("FLATPAK_ID").is_ok() {
            return Some("flatpak".to_string());
        }

        // AUR: check if pacman's local database has a tabularis-bin entry.
        // Skipped in dev builds because an installed package alongside the dev
        // environment would otherwise be misdetected as the build source.
        if !cfg!(debug_assertions) {
            if let Ok(entries) = std::fs::read_dir("/var/lib/pacman/local") {
                let is_aur = entries.filter_map(|entry| entry.ok()).any(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("tabularis-bin-")
                });
                if is_aur {
                    return Some("aur".to_string());
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Skipped in debug builds: a local `brew install --cask tabularis`
        // next to a checkout would otherwise false-positive as the build source.
        if !cfg!(debug_assertions) {
            if let Some(name) =
                detect_homebrew_cask_with(|path| std::path::Path::new(path).is_dir())
            {
                return Some(name);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Skipped in debug builds for the same reason as AUR / Homebrew.
        if !cfg!(debug_assertions) {
            if let Some(name) = detect_winget_runtime() {
                return Some(name);
            }
        }
    }

    None
}

/// Detect a Homebrew cask install by checking Caskroom receipt paths.
///
/// `path_exists` is injected so unit tests can exercise the logic without
/// creating real directories under `/opt/homebrew` or `/usr/local`.
fn detect_homebrew_cask_with(path_exists: impl Fn(&str) -> bool) -> Option<String> {
    if HOMEBREW_CASK_PATHS.iter().copied().any(path_exists) {
        Some("Homebrew".to_string())
    } else {
        None
    }
}

/// True when `packages_dir` contains a folder whose name starts with the
/// Tabularis WinGet package id (e.g. `Debba.Tabularis_Microsoft.Winget.Source_…`).
fn winget_packages_dir_has_tabularis(packages_dir: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(packages_dir) else {
        return false;
    };
    entries.filter_map(|entry| entry.ok()).any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with(WINGET_PACKAGE_ID_PREFIX)
    })
}

/// True when an absolute executable path sits under a WinGet Packages tree for
/// Debba.Tabularis (portable / Links layout).
fn exe_path_looks_like_winget(exe: &str) -> bool {
    let normalized = exe.replace('/', "\\").to_ascii_lowercase();
    normalized.contains("\\microsoft\\winget\\packages\\debba.tabularis")
}

/// Detect WinGet using injectable env roots and directory/exe predicates.
///
/// Covers portable WinGet layouts under `%LOCALAPPDATA%\Microsoft\WinGet\Packages`
/// and machine roots under `Program Files\WinGet\Packages`. Traditional NSIS
/// installer packages submitted via `.github/workflows/winget.yml` may not leave
/// those folders; for those, prefer baking `PACKAGE_MANAGER_SRC` /
/// `PACKAGE_MANAGER_NAME` into a WinGet-channel build (see PR notes).
fn detect_winget_with(
    local_app_data: Option<&str>,
    program_files: Option<&str>,
    program_files_x86: Option<&str>,
    packages_dir_has_tabularis: impl Fn(&str) -> bool,
    current_exe: Option<&str>,
) -> Option<String> {
    let mut package_roots = Vec::new();

    if let Some(local_app_data) = local_app_data.filter(|value| !value.is_empty()) {
        package_roots.push(format!(
            "{local_app_data}\\Microsoft\\WinGet\\Packages"
        ));
    }
    if let Some(program_files) = program_files.filter(|value| !value.is_empty()) {
        package_roots.push(format!("{program_files}\\WinGet\\Packages"));
    }
    if let Some(program_files_x86) = program_files_x86.filter(|value| !value.is_empty()) {
        package_roots.push(format!("{program_files_x86}\\WinGet\\Packages"));
    }

    if package_roots
        .iter()
        .any(|root| packages_dir_has_tabularis(root))
    {
        return Some("WinGet".to_string());
    }

    if let Some(exe) = current_exe {
        if exe_path_looks_like_winget(exe) {
            return Some("WinGet".to_string());
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn detect_winget_runtime() -> Option<String> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok();
    let program_files = std::env::var("ProgramFiles").ok();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").ok();
    let current_exe = std::env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().into_owned());

    detect_winget_with(
        local_app_data.as_deref(),
        program_files.as_deref(),
        program_files_x86.as_deref(),
        winget_packages_dir_has_tabularis,
        current_exe.as_deref(),
    )
}

/// Returns true when updates should not be managed by the app itself.
pub(super) fn is_managed_package() -> bool {
    detect_installation_source().is_some()
}

#[cfg(test)]
mod tests;
