//! The stellar tables of `crates/science/data`, parsed once.
//!
//! The tables are part of the build (`include_str!`): a malformed line is a
//! bug of the build, not an input error, so parsing panics with the file and
//! line number. Every value is described in `crates/science/data/STELLAR.md`.

use std::sync::OnceLock;

/// The YREC main-sequence tracks, 0.3-8 M☉.
const TRACKS: &str = include_str!("../../data/stellar-tracks.tsv");
/// The Baraffe rows and Dartmouth offsets, 0.08-0.4 M☉.
const LOW_MASS: &str = include_str!("../../data/stellar-low-mass.tsv");

/// Definition: a YREC track reaches the TAMS when its last row has
/// xc_frac below 1e-3; one that stops at 20 Gyr first ends at xc_frac
/// 1.64e-3 or more (`STELLAR.md`, The YREC grid).
const TAMS_XC_FRAC: f64 = 1e-3;

/// Definition: the ZAMS of a Baraffe track is the first age at which
/// |Δ log R / Δ log t| between consecutive rows falls below 0.01
/// (`STELLAR.md`, `stellar-low-mass.tsv`).
const ZAMS_RADIUS_SLOPE: f64 = 0.01;

/// One row of a YREC track, all logarithms decimal.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TrackRow {
    /// Age since the start of the run, Gyr.
    pub age: f64,
    pub log_l: f64,
    pub log_r: f64,
    pub log_teff: f64,
    /// Central hydrogen over its ZAMS value.
    pub xc_frac: f64,
}

/// A YREC track of one mass and metallicity, from its ZAMS row on.
#[derive(Debug)]
pub(crate) struct Track {
    pub mass: f64,
    pub feh: f64,
    pub rows: Vec<TrackRow>,
}

impl Track {
    /// Age of the ZAMS (the first row), Gyr.
    pub fn zams(&self) -> f64 {
        self.rows[0].age
    }

    /// Age of the last row, Gyr: the TAMS when [`Track::reaches_tams`],
    /// otherwise the end of the run (20 Gyr).
    pub fn end(&self) -> f64 {
        self.last().age
    }

    /// Whether the run reached the TAMS before 20 Gyr.
    pub fn reaches_tams(&self) -> bool {
        self.last().xc_frac < TAMS_XC_FRAC
    }

    fn last(&self) -> &TrackRow {
        &self.rows[self.rows.len() - 1]
    }
}

/// One Baraffe row, all logarithms decimal.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LowMassRow {
    /// Age, Gyr.
    pub age: f64,
    pub log_teff: f64,
    pub log_l: f64,
    pub log_r: f64,
}

/// A Baraffe track of one mass at solar metallicity.
#[derive(Debug)]
pub(crate) struct LowMassTrack {
    pub mass: f64,
    pub rows: Vec<LowMassRow>,
    /// Age of the ZAMS by [`ZAMS_RADIUS_SLOPE`], Gyr.
    pub zams: f64,
}

/// A Dartmouth offset: value at `feh` minus value at [Fe/H] 0, in dex.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Offset {
    pub mass: f64,
    pub feh: f64,
    pub d_log_teff: f64,
    pub d_log_l: f64,
    pub d_log_r: f64,
}

/// All stellar tables.
#[derive(Debug)]
pub(crate) struct Tables {
    /// YREC tracks sorted by metallicity, then mass.
    pub tracks: Vec<Track>,
    /// The YREC metallicities, increasing.
    pub metallicities: Vec<f64>,
    /// Baraffe tracks by increasing mass.
    pub low_mass: Vec<LowMassTrack>,
    /// Dartmouth offsets sorted by metallicity, then mass.
    pub offsets: Vec<Offset>,
}

impl Tables {
    /// The YREC tracks of one tabulated metallicity, by increasing mass.
    pub fn tracks_at(&self, feh: f64) -> &[Track] {
        let start = self.tracks.partition_point(|t| t.feh < feh);
        let end = self.tracks.partition_point(|t| t.feh <= feh);
        &self.tracks[start..end]
    }
}

/// The parsed tables, built on first use.
pub(crate) fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let tracks = parse_tracks(TRACKS);
        let mut metallicities: Vec<f64> = tracks.iter().map(|t| t.feh).collect();
        metallicities.dedup();
        let (low_mass, offsets) = parse_low_mass(LOW_MASS);
        Tables {
            tracks,
            metallicities,
            low_mass,
            offsets,
        }
    })
}

/// The data lines of a table, numbered from 1 as in the file, comments and
/// blank lines left out.
fn data_lines(text: &str) -> impl Iterator<Item = (usize, Vec<&str>)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#') && !line.trim().is_empty())
        .map(|(index, line)| (index + 1, line.split('\t').collect()))
}

fn number(file: &str, line: usize, field: &str) -> f64 {
    match field.trim().parse::<f64>() {
        Ok(value) if value.is_finite() => value,
        _ => panic!("{file} line {line}: {field:?} is not a finite number"),
    }
}

