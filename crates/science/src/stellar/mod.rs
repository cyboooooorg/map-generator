//! Main-sequence stars from 0.08 to 8 M☉ and [Fe/H] −1 to +0.5, read from
//! the tables of `crates/science/data` (see `STELLAR.md` there).
//!
//! [`life`] gives the ages at which a star reaches and leaves the main
//! sequence, [`main_sequence`] its radius, luminosity and effective
//! temperature at a given age, [`profile`] its interior. YREC tracks and
//! profiles cover 0.3-8 M☉, Baraffe et al. (2015) with Dartmouth
//! metallicity offsets cover 0.08-0.35 M☉, blended over 0.30-0.35 M☉; below
//! 0.30 M☉ the interior is an n = 1.5 polytrope.

mod polytrope;
mod profile;
mod tables;
mod values;

pub use profile::{StellarLayer, StellarProfile, StellarSample, profile};

pub use values::{
    Life, Lifetime, MAX_MASS, MAX_METALLICITY, MIN_MASS, MIN_METALLICITY, MainSequence,
    StellarError, life, main_sequence,
};

/// Two neighbouring grid points around a value and the weight of the upper
/// one, in `[0, 1]`. `lo == hi` when the value falls on a grid point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bracket {
    pub lo: usize,
    pub hi: usize,
    pub weight: f64,
}

impl Bracket {
    /// The grid indices with a non-zero weight and their weights, which sum
    /// to 1.
    pub fn terms(self) -> impl Iterator<Item = (usize, f64)> {
        let pairs = if self.lo == self.hi {
            [(self.lo, 1.0), (self.hi, 0.0)]
        } else {
            [(self.lo, 1.0 - self.weight), (self.hi, self.weight)]
        };
        pairs.into_iter().filter(|&(_, weight)| weight > 0.0)
    }
}

/// The bracket of `x` in the increasing `grid`, the weight linear in `x`;
/// `None` outside the grid (or for NaN, or an empty grid).
pub(crate) fn bracket(grid: &[f64], x: f64) -> Option<Bracket> {
    bracket_by(grid.len(), |i| grid[i], x, |v| v)
}

/// As [`bracket`], the weight linear in `ln x` (all values positive).
pub(crate) fn bracket_log(grid: &[f64], x: f64) -> Option<Bracket> {
    bracket_by(grid.len(), |i| grid[i], x, f64::ln)
}

/// The bracket of `x` among `len` (at least one) increasing keys `key(i)`,
/// the weight linear in `scale(x)`; outside the keys, the first or the last
/// key alone (the value is held).
pub(crate) fn bracket_clamped(
    len: usize,
    key: impl Fn(usize) -> f64,
    x: f64,
    scale: impl Fn(f64) -> f64,
) -> Bracket {
    let at = |i| Bracket {
        lo: i,
        hi: i,
        weight: 0.0,
    };
    if x <= key(0) {
        return at(0);
    }
    bracket_by(len, key, x, scale).unwrap_or(at(len - 1))
}

fn bracket_by(
    len: usize,
    key: impl Fn(usize) -> f64,
    x: f64,
    scale: impl Fn(f64) -> f64,
) -> Option<Bracket> {
    if len == 0 || !(x >= key(0) && x <= key(len - 1)) {
        return None;
    }
    let lo = (0..len).rev().find(|&i| key(i) <= x).unwrap_or(0);
    if key(lo) == x || lo == len - 1 {
        return Some(Bracket {
            lo,
            hi: lo,
            weight: 0.0,
        });
    }
    let hi = lo + 1;
    let (a, b) = (scale(key(lo)), scale(key(hi)));
    Some(Bracket {
        lo,
        hi,
        weight: (scale(x) - a) / (b - a),
    })
}

/// `a` at weight 0, `b` at weight 1.
pub(crate) fn lerp(a: f64, b: f64, weight: f64) -> f64 {
    a + (b - a) * weight
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Definition: a test grid.
    const GRID: [f64; 3] = [1.0, 2.0, 4.0];

    #[test]
    fn a_bracket_weighs_the_upper_point() {
        let b = bracket(&GRID, 3.0).unwrap();
        assert_eq!((b.lo, b.hi, b.weight), (1, 2, 0.5));
        let log = bracket_log(&GRID, 2.0_f64.powf(1.5)).unwrap();
        assert!((log.weight - 0.5).abs() < 1e-12);
    }

    #[test]
    fn a_grid_point_is_used_alone() {
        for (i, x) in GRID.iter().enumerate() {
            let terms: Vec<_> = bracket(&GRID, *x).unwrap().terms().collect();
            assert_eq!(terms, vec![(i, 1.0)]);
        }
    }

    #[test]
    fn outside_the_grid_there_is_no_bracket() {
        assert_eq!(bracket(&GRID, 0.5), None);
        assert_eq!(bracket(&GRID, f64::NAN), None);
        let held = bracket_clamped(GRID.len(), |i| GRID[i], 9.0, |v| v);
        assert_eq!(held.terms().collect::<Vec<_>>(), vec![(2, 1.0)]);
    }
}
