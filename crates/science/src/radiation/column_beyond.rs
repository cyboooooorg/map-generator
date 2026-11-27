//! What a column sends out past the longwave table's top,
//! [`SPECTRUM_TOP_CM`] (5000 cm⁻¹, 2 µm): the tail band (`spectrum::TAIL`) of
//! [`super::radiate`] (study 41, issue #775).
//!
//! - **The near infrared, 5000 to 14 364 cm⁻¹ (2 to 0.7 µm)**, on the
//!   bins of the shortwave table (`shortwave.bin`, [`SHORTWAVE`]): the
//!   same HITRAN2024 lines of CO₂ and water and MT_CKD 4.3's continua the
//!   column's light crosses (`OPACITY.md`, "Shortwave table"), the bin
//!   that straddles 5000 cm⁻¹ counted from 5000 cm⁻¹ only. The air emits
//!   and absorbs there as it does in the longwave bins: each layer sends
//!   out its mean Planck flux times what it takes of the transmission to
//!   space; within a bin CO₂'s and water's g-points overlap at random
//!   (Lacis & Oinas 1991, *JGR* 96, 9027), the continua with them; a
//!   layer's depth is the logarithmic mean of the rates at its two levels
//!   times its span, and the air above the top is uniform (half the top's
//!   rate). The air does not scatter there.
//! - **Past 14 364 cm⁻¹ (0.7 µm)**, where the shortwave table holds no
//!   absorption, the air only scatters: each band of the star's light
//!   ([`super::super::starlight`], 0.2 to 0.7 µm) lets the ground's
//!   Planck flux out through the Rayleigh depth of the air above it,
//!   `τ_R(550 nm) (550 nm / λ)⁴` ([`rayleigh_depth_550`],
//!   [`rayleigh_scale`]), with the transmission of a conservative
//!   two-stream layer, `1 / (1 + (D/2) τ)` (Meador & Weaver 1980,
//!   [`super::super::shortwave::layer`] with no absorption). A scattering
//!   air emits nothing. Past 0.2 µm (50 000 cm⁻¹) the light takes the
//!   bluest band's transmission.
//!
//! The two parts and the longwave bins split the black body at their
//! edges exactly ([`spectrum::planck_over`]), so that nothing is counted
//! twice and a transparent air sends out `σT⁴`. The parts a column's
//! warmest level cannot send out ([`NEGLIGIBLE`] of its `σT⁴`) are left
//! out of the sum and taken as clear, as [`super::summed_bins`] takes the
//! longwave bins. Water clouds cover the sky here as they do in the
//! longwave bins ([`super::radiate`]): a grey layer at their top.
//!
//! The shortwave table's grid stops at 900 K and 100 bar, as the
//! longwave table's does: a hotter level reads its 900 K lines and a
//! deeper one scales them as `p` (`OPACITY.md`, Grid). Past 4300 K the
//! surface is outside every published steam profile (Turbet et al. 2020),
//! which the climate marks (`science::steam::PUBLISHED_SURFACE_TOP`).

use std::sync::LazyLock;

use super::super::rayleigh::rayleigh_depth_550;
use super::super::shortwave::{LevelAbsorption, SHORTWAVE};
use super::super::spectrum::{self, DIFFUSIVITY, G, SPECTRUM_TOP_CM};
use super::super::starlight::{self, band_edge_m, rayleigh_scale};
use super::{Column, NEGLIGIBLE, OPAQUE};
use crate::constants::STEFAN_BOLTZMANN;

#[cfg(test)]
#[path = "column_beyond_tests.rs"]
mod tests;

/// Definition: wavenumber, cm⁻¹, of a wavelength in metres.
fn wavenumber_cm(wavelength_m: f64) -> f64 {
    1e-2 / wavelength_m
}

