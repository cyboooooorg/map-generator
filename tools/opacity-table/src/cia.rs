//! HITRAN's collision-induced absorption files, binned on 20 cm⁻¹
//! cells: CO₂–CO₂ (`CO2-CO2_2024.cia`) at four temperatures
//! ([`Cia::binned`]), and the nitrogen, oxygen and methane pairs at
//! every 10 K from 70 to 400 K ([`Cia::interpolated`]).
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

impl Cia {
    /// The cells of `cell_cm` cm⁻¹ from 0 up to `top_cm`: the start of
    /// each and its mean coefficient at each of `temperatures`
    /// (cm⁵/molecule²), the method of study 36's binning (Wiki study 36,
    /// "Data and cost"), for files whose blocks cover one band at many
    /// temperatures.
    ///
    /// The blocks are grouped into bands: a block joins the first band
    /// one of whose blocks shares wavenumbers with it, or starts its own.
    /// In each band, each block's coefficient is averaged over the cell
    /// (the trapezoid integral of its piecewise linear curve, a negative
    /// point of the file's noise taken as zero, zero outside the block's
    /// points); blocks of one band at the same temperature are averaged
    /// (none of the five pair files has two);
    /// then the band's means are interpolated linearly in temperature
    /// between its blocks, held at its coldest and warmest. The bands
    /// that reach a cell add up. Cells under [`NEGLIGIBLE`] at every
    /// temperature are left out.
    pub fn interpolated(
        &self,
        cell_cm: f64,
        top_cm: f64,
        temperatures: &[f64],
    ) -> Vec<(f64, Vec<f64>)> {
        let clamped: Vec<Set> = self
            .sets
            .iter()
            .map(|s| Set {
                points: s.points.iter().map(|&(nu, k)| (nu, k.max(0.0))).collect(),
                ..s.clone()
            })
            .collect();
        let mut bands: Vec<Vec<&Set>> = Vec::new();
        for set in &clamped {
            let shares = |other: &&Set| {
                !(set.band_cm.1 < other.band_cm.0 || set.band_cm.0 > other.band_cm.1)
            };
            match bands.iter_mut().find(|band| band.iter().any(shares)) {
                Some(band) => band.push(set),
                None => bands.push(vec![set]),
            }
        }
        let cells = (top_cm / cell_cm).round() as usize;
        (0..cells)
            .filter_map(|c| {
                let (lo, hi) = (c as f64 * cell_cm, (c + 1) as f64 * cell_cm);
                let mut values = vec![0.0; temperatures.len()];
                for band in &bands {
                    if band.iter().all(|s| s.band_cm.1 < lo || s.band_cm.0 > hi) {
                        continue;
                    }
                    // The band's mean at each of its temperatures, ascending.
                    let mut by_t: Vec<(f64, f64, usize)> = Vec::new();
                    for s in band {
                        let mean = trapezoid_mean(&s.points, false, lo, hi);
                        match by_t.iter_mut().find(|(t, _, _)| *t == s.temperature_k) {
                            Some((_, sum, n)) => {
                                *sum += mean;
                                *n += 1;
                            }
                            None => by_t.push((s.temperature_k, mean, 1)),
                        }
                    }
                    by_t.sort_by(|a, b| a.0.total_cmp(&b.0));
                    let curve: Vec<(f64, f64)> = by_t
                        .iter()
                        .map(|&(t, sum, n)| (t, sum / n as f64))
                        .collect();
                    for (value, &t) in values.iter_mut().zip(temperatures) {
                        *value += interpolate(&curve, t);
                    }
                }
                values
                    .iter()
                    .any(|&v| v >= NEGLIGIBLE)
                    .then_some((lo, values))
            })
            .collect()
    }
}

