//! What a planet loses to space over its life: the hydrogen-helium
//! envelope its star's bolometric light boils off when the disc goes, the
//! energy-limited escape of what is left (and of the hydrogen of a steam
//! atmosphere) under its star's X-ray and extreme-ultraviolet (XUV)
//! light, and the cosmic shoreline that splits the bodies with and
//! without air.
//!
//! **Boil-off** (Tang, Fortney & Murray-Clay 2024, arXiv:2410.08577,
//! Table 2; Owen & Wu 2016; Ginzburg, Schlichting & Sari 2016): when the
//! disc disperses, a young planet's envelope escapes as a Parker wind
//! heated by the star's bolometric light, within about a million years,
//! down to a mass fraction that depends on the core's mass and the flux
//! alone ([`boil_off_ceiling`]). The core-powered loss that follows
//! removes at most 0.1 % more (Tang et al. 2024, § 3.5): the ceiling
//! stands for both. It acts round a star without a corona too.
//!
//! **Energy-limited escape** (Owen & Wu 2017, ApJ 847, 29,
//! doi:10.3847/1538-4357/aa890a, section 2.2, eq. 19, after Lecavelier
//! des Etangs 2007 and Erkaev et al. 2007): the paper's rate `Ṁ = η π R³
//! L_HE / (4 π a² G M)`, integrated over the XUV energy per unit area
//! received, `F = ∫ L_HE / (4 π a²) dt`, gives the mass lost
//! `ΔM = η π R³ F / (G M)` ([`energy_limited_loss`]). It runs after the
//! boil-off ("photoevaporation should start right after boil-off around
//! 1 Myr", Tang et al. 2024, § 3.3). The radius valley is opened by both:
//! the boil-off's stripping line already has the valley's slope (`R ∝
//! F^0.081` against `F^0.069`, Berger et al. 2023) but lies under it
//! (1.1 to 1.6 R⊕, "cannot by itself form a pronounced radius gap"), and
//! the XUV escape carves the rest. Nothing here is tuned to the valley's
//! position. The near-ultraviolet escape of hot stars' planets (#701) is
//! not modelled.
//!
//! **Cosmic shoreline** (Zahnle & Catling 2017, ApJ 843, 122,
//! doi:10.3847/1538-4357/aa7846): the bodies with and without air are
//! split by `I_XUV ∝ v_esc⁴`, `I_XUV` the cumulative XUV fluence relative
//! to Earth's. Their line is "drawn by eye" and has no width in the
//! paper, so its position and steepness here are calibrations
//! ([`shoreline_kept`]).
//!
//! The XUV escape and the shoreline are ported from the tag
//! `legacy-final` (`planet/escape.rs` and `planet/atmosphere.rs`
//! l.126-146, 182-185, 288-320), with the sources checked in the study
//! [21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit).
//! The boil-off is new to the rewrite, from the study
//! [30](https://github.com/cyboooooorg/map-generator/wiki/30-core-powered-loss-audit).

use crate::constants::GRAVITATIONAL_CONSTANT;
use crate::units::{
    EarthFlux, EarthMass, Fraction, JoulePerSquareMetre, Kilogram, Metre, MetrePerSecond,
};
use crate::xuv::EARTH_XUV_FLUENCE;

/// Source: efficiency `η` of the XUV photons at removing mass, Owen & Wu
/// 2017, ApJ 847, 29, doi:10.3847/1538-4357/aa890a, section 2.2, after
/// eq. 19: "of order 0.1 for low-mass planets ... For simplicity, we adopt
/// a constant eta = 0.1". It falls for deep potential wells (their
/// section 4.5, eq. 31, `η = 0.1 (v_esc / 15 km/s)^-2`), which their own
/// footnote says is "suitable in a narrow range of parameter space": the
/// constant is kept, and a giant loses next to nothing with it already.
pub const EFFICIENCY: f64 = 0.1;

/// Calibration: share of a planet's mass under which its hydrogen-helium
/// envelope is gone, what is left being a thin atmosphere, not a hydrogen
/// shell. One tenth of the smallest envelope of Owen & Wu 2017's sample
/// (`X_min = 0.01`, section 3.3; the planets that keep an envelope keep
/// "roughly ~1 % in mass", where the smaller ones are bare cores). No
/// paper searched gives a threshold under which an envelope counts as
/// gone (study 21, part 2).
pub const MIN_ENVELOPE: f64 = 1.0e-3;

/// Definition: the number of core masses and of fluxes in Tang et al.
/// 2024's Table 2, a 7 × 7 grid.
const GRID: usize = 7;