/// The edges of the region past [`SPECTRUM_TOP_CM`], ascending, cm⁻¹:
/// the near infrared's (5000 cm⁻¹, then the shortwave table's edges
/// above it) and the visible's (the star's bands from 0.7 µm to 0.2 µm).
struct Edges {
    edges: Vec<f64>,
    /// The first bin of the shortwave table counted.
    first_bin: usize,
    /// The index of the edge at 14 364 cm⁻¹, where the near infrared
    /// ends.
    visible_from: usize,
}

/// Definition: the edges of every column's region past
/// [`SPECTRUM_TOP_CM`] ([`Edges`]), from the shortwave table's and the
/// star's bands, taken once.
static EDGES: LazyLock<Edges> = LazyLock::new(|| {
    let table = &*SHORTWAVE;
    let first_bin = table
        .edges_cm
        .windows(2)
        .position(|e| e[1] > SPECTRUM_TOP_CM)
        .expect("the shortwave table reaches past 5000 cm⁻¹");
    let mut edges = vec![SPECTRUM_TOP_CM];
    edges.extend_from_slice(&table.edges_cm[first_bin + 1..]);
    let visible_from = edges.len() - 1;
    // The star's bands bluer than the table, reddest first.
    let visible = starlight::BANDS - table.bins;
    edges.extend(
        (0..visible)
            .rev()
            .map(|band| wavenumber_cm(band_edge_m(band))),
    );
    Edges {
        edges,
        first_bin,
        visible_from,
    }
});

/// Workspace of [`beyond`]: the absorption at a layer's two levels, the
/// depths, Planck fluxes and transmissions of each part. Kept in the
/// caller's `ColumnCache` with the longwave's (`super::Scratch`), so that
/// a column's sum allocates nothing once its vectors have grown.
pub(super) struct Workspace {
    rate: LevelAbsorption,
    rate_below: LevelAbsorption,
    co2: Vec<f64>,
    water: Vec<f64>,
    smooth: Vec<f64>,
    planck: Vec<f64>,
    planck_below: Vec<f64>,
    ground: Vec<f64>,
    seen: Vec<f64>,
    out: Vec<f64>,
    seen_above: Vec<f64>,
    out_above: Vec<f64>,
}

impl Workspace {
    pub(super) fn new() -> Self {
        Self {
            rate: LevelAbsorption::new(0..0),
            rate_below: LevelAbsorption::new(0..0),
            co2: Vec::new(),
            water: Vec::new(),
            smooth: Vec::new(),
            planck: Vec::new(),
            planck_below: Vec::new(),
            ground: Vec::new(),
            seen: Vec::new(),
            out: Vec::new(),
            seen_above: Vec::new(),
            out_above: Vec::new(),
        }
    }
}

/// What a column sends out past [`SPECTRUM_TOP_CM`], W/m².
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Sent {
    /// All of it, its air, its clouds and its ground ([`beyond`]).
    pub(super) total: f64,
    /// The ground's own light that reaches space through the clear
    /// column (no cloud): the near infrared's transmitted to space from
    /// the ground (none when the column is opaque above it), the
    /// visible's through the Rayleigh depth, and the parts left out of
    /// the sum, clear.
    pub(super) ground: f64,
}

/// What `column` sends out past [`SPECTRUM_TOP_CM`], W/m² (the module's
/// documentation), its clouds and its ground included, in the
/// workspace `ws`.
pub(super) fn beyond(column: &Column, ws: &mut Workspace) -> f64 {
    sent(column, ws).total
}

