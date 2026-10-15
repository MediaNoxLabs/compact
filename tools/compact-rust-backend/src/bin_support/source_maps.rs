//! Rebase the compiler's ordinary JS map across staging and final publication.
//! Uses lexical directories; mappings and every non-path JSON value stay intact.
use serde_json::Value;
use std::{
    error::Error,
    fs, io,
    path::{Component, Path, PathBuf},
};

pub(crate) fn rebase(
    staging: &Path,
    original_frontend: &Path,
    final_output: &Path,
) -> Result<bool, Box<dyn Error>> {
    let path = staging.join("contract/index.js.map");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut map: Value = serde_json::from_slice(&bytes)?;
    let cwd = std::env::current_dir()?;
    let original = absolute(&original_frontend.join("contract"), &cwd);
    let published = absolute(&final_output.join("contract"), &cwd);
    if !rebase_value(&mut map, &original, &published)? {
        return Ok(false);
    }
    // Serialization may normalize formatting/key order. All non-path values,
    // including mappings, sourcesContent, names and extensions, are preserved.
    fs::write(path, serde_json::to_vec_pretty(&map)?)?;
    Ok(true)
}

fn rebase_value(map: &mut Value, original: &Path, published: &Path) -> Result<bool, io::Error> {
    let fields = map
        .as_object_mut()
        .ok_or_else(|| invalid("source map must be a JSON object"))?;
    if let Some(root) = fields.get_mut("sourceRoot") {
        let old = root
            .as_str()
            .ok_or_else(|| invalid("sourceRoot must be a string"))?;
        if external(old) {
            return Ok(false);
        }
        let destination = normalize(&original.join(old));
        let mut new = path_text(&relative(published, &destination))?;
        if old.ends_with('/') && !new.ends_with('/') {
            new.push('/');
        }
        if new == old {
            return Ok(false);
        }
        *root = Value::String(new);
        return Ok(true);
    }
    let Some(sources) = fields.get_mut("sources") else {
        return Ok(false);
    };
    let sources = sources
        .as_array_mut()
        .ok_or_else(|| invalid("sources must be an array"))?;
    let mut changed = false;
    for source in sources {
        let old = source
            .as_str()
            .ok_or_else(|| invalid("source must be a string"))?;
        if external(old) {
            continue;
        }
        let new = path_text(&relative(published, &normalize(&original.join(old))))?;
        if new != old {
            *source = Value::String(new);
            changed = true;
        }
    }
    Ok(changed)
}

fn external(value: &str) -> bool {
    if Path::new(value).is_absolute() || value.starts_with('/') || value.starts_with('\\') {
        return true;
    }
    // URI schemes (including drive-qualified paths) are opaque to this helper.
    value.split_once(':').is_some_and(|(scheme, _)| {
        scheme
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
            && scheme
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"+-.".contains(&byte))
    })
}
fn absolute(path: &Path, cwd: &Path) -> PathBuf {
    normalize(&if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    })
}
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(result.components().next_back(), Some(Component::Normal(_))) {
                    result.pop();
                }
            }
            _ => result.push(component.as_os_str()),
        }
    }
    result
}
fn relative(from: &Path, destination: &Path) -> PathBuf {
    let from = normalize(from);
    let destination = normalize(destination);
    let base = from.components().collect::<Vec<_>>();
    let target = destination.components().collect::<Vec<_>>();
    let common = base
        .iter()
        .zip(&target)
        .take_while(|(left, right)| left == right)
        .count();
    // Different Windows volume prefixes have no relative route.
    if common == 0 {
        return destination;
    }
    let mut result = PathBuf::new();
    for _ in &base[common..] {
        result.push("..");
    }
    for component in &target[common..] {
        result.push(component.as_os_str());
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    result
}
fn path_text(path: &Path) -> Result<String, io::Error> {
    let text = path
        .to_str()
        .ok_or_else(|| invalid("source map path is not UTF-8"))?;
    #[cfg(windows)]
    let text = text.replace('\\', "/");
    Ok(text.to_owned())
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
#[path = "source_maps/tests.rs"]
mod tests;