/// The value at `t` of the piecewise linear curve through `points` (`t`
/// ascending), held at its ends.
fn interpolate(points: &[(f64, f64)], t: f64) -> f64 {
    let (first, last) = (points[0], points[points.len() - 1]);
    if t <= first.0 {
        return first.1;
    }
    if t >= last.0 {
        return last.1;
    }
    let w = points
        .windows(2)
        .find(|w| w[0].0 <= t && t <= w[1].0)
        .expect("t is inside the curve");
    let f = (t - w[0].0) / (w[1].0 - w[0].0);
    w[0].1 + (w[1].1 - w[0].1) * f
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

    /// A flat block at `t` K over `lo`–`hi` cm⁻¹ (points every 1 cm⁻¹)
    /// of coefficient `k`.
    fn flat_block(t: f64, (lo, hi): (f64, f64), k: f64) -> Set {
        Set {
            temperature_k: t,
            band_cm: (lo, hi),
            points: (lo as usize..=hi as usize).map(|n| (n as f64, k)).collect(),
        }
    }

    #[test]
    fn a_band_is_interpolated_in_temperature_and_held_at_its_ends() {
        let cia = Cia {
            sets: vec![
                flat_block(100.0, (0.0, 450.0), 4e-45),
                flat_block(200.0, (0.0, 550.0), 2e-45),
            ],
        };
        let cells = cia.interpolated(20.0, 600.0, &[70.0, 100.0, 150.0, 200.0, 400.0]);
        let (_, k) = cells.iter().find(|c| c.0 == 100.0).expect("cell");
        for (got, want) in k.iter().zip([4e-45, 4e-45, 3e-45, 2e-45, 2e-45]) {
            assert!((got / want - 1.0).abs() < 1e-12, "{k:?}");
        }
        // Past the cold block's end only the warm block holds: the cold
        // one's zero is interpolated with it.
        let (_, k) = cells.iter().find(|c| c.0 == 500.0).expect("cell");
        for (got, want) in k.iter().zip([0.0, 0.0, 1e-45, 2e-45, 2e-45]) {
            assert!((got - want).abs() < 1e-57, "{k:?}");
        }
        assert!(cells.iter().all(|c| c.0 < 560.0));
    }

    #[test]
    fn separate_bands_add_and_same_temperatures_average() {
        // Two bands that share no wavenumber, both reaching the cell
        // 1840-1860 cm⁻¹, add there; two blocks of one band at the same
        // temperature average.
        let cia = Cia {
            sets: vec![
                flat_block(200.0, (1300.0, 1849.0), 1e-45),
                flat_block(300.0, (1850.0, 3000.0), 2e-45),
                flat_block(300.0, (1850.0, 3000.0), 4e-45),
            ],
        };
        let cells = cia.interpolated(20.0, 5000.0, &[250.0]);
        let at = |start: f64| cells.iter().find(|c| c.0 == start).expect("cell").1[0];
        assert!((at(1500.0) / 1e-45 - 1.0).abs() < 1e-12);
        // The two 300 K blocks average to 3e-45.
        assert!((at(2200.0) / 3e-45 - 1.0).abs() < 1e-12);
        // 1840-1849 of the first band (9/20 of the cell, its last point
        // at 1849) and 1850-1860 of the second (10/20).
        let edge = at(1840.0);
        let want = 1e-45 * 9.0 / 20.0 + 3e-45 * 10.0 / 20.0;
        assert!((edge / want - 1.0).abs() < 1e-12, "{edge}");
    }

    #[test]
    fn a_negative_point_counts_as_zero_when_interpolated() {
        let mut block = flat_block(100.0, (0.0, 100.0), 1e-45);
        for point in &mut block.points[40..60] {
            point.1 = -1e-45;
        }
        let cells = Cia { sets: vec![block] }.interpolated(20.0, 100.0, &[100.0]);
        let (_, k) = cells.iter().find(|c| c.0 == 40.0).expect("cell");
        assert!(k[0] >= 0.0 && k[0] < 1e-45, "{k:?}");
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