/// Source: the core masses of Tang, Fortney & Murray-Clay 2024
/// (arXiv:2410.08577), Table 2, its columns, M⊕.
const BOIL_OFF_CORE_MASSES: [f64; GRID] = [1.5, 2.4, 3.6, 5.5, 8.5, 13.0, 20.0];

/// Source: the bolometric fluxes of Tang et al. 2024, Table 2, its rows,
/// F⊕.
const BOIL_OFF_FLUXES: [f64; GRID] = [1.0, 3.0, 10.0, 30.0, 100.0, 300.0, 1000.0];

/// A cell of Tang et al. 2024's Table 2.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    /// The largest H/He mass fraction kept after the boil-off.
    Kept(f64),
    /// "−": "zero or very slight mass loss", read as [`NO_BOIL_OFF`].
    NoLoss,
    /// "0": the planet keeps nothing, read as [`NOTHING_LEFT`].
    NothingLeft,
}

use Cell::{Kept, NoLoss, NothingLeft};

/// Source: Tang, Fortney & Murray-Clay 2024 (arXiv:2410.08577), Table 2,
/// "Maximum final mass fraction allowed after boil-off": the H/He mass
/// fraction of the planet a core of [`BOIL_OFF_CORE_MASSES`] (columns)
/// keeps under the bolometric flux [`BOIL_OFF_FLUXES`] (rows), whatever
/// it started with below 20 % (§ 3.3), read on the paper's rendered page.
/// The per cent cells are divided by 100. The starred cells (2 × 10⁻⁶,
/// 6 × 10⁻⁶, 4 × 10⁻⁶, no per cent sign) are planets that lost their
/// whole envelope, the value a radiative layer alone; the double-starred
/// ones (above 10 %) were started above the default 10 %.
#[rustfmt::skip]
const BOIL_OFF_CEILINGS: [[Cell; GRID]; GRID] = [
    [Kept(1.18e-2), Kept(3.38e-2), Kept(8.62e-2), NoLoss, NoLoss, NoLoss, NoLoss],
    [Kept(0.49e-2), Kept(1.79e-2), Kept(4.93e-2), Kept(14.64e-2), NoLoss, NoLoss, NoLoss],
    [Kept(0.14e-2), Kept(0.77e-2), Kept(2.56e-2), Kept(8.85e-2), NoLoss, NoLoss, NoLoss],
    [Kept(0.02e-2), Kept(0.25e-2), Kept(1.17e-2), Kept(4.44e-2), Kept(16.75e-2), NoLoss, NoLoss],
    [Kept(2e-6), Kept(0.03e-2), Kept(0.30e-2), Kept(1.67e-2), Kept(7.21e-2), Kept(23.49e-2), NoLoss],
    [NothingLeft, Kept(6e-6), Kept(0.04e-2), Kept(0.53e-2), Kept(3.14e-2), Kept(11.13e-2), NoLoss],
    [NothingLeft, NothingLeft, Kept(4e-6), Kept(0.04e-2), Kept(0.73e-2), Kept(4.04e-2), Kept(13.50e-2)],
];

/// Calibration: a "−" cell of Tang et al. 2024's Table 2, "zero or very
/// slight mass loss", reads as 0.20, the highest envelope drawn below
/// 20 M⊕ (Lopez & Fortney 2014's last envelope column,
/// [`crate::envelope::HIGHEST_TABULATED`]): the ceiling there never cuts
/// a draw. The paper gives no value; none was found in study 30
/// ([planet-atmospheres](../../../docs/science/planet-atmospheres.md#boil-off-and-core-powered-mass-loss-study-30),
/// calibration).
pub const NO_BOIL_OFF: f64 = crate::envelope::HIGHEST_TABULATED.get();

/// Definition: a "0" cell of Tang et al. 2024's Table 2, a planet that
/// keeps nothing, reads as 1e-7 so that its logarithm exists. Every
/// neighbour of a "0" cell is under [`MIN_ENVELOPE`] (the starred cells
/// are 2 to 6 × 10⁻⁶), so the value decides no stripping (study 30).
pub const NOTHING_LEFT: f64 = 1.0e-7;

/// How [`boil_off_ceiling`] reads a point off Tang et al. 2024's grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffGrid {
    /// The end segment of each axis extended in log-log from the grid's
    /// edge.
    Extend,
    /// The value at the nearest edge of the grid.
    Clamp,
}

