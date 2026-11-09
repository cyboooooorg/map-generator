//! The resolutions of a generated body, each in a directory of its own:
//!
//! ```text
//! bodies/<body>/
//! ├── thumb.png            from the original resolution
//! └── sizes/<W>x<H>/       one resolution: world.png,
//!                          legend.png, raw_data/; world.svg and
//!                          noise_maps/ only on request
//! ```
//!
//! The original resolution (the size in `system.json`) is one of them, laid
//! out like the others. A resolution is written into `sizes/.tmp-<W>x<H>`
//! and renamed into place once complete, so a name that parses always
//! holds a whole resolution.
use std::path::{Path, PathBuf};

/// The directory of a body's resolutions, in the body's directory.
pub const SIZES_DIR: &str = "sizes";

/// The directory name of a resolution: `2048x1024`.
pub fn size_name(width: u32, height: u32) -> String {
    format!("{width}x{height}")
}

/// The size a resolution's directory name says, if it is one written by
/// [`size_name`]: two positive decimal numbers without leading zeros
/// joined by a lowercase `x`, nothing else.
pub fn parse_size_name(name: &str) -> Option<(u32, u32)> {
    let (width, height) = name.split_once('x')?;
    let number = |text: &str| -> Option<u32> {
        if text.is_empty() || text.starts_with('0') || !text.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        text.parse().ok()
    };
    Some((number(width)?, number(height)?))
}

/// The directory of the `width` × `height` resolution of the body in
/// `body_dir`: `body_dir/sizes/<W>x<H>`.
pub fn size_dir(body_dir: &Path, width: u32, height: u32) -> PathBuf {
    body_dir.join(SIZES_DIR).join(size_name(width, height))
}

/// The resolutions of the body in `body_dir` on disk, smallest first (by
/// pixel count): the directories of `sizes/` whose name parses and that
/// hold a `raw_data/world.json`. Unfinished ones (`.tmp-*`) never parse.
pub fn sizes_on_disk(body_dir: &Path) -> Vec<(u32, u32)> {
    let Ok(read) = std::fs::read_dir(body_dir.join(SIZES_DIR)) else {
        return Vec::new();
    };
    let mut sizes: Vec<(u32, u32)> = read
        .flatten()
        .filter_map(|entry| {
            let size = parse_size_name(&entry.file_name().to_string_lossy())?;
            entry
                .path()
                .join("raw_data")
                .join("world.json")
                .is_file()
                .then_some(size)
        })
        .collect();
    sizes.sort_by_key(|&(w, h)| (u64::from(w) * u64::from(h), w));
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for (w, h) in [(2, 1), (256, 128), (2048, 1024), (16384, 8192)] {
            assert_eq!(parse_size_name(&size_name(w, h)), Some((w, h)));
        }
        assert_eq!(size_name(2048, 1024), "2048x1024");
    }

    #[test]
    fn names_not_written_by_size_name_are_refused() {
        for bad in [
            "",
            "x",
            "2048",
            "2048x",
            "x1024",
            "0x1024",
            "2048x0",
            "02048x1024",
            "2048x01024",
            "2048X1024",
            "2048x1024x2",
            "2048x10a4",
            "+2048x1024",
            "-2x1",
            ".tmp-2048x1024",
            "99999999999x1",
        ] {
            assert_eq!(parse_size_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn size_dir_is_under_sizes() {
        assert_eq!(
            size_dir(Path::new("/b/c"), 512, 256),
            Path::new("/b/c/sizes/512x256")
        );
    }

    #[test]
    fn only_complete_sizes_are_on_disk() {
        let body =
            std::env::temp_dir().join(format!("mapgen-sizes-on-disk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&body);
        assert!(sizes_on_disk(&body).is_empty());
        for (name, complete) in [
            ("2048x1024", true),
            ("256x128", true),
            ("1024x512", false),
            (".tmp-4096x2048", true),
            ("junk", true),
        ] {
            let raw = body.join(SIZES_DIR).join(name).join("raw_data");
            std::fs::create_dir_all(&raw).unwrap();
            if complete {
                std::fs::write(raw.join("world.json"), b"{}").unwrap();
            }
        }
        assert_eq!(sizes_on_disk(&body), vec![(256, 128), (2048, 1024)]);
        let _ = std::fs::remove_dir_all(body);
    }
}