/// [`beyond`], with the ground's share of it.
pub(super) fn sent(column: &Column, ws: &mut Workspace) -> Sent {
    let Edges {
        edges,
        first_bin,
        visible_from,
    } = &*EDGES;
    let (first_bin, visible_from) = (*first_bin, *visible_from);
    let len = edges.len();
    for v in [&mut ws.planck, &mut ws.planck_below, &mut ws.ground] {
        v.resize(len, 0.0);
    }
    let warmest = column.temperature.iter().copied().fold(0.0, f64::max);
    // The intervals summed: all but the last ones whose flux at the
    // warmest level, with the rest past them, is negligible.
    spectrum::planck_over(warmest, edges, &mut ws.planck);
    let floor = NEGLIGIBLE * STEFAN_BOLTZMANN * warmest.powi(4);
    let mut summed = len;
    let mut rest = 0.0;
    while summed > 0 && rest + ws.planck[summed - 1] <= floor {
        rest += ws.planck[summed - 1];
        summed -= 1;
    }
    spectrum::planck_over(ground_k(column), edges, &mut ws.ground);
    let left_out: f64 = ws.ground[summed..]
        .iter()
        .map(|b| column.ground_sent(*b, 1.0))
        .sum();
    let nir = summed.min(visible_from);
    let (air, air_cloud, nir_ground) = near_infrared(column, &edges[..=nir], first_bin, ws);
    let (light, light_cloud) = visible(column, edges, visible_from, summed, ws);
    let blend = |clear: f64, sent: Option<f64>| match (column.cloud, sent) {
        (Some(cloud), Some(sent)) => clear - cloud.weight * (clear - sent),
        _ => clear,
    };
    Sent {
        total: blend(air, air_cloud) + blend(light, light_cloud) + left_out,
        ground: nir_ground + light + left_out,
    }
}

/// The temperature, K, of `column`'s ground.
fn ground_k(column: &Column) -> f64 {
    column
        .ground
        .map_or(column.temperature[0], |g| g.temperature_k)
}

/// The near infrared between the ascending `edges` (from `first_bin` of
/// the shortwave table up; the last edge closes the last bin summed),
/// summed from the top down as [`super::radiate`] sums the longwave bins:
/// the clear column's radiation, W/m², what its cloud top sends out
/// (`above_c + seen_c B_c`, each linear in `ln p` across its layer), when
/// it has a cloud and the sum passed it, and the ground's light in it
/// that reaches space (none when the sum stops on an opaque column).
fn near_infrared(
    column: &Column,
    edges: &[f64],
    first_bin: usize,
    ws: &mut Workspace,
) -> (f64, Option<f64>, f64) {
    let parts = edges.len() - 1;
    if parts == 0 {
        return (0.0, None, 0.0);
    }
    let table = &*SHORTWAVE;
    let w = table.weights;
    let n = column.top();
    let bins = first_bin..first_bin + parts;
    ws.rate.resize(bins.clone());
    ws.rate_below.resize(bins);
    // Each bin's slant depth to space per g-point of CO₂ and of water,
    // and of the continua; its transmission to space and its radiation.
    for v in [&mut ws.co2, &mut ws.water] {
        v.resize(parts * G, 0.0);
    }
    for v in [&mut ws.smooth, &mut ws.seen, &mut ws.out] {
        v.resize(parts, 0.0);
    }
    let edges_len = edges.len();
    let Workspace {
        rate,
        rate_below,
        co2,
        water,
        smooth,
        planck,
        planck_below,
        ground,
        seen,
        out,
        seen_above,
        out_above,
    } = ws;
    let (mut planck, mut planck_below) = (&mut planck[..edges_len], &mut planck_below[..edges_len]);
    let through = |co2: &[f64], water: &[f64], smooth: &[f64], b: usize| {
        let gas = |depth: &[f64]| -> f64 {
            w.iter()
                .zip(&depth[b * G..(b + 1) * G])
                .map(|(w, d)| w * (-d).exp())
                .sum()
        };
        gas(co2) * gas(water) * (-smooth[b]).exp()
    };
    // The uniform air above the top: half the top's rate.
    rate.fill(table, &column.level(n));
    spectrum::planck_over(column.temperature[n], edges, planck);
    for (depth, rates) in [(&mut *co2, &rate.co2), (&mut *water, &rate.water)] {
        for (d, r) in depth.iter_mut().zip(rates) {
            *d = DIFFUSIVITY * 0.5 * f64::from(*r);
        }
    }
    for (d, r) in smooth.iter_mut().zip(&rate.smooth) {
        *d = DIFFUSIVITY * spectrum::smooth_above(f64::from(*r));
    }
    for b in 0..parts {
        seen[b] = through(co2, water, smooth, b);
        out[b] = planck[b] * (1.0 - seen[b]);
    }
    let mut cloud_top = None;
    for i in (0..n).rev() {
        let at_cloud = column.cloud.filter(|c| c.layer == i);
        if at_cloud.is_some() {
            out_above.clone_from(out);
            seen_above.clone_from(seen);
        }
        rate_below.fill(table, &column.level(i));
        spectrum::planck_over(column.temperature[i], edges, planck_below);
        let slant = DIFFUSIVITY * column.span(i);
        let deepen = |depth: &mut [f64], top: &[f32], bottom: &[f32]| {
            for ((d, a), b) in depth.iter_mut().zip(top).zip(bottom) {
                *d += slant * spectrum::log_mean(f64::from(*a), f64::from(*b));
            }
        };
        deepen(co2, &rate.co2, &rate_below.co2);
        deepen(water, &rate.water, &rate_below.water);
        deepen(smooth, &rate.smooth, &rate_below.smooth);
        let mut open = false;
        for b in 0..parts {
            let below = through(co2, water, smooth, b);
            out[b] += 0.5 * (planck[b] + planck_below[b]) * (seen[b] - below);
            seen[b] = below;
            open |= below > OPAQUE;
        }
        if let Some(cloud) = at_cloud {
            let mid = |below: f64, above: f64| below + cloud.up * (above - below);
            let sent = (0..parts)
                .map(|b| {
                    mid(out[b], out_above[b])
                        + mid(seen[b], seen_above[b]) * mid(planck_below[b], planck[b])
                })
                .sum();
            cloud_top = Some(sent);
        }
        std::mem::swap(rate, rate_below);
        std::mem::swap(&mut planck, &mut planck_below);
        if !open {
            return (out.iter().sum(), cloud_top, 0.0);
        }
    }
    // `ground` holds the ground's Planck flux on every edge (`sent`).
    let from_ground: f64 = (0..parts)
        .map(|b| column.ground_sent(ground[b], seen[b]))
        .sum();
    (
        out.iter().sum::<f64>() + from_ground,
        cloud_top,
        from_ground,
    )
}