/// Calibration: off Tang et al. 2024's grid (below 1.5 M⊕, below 1 F⊕,
/// above 20 M⊕ or 1 000 F⊕), the end segment of each axis is extended in
/// log-log from the grid's edge ([`OffGrid::Extend`]), as [`crate::xuv`]
/// extends its saturation anchors. The physics fixes the direction
/// (lighter and hotter planets lose more, Owen & Wu 2016), not the slope;
/// no published value off the grid was found in study 30. Clamping at
/// the edges instead changes none of the 22 reference placements'
/// outcomes; it keeps more envelope on Earth (1.18 % against 0.48 %) and
/// on the hottest Neptunes
/// ([planet-atmospheres](../../../docs/science/planet-atmospheres.md#boil-off-and-core-powered-mass-loss-study-30),
/// calibration).
pub const OFF_GRID: OffGrid = OffGrid::Extend;

/// Definition: mass of water whose hydrogen is one unit of mass of
/// hydrogen, by stoichiometry: H₂O is 18 for 2 of hydrogen.
pub const WATER_PER_HYDROGEN: f64 = 9.0;

/// Calibration: escape velocity at which the cosmic shoreline crosses
/// Earth's XUV fluence. Zahnle & Catling 2017 (ApJ 843, 122,
/// doi:10.3847/1538-4357/aa7846) draw their line "by eye" on their
/// Figures 1 and 2 and print no equation, constant or crossing velocity;
/// the position was not read off the figures. Fitted on the Solar System
/// bodies: Mars keeps a trace (the paper places it at the line), Titan a
/// bar, Venus and Earth their air (study 21, part 7).
pub const SHORELINE_VELOCITY: MetrePerSecond = MetrePerSecond(6_500.0);

/// Source: power of the escape velocity along the cosmic shoreline,
/// `I_XUV ∝ v_esc⁴`: Zahnle & Catling 2017, abstract ("The dividing line
/// goes at I proportional to v_esc^4") and Figure 2.
pub const SHORELINE_EXPONENT: i32 = 4;

/// Calibration: steepness `k` of the share kept across the shoreline,
/// `1 / (1 + x^k)`. Zahnle & Catling 2017 give the line no width (section
/// 10: "we do not know if the shoreline is broad or narrow"). Fitted on
/// the Solar System bodies: Titan, about 0.1 dex on the side of the air,
/// keeps most; Mars, about 0.4 dex on the other side, keeps under 1 %
/// (study 21, part 7).
pub const SHORELINE_STEEPNESS: f64 = 6.0;

/// Calibration: factor on the XUV fluence a body without a dynamo meets:
/// the stellar wind reaches its upper atmosphere and picks up its ions.
/// No factor is published (Zahnle & Catling 2017, section 6, on the
/// solar wind's erosion of Mars, is the nearest read and gives none);
/// kept small enough that Venus, with no dynamo, keeps its air. Tied to
/// issue #344 (study 21, part 7).
pub const UNMAGNETISED_ESCAPE: f64 = 2.0;

/// Mass a body of `radius` and `mass` can lose to the XUV energy per
/// unit area `fluence` it received: `η π R³ F / (G M)` (Owen & Wu 2017,
/// eq. 19, integrated over the age), with `η` [`EFFICIENCY`]. It is not
/// capped by what the body holds; the caller caps it. Takes finite
/// inputs and a positive mass.
pub fn energy_limited_loss(
    radius: Metre,
    mass: Kilogram,
    fluence: JoulePerSquareMetre,
) -> Kilogram {
    Kilogram(
        EFFICIENCY * std::f64::consts::PI * radius.0.powi(3) * fluence.0
            / (GRAVITATIONAL_CONSTANT * mass.0),
    )
}

/// Escape velocity at the surface of a body of `mass` and `radius`:
/// `v = √(2 G M / R)`.
pub fn escape_velocity(mass: Kilogram, radius: Metre) -> MetrePerSecond {
    MetrePerSecond((2.0 * GRAVITATIONAL_CONSTANT * mass.0 / radius.0).sqrt())
}

/// Share of its air a body of surface `escape_velocity` keeps after the
/// XUV energy per unit area `fluence`, on the cosmic shoreline:
///
/// ```text
/// kept = 1 / (1 + (φ s / (v / v₀)⁴)^k)
/// ```
///
/// with `φ` the fluence over [`EARTH_XUV_FLUENCE`], `v₀`
/// [`SHORELINE_VELOCITY`], `k` [`SHORELINE_STEEPNESS`] and `s` 1 for a
/// `magnetised` body, [`UNMAGNETISED_ESCAPE`] for one without a dynamo.
/// Half on the line, nearly all well below it, nearly nothing well above.
/// Every gas is kept in the same share: the shoreline is a bulk rule, not
/// a species-by-species escape. No fluence keeps all the air, whatever
/// the velocity. Takes finite inputs.
pub fn shoreline_kept(
    escape_velocity: MetrePerSecond,
    fluence: JoulePerSquareMetre,
    magnetised: bool,
) -> Fraction {
    let phi = fluence.0.max(0.0) / EARTH_XUV_FLUENCE.0;
    if phi == 0.0 {
        return Fraction::new(1.0).expect("one is a fraction");
    }
    let shield = if magnetised { 1.0 } else { UNMAGNETISED_ESCAPE };
    let line = (escape_velocity.0 / SHORELINE_VELOCITY.0).powi(SHORELINE_EXPONENT);
    let exposure = phi * shield / line;
    let kept = 1.0 / (1.0 + exposure.powf(SHORELINE_STEEPNESS));
    Fraction::new(kept).expect("a share in 0 to 1 for finite inputs")
}

