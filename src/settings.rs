use zed_extension_api::{Result, serde_json::Value};

use crate::installer::DEFAULT_PACKAGE_VERSION;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BinarySource {
    Local(String),
    Managed,
}

pub(crate) fn binary_source(
    explicit: Option<String>,
    which: impl FnOnce() -> Option<String>,
) -> Result<BinarySource> {
    if let Some(path) = explicit {
        if path.trim().is_empty() {
            return Err("lsp.tsrs.binary.path must not be empty; set an absolute native executable path or remove the override to use PATH/managed installation".into());
        }

        return Ok(BinarySource::Local(path));
    }
    Ok(which().map_or(BinarySource::Managed, BinarySource::Local))
}

pub(crate) fn package_version(settings: Option<&Value>) -> Result<&str> {
    match settings.and_then(|settings| settings.get("package_version")) {
        None => Ok(DEFAULT_PACKAGE_VERSION),
        Some(Value::String(version)) if !version.trim().is_empty() => Ok(version),
        Some(_) => Err(
            "lsp.tsrs.settings.package_version must be a nonempty exact npm version string".into(),
        ),
    }
}

pub(crate) fn workspace_configuration(settings: Option<Value>) -> Option<Value> {
    settings.map(|mut settings| {
        if let Value::Object(object) = &mut settings {
            object.remove("package_version");
        }
        settings
    })
}
