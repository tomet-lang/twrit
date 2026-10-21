use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use cargo_metadata::{Metadata, MetadataCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flavour {
    NoDeps,
    Deps,
}

type MetadataResult = Result<Arc<Metadata>, String>;
type CacheMap = HashMap<(PathBuf, Flavour), MetadataResult>;

static CACHE: Mutex<Option<CacheMap>> = Mutex::new(None);
static EXEC_COUNTS: Mutex<Option<HashMap<PathBuf, usize>>> = Mutex::new(None);

/// Returns the number of times `MetadataCommand::exec()` was executed for a specific manifest.
#[doc(hidden)]
pub fn exec_count_for(manifest_path: &Path) -> usize {
    let lock = EXEC_COUNTS.lock().unwrap();
    lock.as_ref()
        .and_then(|m| m.get(manifest_path).copied())
        .unwrap_or(0)
}

/// Fetches cargo metadata for the given manifest path and flavour, using the
/// cache if already queried for this manifest and flavour.
///
/// An error during `cargo metadata` execution is also cached and returned on
/// subsequent calls, ensuring errors are reported consistently.
pub fn get(manifest_path: &Path, flavour: Flavour) -> Result<Arc<Metadata>> {
    let key = (manifest_path.to_path_buf(), flavour);

    // 1. Fast path: check cache under lock
    {
        let mut lock = CACHE.lock().unwrap();
        let cache = lock.get_or_insert_with(HashMap::new);
        if let Some(entry) = cache.get(&key) {
            return match entry {
                Ok(meta) => Ok(Arc::clone(meta)),
                Err(err_msg) => Err(anyhow::anyhow!("{err_msg}")),
            };
        }
        // `Deps` metadata is a strict superset of `NoDeps` (contains all packages and workspace members).
        if flavour == Flavour::NoDeps {
            let deps_key = (manifest_path.to_path_buf(), Flavour::Deps);
            if let Some(entry) = cache.get(&deps_key) {
                return match entry {
                    Ok(meta) => Ok(Arc::clone(meta)),
                    Err(err_msg) => Err(anyhow::anyhow!("{err_msg}")),
                };
            }
        }
    }

    // 2. Track execution count for tests
    {
        let mut counts = EXEC_COUNTS.lock().unwrap();
        *counts
            .get_or_insert_with(HashMap::new)
            .entry(manifest_path.to_path_buf())
            .or_insert(0) += 1;
    }

    // 3. Execute cargo metadata without holding the CACHE lock
    let mut cmd = MetadataCommand::new();
    cmd.manifest_path(manifest_path);
    if flavour == Flavour::NoDeps {
        cmd.no_deps();
    }

    let result = match cmd.exec() {
        Ok(meta) => Ok(Arc::new(meta)),
        Err(e) => Err(format!("{e}")),
    };

    // 4. Store result in cache under lock
    let mut lock = CACHE.lock().unwrap();
    let cache = lock.get_or_insert_with(HashMap::new);
    match &result {
        Ok(meta) => {
            cache.insert(key, Ok(Arc::clone(meta)));
            Ok(Arc::clone(meta))
        }
        Err(msg) => {
            cache.insert(key, Err(msg.clone()));
            Err(anyhow::anyhow!("{msg}"))
        }
    }
}
