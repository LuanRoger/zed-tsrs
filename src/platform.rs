use zed_extension_api::{self as zed, Architecture, Os, Result};

pub(crate) struct NativePlatform {
    pub package: String,
    pub executable: &'static str,
}

pub(crate) fn current() -> Result<NativePlatform> {
    let (os, arch) = zed::current_platform();
    // Reject unsupported CPUs before invoking Node or doing any installation.
    if matches!(arch, Architecture::X86) {
        return Err("tsrs has no 32-bit x86 npm binary; use an x86-64 or ARM64 system, or configure a custom binary.".into());
    }
    let musl = matches!(os, Os::Linux) && linux_is_musl()?;
    platform(os, arch, musl)
}

fn platform(os: Os, arch: Architecture, musl: bool) -> Result<NativePlatform> {
    let cpu = match arch {
        Architecture::X8664 => "x64",
        Architecture::Aarch64 => "arm64",
        Architecture::X86 => return Err("tsrs does not publish 32-bit x86 native packages".into()),
    };
    let (os, suffix, executable) = match os {
        Os::Mac => ("darwin", "", "tsrs"),
        Os::Windows => ("win32", "", "tsrs.exe"),
        Os::Linux => ("linux", if musl { "-musl" } else { "-gnu" }, "tsrs"),
    };
    Ok(NativePlatform {
        package: format!("@tsrs/{os}-{cpu}{suffix}"),
        executable,
    })
}

fn linux_is_musl() -> Result<bool> {
    // WASI's filesystem and compile-time target describe the extension sandbox,
    // not the host libc. Probe through Zed's host Node, like npm/tsrs/lib/platform.js.
    const PROBE: &str = r#"
const fs = require('node:fs');
const cp = require('node:child_process');
function isMusl() {
    try { return fs.readFileSync('/usr/bin/ldd', 'utf8').includes('musl'); } catch {}
    if (typeof process.report?.getReport === 'function') {
        const previous = process.report.excludeNetwork;
        try {
            process.report.excludeNetwork = true;
            const report = process.report.getReport();
            if (report?.header?.glibcVersionRuntime) return false;
            if (report?.sharedObjects?.some(p => p.includes('libc.musl-') || p.includes('ld-musl-'))) return true;
        } catch {} finally { process.report.excludeNetwork = previous; }
    }
    try { return cp.execFileSync('ldd', ['--version'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).includes('musl'); }
    catch (e) { return `${e.stdout ?? ''}${e.stderr ?? ''}`.includes('musl'); }
}
process.stdout.write(isMusl() ? 'musl' : 'gnu');
"#;
    let node = zed::node_binary_path().map_err(|e| {
        format!("Cannot detect tsrs Linux libc: Zed's Node runtime is unavailable: {e}")
    })?;
    let output = zed::process::Command::new(node)
        .args(["-e", PROBE])
        .output()
        .map_err(|e| format!("Cannot run tsrs Linux libc detection: {e}"))?;
    match (output.status, output.stdout.as_slice()) {
        (Some(0), b"musl") => Ok(true),
        (Some(0), b"gnu") => Ok(false),
        _ => Err(format!(
            "Cannot detect tsrs Linux libc (exit {:?}): {}. Configure a custom native binary if Node execution is restricted.",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )),
    }
}
