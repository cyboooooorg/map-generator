//! World thumbnails (`thumb.png`, see [`mapgen::export::thumbnail`]).
//!
//! The export always writes the thumbnail. This only repairs a deleted
//! `thumb.png`: the world then gets one on its first request, its `world.png` is decoded row by row into the
//! same downscaler the export uses, so a thumbnail costs a few MB of memory
//! even for an 8192 × 4096 map (measured: 4 MB and 0.4 s; decoding the
//! whole image would take 100 MB), then written beside the body's `sizes/`
//! for the next requests.
//!
//! Decoding still takes a core while it runs: at most
//! [`CONCURRENT_MAKES`] run at once, off the async workers, so a list of
//! old worlds does not decode them all together. With so little memory at
//! stake, they run during generations too, unlike the tile loads.
use image::{ImageFormat, RgbImage};
use mapgen::export::thumbnail::{Downscaler, THUMB_FILE};
use std::io::{BufReader, Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Thumbnails made from `world.png` at once.
pub const CONCURRENT_MAKES: usize = 2;
/// Name of the file a thumbnail is written into before it takes its place.
const TEMP_FILE: &str = ".thumb.png.tmp";

/// Why a thumbnail could not be served.
#[derive(Debug)]
pub enum ThumbError {
    /// The world has no `world.png`.
    NotFound,
    Failed(String),
}

pub struct Thumbnails {
    makes: Arc<Semaphore>,
}

impl Default for Thumbnails {
    fn default() -> Self {
        Self {
            makes: Arc::new(Semaphore::new(CONCURRENT_MAKES)),
        }
    }
}

impl Thumbnails {
    /// The PNG bytes of the thumbnail in `thumb_dir`, made from
    /// `world_png` first when there is none (a body keeps its thumbnail
    /// beside its resolutions, made from the original one's `world.png`).
    pub async fn get(&self, thumb_dir: &Path, world_png: &Path) -> Result<Vec<u8>, ThumbError> {
        let path = thumb_dir.join(THUMB_FILE);
        match tokio::fs::read(&path).await {
            Ok(bytes) => return Ok(bytes),
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(ThumbError::Failed(e.to_string())),
        }
        // The permit moves into the blocking task: a request dropped while
        // its thumbnail is being made does not free a turn early.
        let permit = self
            .makes
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| ThumbError::Failed(e.to_string()))?;
        let (dir, world_png) = (thumb_dir.to_path_buf(), world_png.to_path_buf());
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            // Made by a request that held the turn before this one.
            match std::fs::read(&path) {
                Ok(bytes) => Ok(bytes),
                Err(_) => make(&dir, &world_png),
            }
        })
        .await
        .map_err(|e| ThumbError::Failed(e.to_string()))?
    }
}

/// Makes a thumbnail from the world map `source` (a `world.png`), writes
/// it into `dir` (best effort: a read-only library still gets thumbnails)
/// and returns its PNG bytes.
fn make(dir: &Path, source: &Path) -> Result<Vec<u8>, ThumbError> {
    let file = std::fs::File::open(source).map_err(|e| match e.kind() {
        ErrorKind::NotFound => ThumbError::NotFound,
        _ => ThumbError::Failed(format!("{}: {e}", source.display())),
    })?;
    let failed = |e: String| ThumbError::Failed(format!("{}: {e}", source.display()));
    let thumb = downscale(BufReader::new(file)).map_err(failed)?;
    let mut bytes = Cursor::new(Vec::new());
    thumb
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|e| failed(e.to_string()))?;
    let bytes = bytes.into_inner();
    match install(dir, &bytes) {
        Ok(Some(existing)) => return Ok(existing),
        Ok(None) => tracing::info!("made the thumbnail of {}", dir.display()),
        Err(e) => tracing::warn!("could not write the thumbnail of {}: {e}", dir.display()),
    }
    Ok(bytes)
}

