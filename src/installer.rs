use std::fs;
use std::path::{Path, PathBuf};

use zed_extension_api::{self as zed, LanguageServerId, Result, serde_json::Value};

use crate::platform::{self, NativePlatform};

// Published on npm, including all eight native optional dependencies.
// Layout: https://github.com/oxc-project/tsrs/blob/main/npm/build.mjs
pub const DEFAULT_PACKAGE_VERSION: &str = "0.25.0-ts7.1.0-beta";
const PACKAGE: &str = "tsrs";

pub fn managed_binary(id: &LanguageServerId, version: &str) -> Result<String> {
    validate_version(version)?;
    zed::set_language_server_installation_status(
        id,
        &zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );
    let platform = platform::current()?;
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Cannot locate Zed's tsrs extension directory: {e}"))?;
    let cache = cwd
        .join("tsrs-versions")
        .join(version)
        .join(platform.package.trim_start_matches("@tsrs/"));
    if let Ok(binary) = validate_install(&cache, version, &platform) {
        return Ok(binary.to_string_lossy().into_owned());
    }

    zed::set_language_server_installation_status(
        id,
        &zed::LanguageServerInstallationStatus::Downloading,
    );

    let scratch = cwd.join("node_modules");
    for package in [PACKAGE, platform.package.as_str()] {
        remove_dir(&scratch.join(package))?;
    }
    zed::npm_install_package(PACKAGE, version).map_err(|e| {
        format!(
            "Failed to install tsrs@{version} through Zed's npm runtime: {e}. Check network/registry access and that this exact version is published."
        )
    })?;
    let installed = zed::npm_package_installed_version(PACKAGE)
        .map_err(|e| format!("Cannot inspect the installed tsrs npm version: {e}"))?;
    if installed.as_deref() != Some(version) {
        return Err(format!(
            "Requested tsrs@{version}, but npm reports {installed:?}; refusing to launch a different version."
        ));
    }
    validate_install(&cwd, version, &platform).map_err(|e| {
        format!(
            "npm installed tsrs@{version}, but its native distribution is incomplete: {e}. Ensure npm optional dependencies are enabled (remove omit=optional/no-optional) and this release publishes {}.",
            platform.package
        )
    })?;

    let staging = cache.with_extension("installing");
    remove_dir(&staging)?;
    let modules = staging.join("node_modules");
    fs::create_dir_all(modules.join("@tsrs"))
        .map_err(|e| format!("Cannot create tsrs cache {}: {e}", staging.display()))?;
    // Move entire packages, including LICENSE/NOTICE and every default library,
    // not just the executable. No cwd mutation or npm wrapper is involved.
    for package in [PACKAGE, platform.package.as_str()] {
        let source = scratch.join(package);
        let destination = modules.join(package);
        fs::rename(&source, &destination).map_err(|e| {
            format!(
                "Cannot move installed {package} from {} to {}: {e}. Check extension-directory permissions and retry.",
                source.display(), destination.display()
            )
        })?;
    }

    validate_install(&staging, version, &platform)?;
    remove_dir(&cache)?;

    fs::rename(&staging, &cache)
        .map_err(|e| format!("Cannot publish tsrs cache {}: {e}", cache.display()))?;

    let binary = validate_install(&cache, version, &platform)?;
    Ok(binary.to_string_lossy().into_owned())
}

fn validate_install(root: &Path, version: &str, platform: &NativePlatform) -> Result<PathBuf> {
    let modules = root.join("node_modules");
    let main = package_manifest(&modules.join(PACKAGE), PACKAGE, version)?;
    if main
        .get("optionalDependencies")
        .and_then(|deps| deps.get(&platform.package))
        .and_then(Value::as_str)
        != Some(version)
    {
        return Err(format!(
            "tsrs@{version} does not declare {}@{version} as an optional dependency",
            platform.package
        ));
    }
    let native = modules.join(&platform.package);
    let manifest = package_manifest(&native, &platform.package, version)?;
    let binary = native.join(platform.executable);
    nonempty_file(&binary)?;

    let files = manifest
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "{} has no published asset list in package.json",
                platform.package
            )
        })?;
    let mut libraries = 0;

    for file in files.iter().filter_map(Value::as_str) {
        if file.starts_with("lib.") && file.ends_with(".d.ts") {
            if file.contains(['/', '\\']) {
                return Err(format!("Invalid tsrs library asset name {file:?}"));
            }
            nonempty_file(&native.join(file))?;
            libraries += 1;
        }
    }
    if libraries == 0 {
        return Err(format!(
            "{} contains no default library assets",
            native.display()
        ));
    }
    for required in [
        "lib.d.ts",
        "lib.es5.d.ts",
        "lib.dom.d.ts",
        "lib.esnext.d.ts",
    ] {
        nonempty_file(&native.join(required))?;
    }
    Ok(binary)
}

fn package_manifest(directory: &Path, name: &str, version: &str) -> Result<Value> {
    let path = directory.join("package.json");
    let bytes = fs::read(&path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let manifest: Value = zed::serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid npm manifest {}: {e}", path.display()))?;
    if manifest.get("name").and_then(Value::as_str) != Some(name)
        || manifest.get("version").and_then(Value::as_str) != Some(version)
    {
        return Err(format!(
            "{} is not the requested {name}@{version}",
            path.display()
        ));
    }
    Ok(manifest)
}

fn nonempty_file(path: &Path) -> Result<()> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() > 0 => Ok(()),
        Ok(_) => Err(format!(
            "Required tsrs asset {} is empty or not a file",
            path.display()
        )),
        Err(e) => Err(format!(
            "Required tsrs asset {} is missing or unreadable: {e}",
            path.display()
        )),
    }
}

fn remove_dir(path: &Path) -> Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "Cannot remove incomplete tsrs installation {}: {e}",
            path.display()
        )),
    }
}

fn validate_version(version: &str) -> Result<()> {
    // Exact pins only: tags/ranges could resolve differently while sharing the
    // same cache key. Also keep all user input inside one safe path component.
    let (release, build) = version
        .split_once('+')
        .map_or((version, None), |(release, build)| (release, Some(build)));
    let (core, prerelease) = release
        .split_once('-')
        .map_or((release, None), |(core, prerelease)| {
            (core, Some(prerelease))
        });
    let parts: Vec<_> = core.split('.').collect();
    // npm's semver implementation limits core numbers to JS safe integers.
    let valid_number = |part: &&str| {
        !part.is_empty()
            && part.bytes().all(|b| b.is_ascii_digit())
            && (part.len() == 1 || !part.starts_with('0'))
            && part
                .parse::<u64>()
                .is_ok_and(|n| n <= 9_007_199_254_740_991)
    };
    let valid_identifiers = |identifiers: &str, prerelease: bool| {
        identifiers.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && (!prerelease
                    || !identifier.bytes().all(|b| b.is_ascii_digit())
                    || identifier.len() == 1
                    || !identifier.starts_with('0'))
        })
    };
    if version.len() > 256
        || parts.len() != 3
        || !parts.iter().all(valid_number)
        || prerelease.is_some_and(|value| !valid_identifiers(value, true))
        || build.is_some_and(|value| !valid_identifiers(value, false))
    {
        return Err(format!(
            "Invalid tsrs package_version {version:?}; specify an exact published version such as {DEFAULT_PACKAGE_VERSION}, not a tag, range, URL, or path."
        ));
    }
    Ok(())
}