/// The scattering part, the intervals `from..summed` of `edges` (the last
/// interval the rest past 0.2 µm): the ground's Planck flux (`ws.ground`)
/// through the Rayleigh depth of the whole column, and, under a cloud,
/// its top's (the Planck flux linear in `ln p` across its layer, as
/// [`super::radiate`] takes it) through the air above it, the share of
/// the surface pressure over the top.
fn visible(
    column: &Column,
    edges: &[f64],
    from: usize,
    summed: usize,
    ws: &mut Workspace,
) -> (f64, Option<f64>) {
    if summed <= from {
        return (0.0, None);
    }
    let surface = column.levels[0];
    let tau = rayleigh_depth_550(&surface, column.gravity_m_s2);
    let last = edges.len() - 1;
    // Interval `j` is the star's band `last − j − 1` (bluest first); the
    // rest past 0.2 µm takes band 0's.
    let through = |j: usize, mass: f64| {
        let scale = rayleigh_scale((last - j).saturating_sub(1));
        1.0 / (1.0 + 0.5 * DIFFUSIVITY * tau * scale * mass)
    };
    let clear = (from..summed)
        .map(|j| column.ground_sent(ws.ground[j], through(j, 1.0)))
        .sum();
    let cloudy = column.cloud.map(|cloud| {
        let i = cloud.layer;
        let p_top = column.levels[i].total() * (-cloud.up * column.span(i)).exp();
        let mass = p_top / surface.total();
        let (below, above) = (&mut ws.planck, &mut ws.planck_below);
        spectrum::planck_over(column.temperature[i], edges, below);
        spectrum::planck_over(column.temperature[i + 1], edges, above);
        (from..summed)
            .map(|j| (below[j] + cloud.up * (above[j] - below[j])) * through(j, mass))
            .sum()
    });
    (clear, cloudy)
}
