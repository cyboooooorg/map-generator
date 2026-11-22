//! HITRAN's CO₂–CO₂ collision-induced absorption (`CO2-CO2_2024.cia`),
//! binned on 20 cm⁻¹ cells at four temperatures.
//!
//! HITRAN's `.cia` format (Karman et al. 2019, *Icarus* 328, 160,
//! doi:10.1016/j.icarus.2019.02.034, section 2): a block per
//! temperature, each opened by a header line (name, first and last
//! wavenumber, number of points, temperature, maximum, resolution), then
//! one `ν coefficient` row per point, cm⁻¹ and cm⁵/molecule². The CO₂–CO₂
//! file stitches measurements and calculations that cover different
//! bands at different temperatures (see [`REGIONS`]), so a cell takes,
//! at each target temperature, the set nearest it among those of its
//! region.

/// One block of the file: the coefficients at one temperature over one
/// band.
#[derive(Clone, Debug, PartialEq)]
pub struct Set {
    /// Temperature, K.
    pub temperature_k: f64,
    /// First and last wavenumber of the block's header, cm⁻¹.
    pub band_cm: (f64, f64),
    /// `(ν, k)` rows, ν ascending, cm⁻¹ and cm⁵/molecule².
    pub points: Vec<(f64, f64)>,
}

/// The file: every block, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Cia {
    sets: Vec<Set>,
}

/// A band of the file the binning treats as one: the file's sets whose
/// band lies within `lo`–`hi` cm⁻¹ (1 cm⁻¹ of slack on each side), and
/// whether the coefficient is taken as zero at ν = 0 (the
/// roto-translational band starts at 1 cm⁻¹ and falls to nothing at 0).
///
/// Sources, from HITRAN's `CIA_Readme_2024.pdf`: 1-750 cm⁻¹ Gruszka &
/// Borysow 1997; 1120-1850 cm⁻¹ Terragni et al. 2025 and Baranov,
/// Lafferty & Fraser 2004; 2510-2850 cm⁻¹ Baranov et al. 2003;
/// 2850-3250 and 3995-4500 cm⁻¹ Baranov 2018 and Tran et al. 2024.
struct Region {
    lo: f64,
    hi: f64,
    zero_at_origin: bool,
}

/// The bands, in the order they take precedence over each other where
/// they overlap (2840-2860 cm⁻¹: the earlier band wins); a cell belongs
/// to the first band it touches.
const REGIONS: [Region; 5] = [
    Region {
        lo: 0.0,
        hi: 760.0,
        zero_at_origin: true,
    },
    Region {
        lo: 1100.0,
        hi: 1860.0,
        zero_at_origin: false,
    },
    Region {
        lo: 2500.0,
        hi: 2860.0,
        zero_at_origin: false,
    },
    Region {
        lo: 2840.0,
        hi: 3260.0,
        zero_at_origin: false,
    },
    Region {
        lo: 3990.0,
        hi: 4510.0,
        zero_at_origin: false,
    },
];

/// Cells whose coefficient is under this, cm⁵/molecule², at every
/// temperature are left out: at Venus's 92 bar and 735 K they would add
/// under 0.013 to the optical depth per `ln p`.
const NEGLIGIBLE: f64 = 1e-50;

