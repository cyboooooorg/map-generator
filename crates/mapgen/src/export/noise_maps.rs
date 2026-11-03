//! Export of the diagnostic maps: one false-colour PNG per intermediate
//! generation layer (see the README for the list).
//!
//! All maps share the same "jet" ramp: blue (low) → cyan → green → yellow →
//! red (high). Signed layers are rescaled so that -1.0 → blue, 0.0 → green and
//! +1.0 → red; unsigned layers so that 0.0 → blue and 1.0 → red.
//!
//! The maps are written while the world is generated: [`NoiseMapWriter`] is
//! the [`DebugSink`] that colours and encodes each layer as it arrives, so
//! no layer is kept once written. Layers delivered in bands (the noise
//! layers, sampled band by band) are encoded row by row as their bands
//! come; whole layers are handed to a background thread, so encoding does
//! not hold up the generation.
use crate::generation::{DebugLayer, DebugSink, Ramp};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

// ── Colour map ────────────────────────────────────────────────────────────────

/// "Jet" ramp: blue → cyan → green → yellow → red.
/// `t` ∈ [0.0, 1.0].
fn jet(t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    // Piecewise linear hat functions shifted to R, G, B channels.
    let r = (1.5 - (4.0 * t - 3.0).abs()).clamp(0.0, 1.0);
    let g = (1.5 - (4.0 * t - 2.0).abs()).clamp(0.0, 1.0);
    let b = (1.5 - (4.0 * t - 1.0).abs()).clamp(0.0, 1.0);
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

fn colorize(ramp: Ramp, v: f32) -> [u8; 3] {
    match ramp {
        // [-1, 1] → [0, 1]: 0.0 is green.
        Ramp::Signed => jet((v.clamp(-1.0, 1.0) + 1.0) * 0.5),
        Ramp::Unsigned => jet(v),
    }
}

// ── Row-by-row PNG encoder ────────────────────────────────────────────────────

/// An 8-bit RGB PNG written row by row, straight to its file.
///
/// The bytes are those the `image` crate writes for the same pixels (its
/// default PNG settings: `fdeflate`'s fast compression, adaptive filter,
/// one `IDAT` chunk), so the maps do not change with the way they are
/// written; a test checks it. The `IDAT` chunk goes to the file as it is
/// compressed: its length is patched in at the end, and its CRC computed on
/// the way.
struct RgbPng {
    deflate: fdeflate::Compressor<Idat>,
    /// Bytes per row (3 per pixel).
    stride: usize,
    height: u32,
    rows_left: u32,
    previous: Vec<u8>,
    current: Vec<u8>,
    /// The filter type then the filtered row: compressed in one piece, as
    /// the `png` crate does (the compressor's output depends on it).
    filtered: Vec<u8>,
}

/// Offset of the `IDAT` length in the file: the signature (8 bytes) and the
/// `IHDR` chunk (25 bytes) come first.
const IDAT_OFFSET: u64 = 8 + 25;

/// The data of the `IDAT` chunk, written to the file as it comes.
struct Idat {
    file: BufWriter<File>,
    crc: crc32fast::Hasher,
    length: u64,
}

impl Write for Idat {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let n = self.file.write(data)?;
        self.crc.update(&data[..n]);
        self.length += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// Writes one PNG chunk.
fn write_chunk(w: &mut impl Write, kind: &[u8; 4], data: &[u8]) -> std::io::Result<()> {
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(data);
    w.write_all(&(data.len() as u32).to_be_bytes())?;
    w.write_all(kind)?;
    w.write_all(data)?;
    w.write_all(&crc.finalize().to_be_bytes())
}

impl RgbPng {
    fn create(path: &Path, width: u32, height: u32) -> std::io::Result<Self> {
        // Readable too, for the rare rewrite of `store_uncompressed`.
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        let mut file = BufWriter::new(file);
        file.write_all(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'])?;
        let mut header = [0u8; 13];
        header[..4].copy_from_slice(&width.to_be_bytes());
        header[4..8].copy_from_slice(&height.to_be_bytes());
        // 8 bits per channel, RGB, deflate, adaptive filtering, no interlace.
        header[8..].copy_from_slice(&[8, 2, 0, 0, 0]);
        write_chunk(&mut file, b"IHDR", &header)?;
        // Length patched in by `finish`.
        file.write_all(&[0; 4])?;
        file.write_all(b"IDAT")?;
        let mut crc = crc32fast::Hasher::new();
        crc.update(b"IDAT");
        let stride = width as usize * 3;
        Ok(Self {
            deflate: fdeflate::Compressor::new(Idat {
                file,
                crc,
                length: 0,
            })?,
            stride,
            height,
            rows_left: height,
            previous: vec![0; stride],
            current: vec![0; stride],
            filtered: vec![0; stride + 1],
        })
    }

    /// Appends the next row, `stride` bytes of RGB.
    fn write_row(&mut self, row: impl FnOnce(&mut [u8])) -> std::io::Result<()> {
        if self.rows_left == 0 {
            return Err(std::io::Error::other("more rows than the image height"));
        }
        self.rows_left -= 1;
        row(&mut self.current);
        let (kind, filtered) = self.filtered.split_at_mut(1);
        kind[0] = adaptive_filter(&self.previous, &self.current, filtered);
        self.deflate.write_data(&self.filtered)?;
        std::mem::swap(&mut self.previous, &mut self.current);
        Ok(())
    }

    fn finish(self) -> std::io::Result<()> {
        if self.rows_left > 0 {
            return Err(std::io::Error::other("fewer rows than the image height"));
        }
        let Idat {
            mut file,
            crc,
            length,
        } = self.deflate.finish()?;
        let raw = (self.stride + 1) * self.height as usize;
        if length > fdeflate::StoredOnlyCompressor::<()>::compressed_size(raw) as u64 {
            return store_uncompressed(file, length, self.stride);
        }
        let length = u32::try_from(length)
            .ok()
            .filter(|&l| l <= u32::MAX >> 1)
            .ok_or_else(|| std::io::Error::other("image data too large for one chunk"))?;
        file.write_all(&crc.finalize().to_be_bytes())?;
        write_chunk(&mut file, b"IEND", &[])?;
        file.seek(SeekFrom::Start(IDAT_OFFSET))?;
        file.write_all(&length.to_be_bytes())?;
        file.flush()
    }
}

/// Rewrites the image data of a PNG being written as uncompressed stored
/// blocks of unfiltered rows, as the `png` crate does when its fast
/// compression would take more room than that (tiny or noisy images). The
/// compressed data, `length` bytes after the `IDAT` header, is read back
/// and decoded: this rare path holds the image in memory.
fn store_uncompressed(
    mut file: BufWriter<File>,
    length: u64,
    stride: usize,
) -> std::io::Result<()> {
    use std::io::{Cursor, Read};
    file.flush()?;
    let mut file = file.into_inner().map_err(|e| e.into_error())?;
    file.seek(SeekFrom::Start(IDAT_OFFSET + 8))?;
    let mut compressed = vec![0; length as usize];
    file.read_exact(&mut compressed)?;
    let mut data = fdeflate::decompress_to_vec(&compressed)
        .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
    drop(compressed);
    let mut stored = fdeflate::StoredOnlyCompressor::new(Cursor::new(Vec::new()))?;
    let mut previous = vec![0; stride];
    for line in data.chunks_exact_mut(stride + 1) {
        let (kind, row) = line.split_at_mut(1);
        unfilter_row(kind[0], &previous, row);
        stored.write_data(&[0])?;
        stored.write_data(row)?;
        previous.copy_from_slice(row);
    }
    drop(data);
    let stored = stored.finish()?.into_inner();
    file.set_len(IDAT_OFFSET)?;
    file.seek(SeekFrom::Start(IDAT_OFFSET))?;
    let mut file = BufWriter::new(file);
    write_chunk(&mut file, b"IDAT", &stored)?;
    write_chunk(&mut file, b"IEND", &[])?;
    file.flush()
}

/// Reverses [`filter_row`] in place.
fn unfilter_row(kind: u8, previous: &[u8], row: &mut [u8]) {
    for i in 0..row.len() {
        let (a, c) = if i >= BPP {
            (row[i - BPP], previous[i - BPP])
        } else {
            (0, 0)
        };
        let b = previous[i];
        let prediction = match kind {
            SUB => a,
            UP => b,
            AVERAGE => ((a as u16 + b as u16) / 2) as u8,
            PAETH => paeth(a, b, c),
            _ => 0,
        };
        row[i] = row[i].wrapping_add(prediction);
    }
}

/// Filters `current` against `previous` (the row above, zeros for the first
/// one) into `out` and returns the filter type: the one of Up, Sub, Average
/// and Paeth whose output has the smallest sum of absolute values (as
/// signed bytes), the last one on ties, as the `png` crate's adaptive filter
/// chooses.
fn adaptive_filter(previous: &[u8], current: &[u8], out: &mut [u8]) -> u8 {
    let mut best = (u64::MAX, UP);
    for kind in [UP, SUB, AVERAGE, PAETH] {
        filter_row(kind, previous, current, out);
        let cost: u64 = out.iter().map(|&v| (v as i8).unsigned_abs() as u64).sum();
        if cost <= best.0 {
            best = (cost, kind);
            if cost == 0 {
                return kind;
            }
        }
    }
    if best.1 != PAETH {
        filter_row(best.1, previous, current, out);
    }
    best.1
}

/// PNG filter types.
const SUB: u8 = 1;
const UP: u8 = 2;
const AVERAGE: u8 = 3;
const PAETH: u8 = 4;
/// Bytes per pixel: the filters predict each byte from the same channel of
/// the pixel on the left.
const BPP: usize = 3;

/// Applies one PNG filter to a row of whole RGB pixels.
fn filter_row(kind: u8, previous: &[u8], current: &[u8], out: &mut [u8]) {
    // The first pixel has nothing on its left (0).
    let (head, rest) = out.split_at_mut(BPP);
    let left = current.iter();
    let up_left = previous.iter();
    let tail = current[BPP..].iter().zip(&previous[BPP..]);
    match kind {
        SUB => {
            head.copy_from_slice(&current[..BPP]);
            for (o, ((&c, _), &a)) in rest.iter_mut().zip(tail.zip(left)) {
                *o = c.wrapping_sub(a);
            }
        }
        UP => {
            for ((o, &c), &b) in out.iter_mut().zip(current).zip(previous) {
                *o = c.wrapping_sub(b);
            }
        }
        AVERAGE => {
            for ((o, &c), &b) in head.iter_mut().zip(current).zip(previous) {
                *o = c.wrapping_sub(b / 2);
            }
            for (o, ((&c, &b), &a)) in rest.iter_mut().zip(tail.zip(left)) {
                *o = c.wrapping_sub(((a as u16 + b as u16) / 2) as u8);
            }
        }
        _ => {
            for ((o, &c), &b) in head.iter_mut().zip(current).zip(previous) {
                *o = c.wrapping_sub(b);
            }
            for (o, (((&c, &b), &a), &ul)) in rest.iter_mut().zip(tail.zip(left).zip(up_left)) {
                *o = c.wrapping_sub(paeth(a, b, ul));
            }
        }
    }
}

/// The Paeth predictor of the PNG specification.
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i16 + b as i16 - c as i16;
    let (pa, pb, pc) = (
        (p - a as i16).abs(),
        (p - b as i16).abs(),
        (p - c as i16).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

// ── Diagnostic maps ───────────────────────────────────────────────────────────

/// A diagnostic map being written, row by row.
struct MapFile {
    png: RgbPng,
    ramp: Ramp,
}

impl MapFile {
    fn create(dir: &Path, name: &str, ramp: Ramp, size: (u32, u32)) -> std::io::Result<Self> {
        let png = RgbPng::create(&dir.join(format!("{name}.png")), size.0, size.1)?;
        Ok(Self { png, ramp })
    }

    /// Colours and appends whole rows of values.
    fn write(&mut self, values: &[f32]) -> std::io::Result<()> {
        let width = self.png.stride / 3;
        let ramp = self.ramp;
        for row in values.chunks(width) {
            self.png.write_row(|rgb| {
                for (px, &v) in rgb.as_chunks_mut::<3>().0.iter_mut().zip(row) {
                    *px = colorize(ramp, v);
                }
            })?;
        }
        Ok(())
    }
}

/// Writes one map from a whole layer.
fn write_map(dir: &Path, size: (u32, u32), layer: &DebugLayer) -> std::io::Result<()> {
    let mut map = MapFile::create(dir, layer.name, layer.ramp, size)?;
    map.write(&layer.values)?;
    map.png.finish()
}

/// Whole layers waiting for the background encoder: one queued while it
/// encodes another, so at most two are held besides the one being handed
/// over, while the generation goes on.
const QUEUED_LAYERS: usize = 1;

/// The [`DebugSink`] that writes each diagnostic layer as a false-colour
/// PNG into a directory, `<name>.png`, as soon as generation delivers it.
///
/// Create it before generating, pass it in
/// [`GenerateOptions::debug`](crate::generation::GenerateOptions), then
/// call [`NoiseMapWriter::finish`] to wait for the last maps and get the
/// first error, if any.
pub struct NoiseMapWriter {
    dir: PathBuf,
    size: (u32, u32),
    /// Maps delivered in bands, still being written.
    open: Mutex<HashMap<&'static str, Arc<Mutex<MapFile>>>>,
    queue: Option<SyncSender<DebugLayer>>,
    encoder: Option<JoinHandle<std::io::Result<()>>>,
    /// The first error met in the calling threads.
    error: Mutex<Option<std::io::Error>>,
}

impl NoiseMapWriter {
    /// A writer of `width` × `height` maps into `dir`, created if needed.
    pub fn new(dir: &Path, width: u32, height: u32) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let size = (width, height);
        let (queue, layers) = sync_channel::<DebugLayer>(QUEUED_LAYERS);
        let background_dir = dir.to_path_buf();
        let encoder = std::thread::Builder::new()
            .name("noise-maps".into())
            .spawn(move || {
                let mut result = Ok(());
                // Keeps receiving after an error, so the generation never
                // blocks on a full queue.
                for layer in layers {
                    if result.is_ok() {
                        result = write_map(&background_dir, size, &layer);
                    }
                }
                result
            })?;
        Ok(Self {
            dir: dir.to_path_buf(),
            size,
            open: Mutex::new(HashMap::new()),
            queue: Some(queue),
            encoder: Some(encoder),
            error: Mutex::new(None),
        })
    }

    /// Waits for every map to be written; the first error, if any.
    pub fn finish(mut self) -> Result<(), String> {
        drop(self.queue.take());
        let background = match self.encoder.take().map(JoinHandle::join) {
            Some(Ok(result)) => result.map_err(|e| e.to_string()),
            Some(Err(_)) => Err("the noise map encoder crashed".into()),
            None => Ok(()),
        };
        if let Some(e) = self.error.lock().unwrap().take() {
            return Err(e.to_string());
        }
        if let Some(name) = self.open.lock().unwrap().keys().next() {
            return Err(format!("noise map {name} is incomplete"));
        }
        background
    }

    fn record(&self, result: std::io::Result<()>) {
        if let Err(e) = result {
            self.error.lock().unwrap().get_or_insert(e);
        }
    }

    /// Appends a band of rows to the map `layer.name`, opened on its first
    /// band and finished on its last.
    fn write_band(&self, layer: &DebugLayer) -> std::io::Result<()> {
        let map = {
            let mut open = self.open.lock().unwrap();
            match open.get(layer.name) {
                Some(map) => map.clone(),
                None => {
                    let map = MapFile::create(&self.dir, layer.name, layer.ramp, self.size)?;
                    let map = Arc::new(Mutex::new(map));
                    open.insert(layer.name, map.clone());
                    map
                }
            }
        };
        let done = {
            let mut file = map.lock().unwrap();
            file.write(&layer.values)?;
            file.png.rows_left == 0
        };
        if !done {
            return Ok(());
        }
        drop(map);
        let map = self.open.lock().unwrap().remove(layer.name);
        match map.and_then(Arc::into_inner) {
            Some(map) => map.into_inner().unwrap().png.finish(),
            None => Err(std::io::Error::other(format!(
                "noise map {} written from two threads",
                layer.name
            ))),
        }
    }
}

impl DebugSink for NoiseMapWriter {
    fn layer(&self, layer: DebugLayer) {
        let whole = layer.values.len() == self.size.0 as usize * self.size.1 as usize;
        let started = self.open.lock().unwrap().contains_key(layer.name);
        if whole && !started {
            if let Some(queue) = &self.queue {
                // Blocks while the encoder is behind: the queue stays short.
                if queue.send(layer).is_err() {
                    self.record(Err(std::io::Error::other("the noise map encoder stopped")));
                }
            }
        } else {
            let result = self.write_band(&layer);
            self.record(result);
        }
    }
}

impl Drop for NoiseMapWriter {
    fn drop(&mut self) {
        drop(self.queue.take());
        if let Some(encoder) = self.encoder.take() {
            let _ = encoder.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values covering both ramps, with smooth areas, sharp edges and noise,
    /// so every filter type gets chosen somewhere.
    fn values(width: u32, height: u32) -> Vec<f32> {
        let mut state = 12345u32;
        (0..width * height)
            .map(|i| {
                let (x, y) = ((i % width) as f32, (i / width) as f32);
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let noise = (state >> 8) as f32 / (1 << 24) as f32;
                match (i / width) % 4 {
                    0 => (x / width as f32) * 2.0 - 1.0,
                    1 => ((x * 0.3).sin() * (y * 0.2).cos()).clamp(-1.0, 1.0),
                    2 => noise * 2.0 - 1.0,
                    _ => {
                        if x < width as f32 / 2.0 {
                            -0.4
                        } else {
                            0.7
                        }
                    }
                }
            })
            .collect()
    }

    #[test]
    fn maps_match_the_image_crate_byte_for_byte() {
        let dir = std::env::temp_dir().join(format!("mapgen-noise-maps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (width, height) in [(1, 1), (2, 3), (37, 19), (300, 41)] {
            let writer = NoiseMapWriter::new(&dir, width, height).unwrap();
            let values = values(width, height);
            // Whole, through the background thread.
            writer.layer(DebugLayer {
                name: "whole",
                ramp: Ramp::Signed,
                values: values.clone(),
            });
            // In bands of 4 rows.
            for band in values.chunks(4 * width as usize) {
                writer.layer(DebugLayer {
                    name: "bands",
                    ramp: Ramp::Unsigned,
                    values: band.to_vec(),
                });
            }
            writer.finish().unwrap();
            for (name, ramp) in [("whole", Ramp::Signed), ("bands", Ramp::Unsigned)] {
                let expected = dir.join(format!("{name}-expected.png"));
                image::RgbImage::from_fn(width, height, |x, y| {
                    image::Rgb(colorize(ramp, values[(y * width + x) as usize]))
                })
                .save(&expected)
                .unwrap();
                let written = std::fs::read(dir.join(format!("{name}.png"))).unwrap();
                assert!(
                    written == std::fs::read(&expected).unwrap(),
                    "{name} map of {width} × {height} differs"
                );
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_incomplete_map_is_an_error() {
        let dir = std::env::temp_dir().join(format!("mapgen-noise-short-{}", std::process::id()));
        let writer = NoiseMapWriter::new(&dir, 4, 4).unwrap();
        writer.layer(DebugLayer {
            name: "short",
            ramp: Ramp::Unsigned,
            values: vec![0.5; 8],
        });
        assert!(writer.finish().is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
