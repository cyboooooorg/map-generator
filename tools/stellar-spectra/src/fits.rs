//! The few pieces of FITS (Pence et al. 2010, A&A 524, A42) the spectra
//! need: a header's keywords, a one-dimensional image of IEEE floats and
//! the float columns of a binary table, all big-endian.

use std::collections::BTreeMap;

/// Definition: bytes in a FITS block (FITS 4.0, §3.1).
const BLOCK: usize = 2880;

/// Definition: bytes in a header card (FITS 4.0, §4.1).
const CARD: usize = 80;

/// One header-and-data unit: its keywords (values as written, strings
/// without their quotes) and its data bytes.
pub struct Hdu<'a> {
    pub keys: BTreeMap<String, String>,
    pub data: &'a [u8],
}

impl Hdu<'_> {
    /// The integer value of `key`.
    pub fn int(&self, key: &str) -> Result<i64, String> {
        self.keys
            .get(key)
            .ok_or_else(|| format!("no {key}"))?
            .parse()
            .map_err(|e| format!("{key}: {e}"))
    }

    /// The string value of `key`.
    pub fn text(&self, key: &str) -> Result<&str, String> {
        self.keys
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("no {key}"))
    }
}

/// The header-and-data units of a FITS file, in order.
pub fn units(bytes: &[u8]) -> Result<Vec<Hdu<'_>>, String> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let mut keys = BTreeMap::new();
        'header: loop {
            let block = bytes
                .get(at..at + BLOCK)
                .ok_or("a header runs past the end of the file")?;
            at += BLOCK;
            for card in block.chunks(CARD) {
                let card = String::from_utf8_lossy(card);
                let name = card[..8].trim();
                if name == "END" {
                    break 'header;
                }
                if &card[8..10] == "= " {
                    let value = card[10..].split('/').next().unwrap_or("").trim();
                    keys.insert(name.to_owned(), value.trim_matches('\'').trim().to_owned());
                }
            }
        }
        let hdu = Hdu { keys, data: &[] };
        let bitpix = hdu.int("BITPIX")?;
        let naxis = hdu.int("NAXIS")?;
        let mut count: i64 = i64::from(naxis > 0);
        for k in 1..=naxis {
            count *= hdu.int(&format!("NAXIS{k}"))?;
        }
        let pcount = hdu.int("PCOUNT").unwrap_or(0);
        let size = usize::try_from(bitpix.abs() / 8 * count + pcount).map_err(|e| e.to_string())?;
        let data = bytes
            .get(at..at + size)
            .ok_or("the data run past the end of the file")?;
        at += size.div_ceil(BLOCK) * BLOCK;
        out.push(Hdu {
            keys: hdu.keys,
            data,
        });
    }
    Ok(out)
}

/// The values of a one-dimensional image of 32- or 64-bit floats.
pub fn image(hdu: &Hdu) -> Result<Vec<f64>, String> {
    match hdu.int("BITPIX")? {
        -32 => Ok(hdu
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f64::from(f32::from_be_bytes(*b)))
            .collect()),
        -64 => Ok(hdu
            .data
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_be_bytes(*b))
            .collect()),
        other => Err(format!("BITPIX {other} is not a float image")),
    }
}

/// The column `name` of a binary table whose columns are single floats
/// (`E`) or doubles (`D`).
pub fn column(hdu: &Hdu, name: &str) -> Result<Vec<f64>, String> {
    let width = usize::try_from(hdu.int("NAXIS1")?).map_err(|e| e.to_string())?;
    let rows = usize::try_from(hdu.int("NAXIS2")?).map_err(|e| e.to_string())?;
    let fields = hdu.int("TFIELDS")?;
    let mut offset = 0;
    for c in 1..=fields {
        let form = hdu.text(&format!("TFORM{c}"))?;
        let size = match form.chars().last() {
            Some('E') => 4,
            Some('D') => 8,
            _ => return Err(format!("column {c}: format {form} is not a float")),
        };
        if hdu.text(&format!("TTYPE{c}"))? == name {
            return Ok((0..rows)
                .map(|r| {
                    let b = &hdu.data[r * width + offset..r * width + offset + size];
                    if size == 4 {
                        f64::from(f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                    } else {
                        f64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                    }
                })
                .collect());
        }
        offset += size;
    }
    Err(format!("no column {name}"))
}
