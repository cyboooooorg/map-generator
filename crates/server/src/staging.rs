//! Writing a directory safely: into a hidden temporary directory, renamed
//! into place once complete, and swept at startup after a crash. Worlds,
//! systems and the bodies of a system are written this way.
use std::path::{Path, PathBuf};

/// Prefix of the temporary directories generations export into. A dot is
/// never part of a world id, so the library never lists them.
const TEMP_PREFIX: &str = ".tmp-";
/// Prefix a replaced world is renamed to while its replacement moves in
/// (see [`TempDir::commit_replacing`]). Never the prefix of a temporary
/// directory, whatever the world id.
const OLD_PREFIX: &str = ".old-";

/// The directory a world is exported into before it is complete, deleted on
/// drop unless [`commit`](Self::commit) renamed it into place: an export
/// that fails, panics or is cancelled leaves nothing behind.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates `<dir>/.tmp-<id>`, emptied first if a previous attempt left it.
    pub fn create(dir: &Path, id: &str) -> Result<Self, String> {
        let path = dir.join(format!("{TEMP_PREFIX}{id}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Renames the directory to `target`, which must not exist.
    pub fn commit(self, target: &Path) -> Result<(), String> {
        std::fs::rename(&self.path, target).map_err(|e| format!("{}: {e}", target.display()))
        // On failure, drop removes the temporary directory.
    }

    /// Renames the directory to `target`, replacing the world there if
    /// any: the old world is moved aside to `.old-<id>`, the new one
    /// renamed into place, then the old one removed. If the new one cannot
    /// be moved in, the old one is moved back, so a failed commit keeps the
    /// old world. A server killed between the two renames finds the old
    /// world aside at startup and restores it ([`sweep_temp_dirs`]).
    pub fn commit_replacing(self, target: &Path) -> Result<(), String> {
        let name = target
            .file_name()
            .ok_or_else(|| format!("{}: no directory name", target.display()))?
            .to_string_lossy();
        let aside = target.with_file_name(format!("{OLD_PREFIX}{name}"));
        let _ = std::fs::remove_dir_all(&aside);
        let replaced = match std::fs::rename(target, &aside) {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(format!("{}: {e}", target.display())),
        };
        if let Err(e) = std::fs::rename(&self.path, target) {
            if replaced && let Err(back) = std::fs::rename(&aside, target) {
                tracing::error!("could not restore {}: {back}", target.display());
            }
            return Err(format!("{}: {e}", target.display()));
        }
        if replaced && let Err(e) = std::fs::remove_dir_all(&aside) {
            tracing::warn!("could not remove {}: {e}", aside.display());
        }
        Ok(())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Already renamed after a successful commit: nothing to remove.
        if self.path.exists()
            && let Err(e) = std::fs::remove_dir_all(&self.path)
        {
            tracing::warn!("could not remove {}: {e}", self.path.display());
        }
    }
}

/// Removes the temporary directories a killed server left under `dir`, and
/// puts back a replaced directory it had moved aside but not replaced yet
/// (its name valid by `is_valid`; anything else aside is removed).
pub fn sweep(dir: &Path, is_valid: impl Fn(&str) -> bool) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(id) = name.strip_prefix(OLD_PREFIX) {
            let path = entry.path();
            let target = dir.join(id);
            if is_valid(id) && !target.exists() {
                match std::fs::rename(&path, &target) {
                    Ok(()) => tracing::info!(
                        "restored {} after an unfinished replacement",
                        target.display()
                    ),
                    Err(e) => tracing::warn!("could not restore {}: {e}", target.display()),
                }
            } else if let Err(e) = std::fs::remove_dir_all(&path) {
                tracing::warn!("could not remove {}: {e}", path.display());
            }
            continue;
        }
        if name.starts_with(TEMP_PREFIX) {
            let path = entry.path();
            match std::fs::remove_dir_all(&path) {
                Ok(()) => tracing::info!("removed the unfinished export {}", path.display()),
                Err(e) => tracing::warn!("could not remove {}: {e}", path.display()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_dirs_are_renamed_or_removed() {
        let root = std::env::temp_dir().join(format!("mapgen-tempdir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        // Dropped before its commit (error, cancel, panic): nothing is left.
        let temp = TempDir::create(&root, "terran-1").unwrap();
        std::fs::write(temp.path().join("world.png"), b"partial").unwrap();
        drop(temp);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);

        // Committed: the complete directory appears under its final name.
        let temp = TempDir::create(&root, "terran-2").unwrap();
        std::fs::write(temp.path().join("world.png"), b"done").unwrap();
        temp.commit(&root.join("terran-2")).unwrap();
        assert!(root.join("terran-2/world.png").exists());
        assert!(!root.join(".tmp-terran-2").exists());

        // Hidden from the library, removed by the sweep.
        std::fs::create_dir_all(root.join(".tmp-terran-3/raw_data")).unwrap();
        sweep(&root, crate::systems::is_valid_system_id);
        assert!(!root.join(".tmp-terran-3").exists());
        assert!(root.join("terran-2").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replacing_commits_swap_the_world_or_keep_the_old_one() {
        let root = std::env::temp_dir().join(format!("mapgen-replace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("terran-1")).unwrap();
        std::fs::write(root.join("terran-1/world.png"), b"old").unwrap();
        let target = root.join("terran-1");

        // A replacement swaps the directories and removes the old one.
        let temp = TempDir::create(&root, "terran-1").unwrap();
        std::fs::write(temp.path().join("world.png"), b"new").unwrap();
        temp.commit_replacing(&target).unwrap();
        assert_eq!(std::fs::read(target.join("world.png")).unwrap(), b"new");
        assert!(!root.join(".old-terran-1").exists());
        assert!(!root.join(".tmp-terran-1").exists());

        // Nothing to replace: a plain commit.
        let temp = TempDir::create(&root, "terran-2").unwrap();
        temp.commit_replacing(&root.join("terran-2")).unwrap();
        assert!(root.join("terran-2").is_dir());

        // The new directory cannot move in (gone here): the old world stays.
        let temp = TempDir::create(&root, "terran-1").unwrap();
        std::fs::remove_dir_all(temp.path()).unwrap();
        assert!(temp.commit_replacing(&target).is_err());
        assert_eq!(std::fs::read(target.join("world.png")).unwrap(), b"new");
        assert!(!root.join(".old-terran-1").exists());

        // Killed between the renames: the sweep restores the old world, or
        // removes it when its replacement made it into place.
        std::fs::rename(&target, root.join(".old-terran-1")).unwrap();
        std::fs::create_dir_all(root.join(".old-terran-2")).unwrap();
        sweep(&root, crate::systems::is_valid_system_id);
        assert_eq!(std::fs::read(target.join("world.png")).unwrap(), b"new");
        assert!(!root.join(".old-terran-1").exists());
        assert!(!root.join(".old-terran-2").exists());
        assert!(root.join("terran-2").is_dir());
        std::fs::remove_dir_all(root).unwrap();
    }
}