impl Cia {
    /// Reads the blocks of a `.cia` file; `None` when a header or a row
    /// is unreadable, or a block has fewer rows than its header says.
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let mut sets = Vec::new();
        while let Some(header) = lines.next() {
            let fields: Vec<&str> = header.split_whitespace().collect();
            if fields.len() < 5 {
                return None;
            }
            let first: f64 = fields[1].parse().ok()?;
            let last: f64 = fields[2].parse().ok()?;
            let count: usize = fields[3].parse().ok()?;
            let temperature_k: f64 = fields[4].parse().ok()?;
            let mut points = Vec::with_capacity(count);
            for _ in 0..count {
                let mut row = lines.next()?.split_whitespace();
                let nu: f64 = row.next()?.parse().ok()?;
                let k: f64 = row.next()?.parse().ok()?;
                points.push((nu, k));
            }
            sets.push(Set {
                temperature_k,
                band_cm: (first, last),
                points,
            });
        }
        (!sets.is_empty()).then_some(Self { sets })
    }

    /// The blocks of the file.
    #[cfg(test)]
    pub fn sets(&self) -> &[Set] {
        &self.sets
    }

    /// The cells of `cell_cm` cm⁻¹ from 0: the start of each and its
    /// mean coefficient at each of `temperatures` (cm⁵/molecule²).
    ///
    /// The mean is the trapezoid integral of the file's piecewise linear
    /// coefficient over the cell, divided by the cell's width (a cell the
    /// set only partly covers counts the rest as zero; a mean under zero,
    /// from a file's negative noise, counts as zero). Each region takes
    /// the set nearest each temperature among its own. Cells under
    /// [`NEGLIGIBLE`] at every temperature are left out.
    pub fn binned(&self, cell_cm: f64, temperatures: &[f64; 4]) -> Vec<(f64, [f64; 4])> {
        let top = REGIONS.iter().map(|r| r.hi).fold(0.0, f64::max);
        let cells = (top / cell_cm).ceil() as usize;
        (0..cells)
            .filter_map(|c| {
                let start = c as f64 * cell_cm;
                let region = REGIONS
                    .iter()
                    .find(|r| r.lo < start + cell_cm && start < r.hi)?;
                let candidates: Vec<&Set> = self
                    .sets
                    .iter()
                    .filter(|s| s.band_cm.0 >= region.lo - 1.0 && s.band_cm.1 <= region.hi + 1.0)
                    .collect();
                let mut values = [0.0; 4];
                for (value, &t) in values.iter_mut().zip(temperatures) {
                    let nearest = candidates.iter().min_by(|a, b| {
                        (a.temperature_k - t)
                            .abs()
                            .total_cmp(&(b.temperature_k - t).abs())
                    })?;
                    *value = trapezoid_mean(
                        &nearest.points,
                        region.zero_at_origin,
                        start,
                        start + cell_cm,
                    )
                    .max(0.0);
                }
                values
                    .iter()
                    .any(|&v| v >= NEGLIGIBLE)
                    .then_some((start, values))
            })
            .collect()
    }
}