/// The thumbnail of a PNG image, decoded one row at a time.
fn downscale(source: impl std::io::BufRead + std::io::Seek) -> Result<RgbImage, String> {
    let mut decoder = png::Decoder::new(source);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    if info.interlaced {
        return Err("interlaced PNG images are not supported".into());
    }
    let channels = match reader.output_color_type().0 {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Indexed => return Err("unexpanded palette".into()),
    };
    let mut scaler = Downscaler::new(width, height);
    let mut rgb = Vec::with_capacity(width as usize * 3);
    for _ in 0..height {
        let row = reader
            .next_row()
            .map_err(|e| e.to_string())?
            .ok_or("the image ends early")?;
        let data = row.data();
        if channels == 3 {
            scaler.push_row(data);
        } else {
            // world.png is RGB; other formats are converted, alpha dropped.
            rgb.clear();
            rgb.extend(data.chunks_exact(channels).flat_map(|p| {
                if channels >= 3 {
                    [p[0], p[1], p[2]]
                } else {
                    [p[0]; 3]
                }
            }));
            scaler.push_row(&rgb);
        }
    }
    Ok(scaler.finish())
}

/// Writes `bytes` as the thumbnail of `dir`, atomically: into a temporary
/// file first, then linked into place unless a thumbnail appeared
/// meanwhile (the world was regenerated with its own), whose bytes are
/// then returned.
fn install(dir: &Path, bytes: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
    let temp: PathBuf = dir.join(TEMP_FILE);
    let target = dir.join(THUMB_FILE);
    std::fs::write(&temp, bytes)?;
    let linked = std::fs::hard_link(&temp, &target);
    let _ = std::fs::remove_file(&temp);
    match linked {
        Ok(()) => Ok(None),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => std::fs::read(&target).map(Some),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapgen::export::{ExportOptions, export_world};
    use mapgen::generation::generate;
    use mapgen::request::WorldRequest;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mapgen-thumbs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[tokio::test]
    async fn makes_the_thumbnail_of_an_old_world_like_the_export() {
        let dir = temp_dir("old");
        let request = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(3),
            width: Some(1200),
            height: Some(600),
            ..Default::default()
        };
        let params = request.resolve(&mut rand::rng()).unwrap().params;
        let options = ExportOptions {
            raw_data: false,
            thumb_dir: Some(dir.clone()),
        };
        export_world(generate(&params), &dir, options).unwrap();
        let exported = std::fs::read(dir.join(THUMB_FILE)).unwrap();
        // A deleted thumbnail.
        std::fs::remove_file(dir.join(THUMB_FILE)).unwrap();

        let thumbnails = Thumbnails::default();
        let made = thumbnails.get(&dir, &dir.join("world.png")).await.unwrap();
        assert_eq!(made, exported, "the same thumbnail as the export's");
        assert_eq!(std::fs::read(dir.join(THUMB_FILE)).unwrap(), exported);
        assert!(!dir.join(TEMP_FILE).exists());
        let image = image::load_from_memory(&made).unwrap();
        assert_eq!((image.width(), image.height()), (512, 256));

        // Served from the file from then on.
        std::fs::write(dir.join(THUMB_FILE), b"cached").unwrap();
        assert_eq!(
            thumbnails.get(&dir, &dir.join("world.png")).await.unwrap(),
            b"cached"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn reports_missing_and_broken_maps() {
        let dir = temp_dir("missing");
        std::fs::create_dir_all(&dir).unwrap();
        let thumbnails = Thumbnails::default();
        assert!(matches!(
            thumbnails.get(&dir, &dir.join("world.png")).await,
            Err(ThumbError::NotFound)
        ));
        std::fs::write(dir.join("world.png"), b"not a png").unwrap();
        assert!(matches!(
            thumbnails.get(&dir, &dir.join("world.png")).await,
            Err(ThumbError::Failed(_))
        ));
        assert!(!dir.join(THUMB_FILE).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn keeps_a_thumbnail_that_appeared_meanwhile() {
        let dir = temp_dir("race");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(install(&dir, b"first").unwrap(), None);
        assert_eq!(install(&dir, b"second").unwrap().unwrap(), b"first");
        assert_eq!(std::fs::read(dir.join(THUMB_FILE)).unwrap(), b"first");
        assert!(!dir.join(TEMP_FILE).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn converts_other_png_formats() {
        let image = image::RgbaImage::from_pixel(1024, 512, image::Rgba([10, 200, 30, 128]));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        bytes.set_position(0);
        let thumb = downscale(bytes).unwrap();
        assert_eq!(thumb.dimensions(), (512, 256));
        assert!(thumb.pixels().all(|p| p.0 == [10, 200, 30]));
    }
}