fn numbers<const N: usize>(file: &str, line: usize, fields: &[&str]) -> [f64; N] {
    if fields.len() != N {
        panic!("{file} line {line}: {} fields, expected {N}", fields.len());
    }
    std::array::from_fn(|i| number(file, line, fields[i]))
}

fn parse_tracks(text: &str) -> Vec<Track> {
    const FILE: &str = "stellar-tracks.tsv";
    let mut tracks: Vec<Track> = Vec::new();
    for (line, fields) in data_lines(text) {
        let [mass, feh, age, log_l, log_r, log_teff, xc_frac] = numbers(FILE, line, &fields);
        let row = TrackRow {
            age,
            log_l,
            log_r,
            log_teff,
            xc_frac,
        };
        match tracks.last_mut() {
            Some(track) if track.mass == mass && track.feh == feh => {
                if age <= track.end() {
                    panic!("{FILE} line {line}: age {age} does not increase");
                }
                track.rows.push(row);
            }
            _ => tracks.push(Track {
                mass,
                feh,
                rows: vec![row],
            }),
        }
    }
    tracks.sort_by(|a, b| a.feh.total_cmp(&b.feh).then(a.mass.total_cmp(&b.mass)));
    for pair in tracks.windows(2) {
        if pair[0].feh == pair[1].feh && pair[0].mass == pair[1].mass {
            panic!(
                "{FILE}: track {} Msun at {} dex appears twice",
                pair[0].mass, pair[0].feh
            );
        }
    }
    if tracks.is_empty() {
        panic!("{FILE}: no track");
    }
    tracks
}

fn parse_low_mass(text: &str) -> (Vec<LowMassTrack>, Vec<Offset>) {
    const FILE: &str = "stellar-low-mass.tsv";
    let mut tracks: Vec<LowMassTrack> = Vec::new();
    let mut offsets = Vec::new();
    for (line, fields) in data_lines(text) {
        if fields[0] == "D" {
            let [mass, feh, d_log_teff, d_log_l, d_log_r] = numbers(FILE, line, &fields[1..]);
            offsets.push(Offset {
                mass,
                feh,
                d_log_teff,
                d_log_l,
                d_log_r,
            });
            continue;
        }
        let [mass, age, teff, log_l, r] = numbers(FILE, line, &fields);
        if teff <= 0.0 || r <= 0.0 {
            panic!("{FILE} line {line}: T_eff and R must be positive");
        }
        let row = LowMassRow {
            age,
            log_teff: teff.log10(),
            log_l,
            log_r: r.log10(),
        };
        match tracks.last_mut() {
            Some(track) if track.mass == mass => {
                if age <= track.rows[track.rows.len() - 1].age {
                    panic!("{FILE} line {line}: age {age} does not increase");
                }
                track.rows.push(row);
            }
            Some(track) if track.mass > mass => {
                panic!("{FILE} line {line}: masses must increase");
            }
            _ => tracks.push(LowMassTrack {
                mass,
                rows: vec![row],
                zams: 0.0,
            }),
        }
    }
    for track in &mut tracks {
        track.zams = low_mass_zams(&track.rows)
            .unwrap_or_else(|| panic!("{FILE}: {} Msun never settles", track.mass));
    }
    offsets.sort_by(|a, b| a.feh.total_cmp(&b.feh).then(a.mass.total_cmp(&b.mass)));
    if tracks.is_empty() || offsets.is_empty() {
        panic!("{FILE}: no Baraffe track or no Dartmouth offset");
    }
    (tracks, offsets)
}

/// The first age at which the radius stops contracting
/// ([`ZAMS_RADIUS_SLOPE`]): the earlier row of the first pair that settles.
fn low_mass_zams(rows: &[LowMassRow]) -> Option<f64> {
    rows.windows(2).find_map(|pair| {
        let slope = (pair[1].log_r - pair[0].log_r) / (pair[1].age / pair[0].age).log10();
        (slope.abs() < ZAMS_RADIUS_SLOPE).then_some(pair[0].age)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Definition: the YREC grid's four metallicities, in dex (`STELLAR.md`).
    const METALLICITIES: [f64; 4] = [-1.0, -0.5, 0.0, 0.5];

    #[test]
    fn the_tables_parse() {
        let t = tables();
        assert_eq!(t.metallicities, METALLICITIES);
        for feh in METALLICITIES {
            assert!(!t.tracks_at(feh).is_empty(), "{feh}");
        }
        assert!(t.low_mass.iter().all(|track| track.zams > 0.0));
    }

    #[test]
    #[should_panic(expected = "stellar-tracks.tsv line 2")]
    fn a_bad_line_names_its_number() {
        parse_tracks("# header\n0.3\t0\tx\t1\t1\t1\t1\n");
    }
}