/// The mean over `lo`–`hi` of the piecewise linear curve through
/// `points` (with a zero at ν = 0 when asked), zero outside them.
fn trapezoid_mean(points: &[(f64, f64)], zero_at_origin: bool, lo: f64, hi: f64) -> f64 {
    let origin = [(0.0, 0.0)];
    let curve = origin
        .iter()
        .take(usize::from(zero_at_origin))
        .chain(points)
        .copied()
        .collect::<Vec<_>>();
    let mut area = 0.0;
    for w in curve.windows(2) {
        let ((a, x), (b, y)) = (w[0], w[1]);
        let (from, to) = (a.max(lo), b.min(hi));
        if to > from {
            let at = |nu: f64| x + (y - x) * (nu - a) / (b - a);
            area += 0.5 * (at(from) + at(to)) * (to - from);
        }
    }
    area / (hi - lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two blocks of a CO₂–CO₂ file: the first at 200 K over 1-4 cm⁻¹,
    /// the second at 300 K over 1-4 cm⁻¹, with HITRAN's header layout.
    fn sample() -> String {
        "         CO2-CO2   1.000000    4.00000    4  200.00 1.961e-43 -.999   20\n\
         1.0000  1.000E-44\n2.0000  2.000E-44\n3.0000  3.000E-44\n4.0000  4.000E-44\n\
         \x20        CO2-CO2   1.000000    4.00000    4  300.00 1.961e-43 -.999   20\n\
         1.0000  1.000E-45\n2.0000  2.000E-45\n3.0000  3.000E-45\n4.0000  4.000E-45\n"
            .to_string()
    }

    #[test]
    fn a_cia_block_parses() {
        let cia = Cia::parse(&sample()).expect("parses");
        assert_eq!(cia.sets().len(), 2);
        assert_eq!(cia.sets()[0].temperature_k, 200.0);
        assert_eq!(cia.sets()[0].band_cm, (1.0, 4.0));
        assert_eq!(cia.sets()[1].temperature_k, 300.0);
        assert_eq!(cia.sets()[1].points[3], (4.0, 4e-45));
        // A block that promises more rows than it has is refused.
        let cut = sample().replace("4.0000  4.000E-45\n", "");
        assert!(Cia::parse(&cut).is_none());
    }

    #[test]
    fn cia_cells_are_trapezoid_means() {
        // A coefficient linear in ν, k = 1e-45 ν, over 1-750 cm⁻¹: the
        // mean over a cell is its value at the cell's midpoint.
        let points: Vec<(f64, f64)> = (1..=750).map(|n| (n as f64, 1e-45 * n as f64)).collect();
        let set = |t| Set {
            temperature_k: t,
            band_cm: (1.0, 750.0),
            points: points.clone(),
        };
        let cia = Cia {
            sets: vec![set(200.0)],
        };
        let mean = trapezoid_mean(&points, false, 100.0, 120.0);
        assert!((mean / 110e-45 - 1.0).abs() < 1e-12, "{mean}");
        // With the origin pinned to zero the first cell is [0, 20): its
        // mean is the midpoint's value too.
        let mean = trapezoid_mean(&points, true, 0.0, 20.0);
        assert!((mean / 10e-45 - 1.0).abs() < 1e-12, "{mean}");
        // Binned through the regions: the roto-translational cell at 100
        // cm⁻¹ and the same at every temperature (one set only).
        let cells = cia.binned(20.0, &[200.0, 300.0, 450.0, 800.0]);
        let (start, k) = cells.iter().find(|c| c.0 == 100.0).expect("cell");
        assert_eq!(*start, 100.0);
        assert!(k.iter().all(|v| (v / 110e-45 - 1.0).abs() < 1e-12), "{k:?}");
        // Nothing past the file's band.
        assert!(cells.iter().all(|c| c.0 < 760.0));
    }

    #[test]
    fn each_temperature_takes_its_nearest_set() {
        let flat = |t: f64, k: f64| Set {
            temperature_k: t,
            band_cm: (1.0, 750.0),
            points: (1..=750).map(|n| (n as f64, k)).collect(),
        };
        let cia = Cia {
            sets: vec![flat(200.0, 1e-44), flat(300.0, 1e-45), flat(800.0, 1e-46)],
        };
        let cells = cia.binned(20.0, &[200.0, 300.0, 450.0, 800.0]);
        let (_, k) = cells.iter().find(|c| c.0 == 200.0).expect("cell");
        // 450 K is nearer 300 K (150) than 800 K (350).
        for (got, want) in k.iter().zip([1e-44, 1e-45, 1e-45, 1e-46]) {
            assert!((got / want - 1.0).abs() < 1e-12, "{k:?}");
        }
    }

    #[test]
    fn a_negative_mean_counts_as_zero() {
        let set = Set {
            temperature_k: 200.0,
            band_cm: (1.0, 750.0),
            points: (1..=750)
                .map(|n| (n as f64, if n < 200 { 1e-44 } else { -1e-44 }))
                .collect(),
        };
        let cells = Cia { sets: vec![set] }.binned(20.0, &[200.0; 4]);
        assert!(cells.iter().all(|c| c.1.iter().all(|&v| v >= 0.0)));
        assert!(cells.iter().any(|c| c.0 == 100.0));
        assert!(cells.iter().all(|c| c.0 < 200.0));
    }

    #[test]
    fn negligible_cells_are_left_out() {
        let set = Set {
            temperature_k: 200.0,
            band_cm: (1.0, 750.0),
            points: (1..=750).map(|n| (n as f64, 1e-60)).collect(),
        };
        let cells = Cia { sets: vec![set] }.binned(20.0, &[200.0; 4]);
        assert!(cells.is_empty());
    }
}
