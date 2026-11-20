//! A reader of the NetCDF classic format (CDF-1 and CDF-2), enough for
//! MT_CKD's water continuum file `absco-ref_wv-mt-ckd.nc`: its
//! non-record variables of type `float` or `double`, by name.
//!
//! The format (Unidata, "The NetCDF Classic Format Specification",
//! docs.unidata.ucar.edu/netcdf-c/current/file_format_specifications.html)
//! is big-endian: the magic `CDF` and a version byte, the record count,
//! then the lists of dimensions, global attributes and variables. Each
//! variable gives its dimensions, its attributes, its type, its size
//! and the offset of its data.

/// The `float` and `double` variables of a classic NetCDF file, as
/// `f64`, by name.
pub struct NetCdf {
    variables: Vec<(String, Vec<f64>)>,
}

/// Reads the big-endian header of a classic NetCDF file.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.bytes.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_be_bytes(self.take(8)?.try_into().ok()?))
    }

    /// A name: its length, then its bytes padded to 4.
    fn name(&mut self) -> Option<String> {
        let n = self.u32()? as usize;
        let s = self.take(n.div_ceil(4) * 4)?;
        Some(String::from_utf8_lossy(&s[..n]).into_owned())
    }

    /// Skips an attribute list (absent, or `NC_ATTRIBUTE` and its items).
    fn skip_attributes(&mut self) -> Option<()> {
        let tag = self.u32()?;
        let n = self.u32()?;
        if tag == 0 {
            return Some(());
        }
        if tag != NC_ATTRIBUTE {
            return None;
        }
        for _ in 0..n {
            self.name()?;
            let kind = self.u32()?;
            let count = self.u32()? as usize;
            let bytes = count * size_of_type(kind)?;
            self.take(bytes.div_ceil(4) * 4)?;
        }
        Some(())
    }
}

const NC_DIMENSION: u32 = 0x0A;
const NC_VARIABLE: u32 = 0x0B;
const NC_ATTRIBUTE: u32 = 0x0C;
const NC_FLOAT: u32 = 5;
const NC_DOUBLE: u32 = 6;

/// Bytes per value of a NetCDF type (byte, char, short, int, float,
/// double).
fn size_of_type(kind: u32) -> Option<usize> {
    match kind {
        1 | 2 => Some(1),
        3 => Some(2),
        4 | 5 => Some(4),
        6 => Some(8),
        _ => None,
    }
}

impl NetCdf {
    /// Parses a classic NetCDF file; `None` if it is not one.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader { bytes, at: 0 };
        let magic = r.take(4)?;
        if &magic[..3] != b"CDF" || !matches!(magic[3], 1 | 2) {
            return None;
        }
        let offsets_64 = magic[3] == 2;
        r.u32()?; // records
        let tag = r.u32()?;
        let n_dims = r.u32()? as usize;
        let mut dims = Vec::new();
        if tag == NC_DIMENSION {
            for _ in 0..n_dims {
                r.name()?;
                dims.push(r.u32()? as usize);
            }
        } else if tag != 0 {
            return None;
        }
        r.skip_attributes()?;
        let tag = r.u32()?;
        let n_vars = r.u32()? as usize;
        if tag != NC_VARIABLE && !(tag == 0 && n_vars == 0) {
            return None;
        }
        let mut variables = Vec::new();
        for _ in 0..n_vars {
            let name = r.name()?;
            let rank = r.u32()? as usize;
            let mut count = 1usize;
            for _ in 0..rank {
                count *= *dims.get(r.u32()? as usize)?;
            }
            r.skip_attributes()?;
            let kind = r.u32()?;
            r.u32()?; // vsize
            let begin = if offsets_64 {
                r.u64()? as usize
            } else {
                r.u32()? as usize
            };
            let size = size_of_type(kind)?;
            let data = bytes.get(begin..begin + count * size)?;
            let values = match kind {
                NC_FLOAT => data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| f64::from(f32::from_be_bytes(*c)))
                    .collect(),
                NC_DOUBLE => data
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|c| f64::from_be_bytes(*c))
                    .collect(),
                _ => continue,
            };
            variables.push((name, values));
        }
        Some(Self { variables })
    }

    /// The values of the variable `name`, if the file has it as `float`
    /// or `double`.
    pub fn variable(&self, name: &str) -> Option<&[f64]> {
        self.variables
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// The names of the file's `float` and `double` variables.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn names(&self) -> Vec<&str> {
        self.variables.iter().map(|(n, _)| n.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A CDF-1 file with one dimension `x` of 3, one global attribute,
    /// a `double` variable `a` and a `float` variable `b` over it.
    fn sample() -> Vec<u8> {
        let mut h = Vec::new();
        let u = |h: &mut Vec<u8>, x: u32| h.extend_from_slice(&x.to_be_bytes());
        let name = |h: &mut Vec<u8>, s: &str| {
            h.extend_from_slice(&(s.len() as u32).to_be_bytes());
            h.extend_from_slice(s.as_bytes());
            h.resize(h.len().div_ceil(4) * 4, 0);
        };
        h.extend_from_slice(b"CDF\x01");
        u(&mut h, 0);
        u(&mut h, NC_DIMENSION);
        u(&mut h, 1);
        name(&mut h, "x");
        u(&mut h, 3);
        u(&mut h, NC_ATTRIBUTE);
        u(&mut h, 1);
        name(&mut h, "Title");
        u(&mut h, 2);
        u(&mut h, 5);
        h.extend_from_slice(b"hello\0\0\0");
        u(&mut h, NC_VARIABLE);
        u(&mut h, 2);
        let header_end = |h: &Vec<u8>| h.len();
        name(&mut h, "a");
        u(&mut h, 1);
        u(&mut h, 0);
        u(&mut h, 0);
        u(&mut h, 0);
        u(&mut h, NC_DOUBLE);
        u(&mut h, 24);
        let a_begin_at = header_end(&h);
        u(&mut h, 0);
        name(&mut h, "b");
        u(&mut h, 1);
        u(&mut h, 0);
        u(&mut h, 0);
        u(&mut h, 0);
        u(&mut h, NC_FLOAT);
        u(&mut h, 12);
        let b_begin_at = header_end(&h);
        u(&mut h, 0);
        let a_begin = h.len() as u32;
        for x in [1.5f64, -2.0, 3.25] {
            h.extend_from_slice(&x.to_be_bytes());
        }
        let b_begin = h.len() as u32;
        for x in [0.5f32, 4.0, 1e-20] {
            h.extend_from_slice(&x.to_be_bytes());
        }
        h[a_begin_at..a_begin_at + 4].copy_from_slice(&a_begin.to_be_bytes());
        h[b_begin_at..b_begin_at + 4].copy_from_slice(&b_begin.to_be_bytes());
        h
    }

    #[test]
    fn a_classic_file_reads_its_variables() {
        let nc = NetCdf::parse(&sample()).expect("parses");
        assert_eq!(nc.names(), vec!["a", "b"]);
        assert_eq!(nc.variable("a"), Some(&[1.5, -2.0, 3.25][..]));
        let b = nc.variable("b").expect("b");
        assert_eq!(b[..2], [0.5, 4.0]);
        assert!((b[2] / 1e-20 - 1.0).abs() < 1e-6);
        assert!(nc.variable("c").is_none());
        assert!(NetCdf::parse(b"HDF\x01rest").is_none());
    }
}