/// The largest hydrogen-helium mass fraction a planet of rocky
/// `core_mass` (its mass without its envelope) keeps after the boil-off,
/// under the bolometric `flux` of its stars: Tang, Fortney & Murray-Clay
/// 2024's Table 2 ([`BOIL_OFF_CEILINGS`]), interpolated bilinearly in
/// `log M_c` and `log F` of `log f`.
///
/// Off the grid (below 1.5 M⊕, below 1 F⊕, above 20 M⊕ or 1 000 F⊕) the
/// end segment of each axis is extended in log-log from the grid's edge
/// ([`OFF_GRID`]); off both axes the two extensions add, without
/// the bilinear cross term, which would give an envelope back to a light
/// core far above 1 000 F⊕. A ceiling read above one is one: a share is at
/// most the whole planet. A core of no mass keeps nothing; a planet under
/// no light keeps everything (the limits of the extension). Takes finite
/// inputs.
///
/// The ceiling stands for the boil-off and the core-powered loss after
/// it, which Tang et al. find removes at most 0.1 % more (§ 3.5): one
/// bolometric law, never two.
pub fn boil_off_ceiling(core_mass: EarthMass, flux: EarthFlux) -> Fraction {
    let share = if core_mass.0 <= 0.0 {
        0.0
    } else if flux.0 <= 0.0 {
        1.0
    } else {
        log_log_read(core_mass.0.ln(), flux.0.ln()).exp().min(1.0)
    };
    Fraction::new(share).expect("a share in 0 to 1 for finite inputs")
}

/// `log f` at `x = log M_c`, `y = log F`: inside the grid, the bilinear
/// interpolation of the cell holding the point; outside, the same cell's
/// formula read at the point clamped to the grid, plus the cell's slope
/// along each axis times the distance off the grid on that axis.
fn log_log_read(x: f64, y: f64) -> f64 {
    let masses = BOIL_OFF_CORE_MASSES.map(f64::ln);
    let fluxes = BOIL_OFF_FLUXES.map(f64::ln);
    let (xc, yc) = (
        x.clamp(masses[0], masses[GRID - 1]),
        y.clamp(fluxes[0], fluxes[GRID - 1]),
    );
    let (i, j) = (segment(&masses, xc), segment(&fluxes, yc));
    let corner = |a: usize, b: usize| cell(BOIL_OFF_CEILINGS[j + b][i + a]).ln();
    let cell_at = |xx: f64, yy: f64| {
        let u = (xx - masses[i]) / (masses[i + 1] - masses[i]);
        let v = (yy - fluxes[j]) / (fluxes[j + 1] - fluxes[j]);
        (1.0 - u) * (1.0 - v) * corner(0, 0)
            + u * (1.0 - v) * corner(1, 0)
            + (1.0 - u) * v * corner(0, 1)
            + u * v * corner(1, 1)
    };
    match OFF_GRID {
        // The cell's formula is linear in each axis: read at `(x, yc)` it
        // is the edge value plus the mass slope times the distance, and
        // likewise in flux. Inside the grid, `(x, y) = (xc, yc)` and this
        // is the bilinear interpolation itself.
        OffGrid::Extend => cell_at(x, yc) + cell_at(xc, y) - cell_at(xc, yc),
        OffGrid::Clamp => cell_at(xc, yc),
    }
}

/// The index `i` of the segment `[grid[i], grid[i + 1]]` holding `x`, a
/// value within the grid.
fn segment(grid: &[f64; GRID], x: f64) -> usize {
    grid[1..GRID - 1].partition_point(|&g| g < x)
}

/// The value a cell of [`BOIL_OFF_CEILINGS`] reads as.
fn cell(c: Cell) -> f64 {
    match c {
        Cell::Kept(f) => f,
        Cell::NoLoss => NO_BOIL_OFF,
        Cell::NothingLeft => NOTHING_LEFT,
    }
}

#[cfg(test)]
#[path = "escape_tests.rs"]
mod tests;
