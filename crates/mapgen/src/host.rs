//! The kinds of object a planet can orbit, and the notation that names
//! them.
//!
//! Only main-sequence dwarfs are modelled today ([`HostKind::Dwarf`]); the
//! other kinds are recognised by the parser so that a request for one is
//! refused with the issue that tracks it, rather than as a typo.

use crate::star::{CLASSES, StarValues, TYPE_RANGE, values};
use serde::{Deserialize, Serialize};

/// Astronomical unit, km (IAU 2012).
const AU_KM: f64 = 1.495_978_707e8;

/// What a planet's host is, by the physics that sets its light and its
/// surroundings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    /// A main-sequence star (luminosity class V), the only kind modelled.
    Dwarf,
    /// A star leaving the main sequence (luminosity class IV).
    Subgiant,
    /// A giant or supergiant (luminosity classes I to III).
    Giant,
    /// The degenerate core of a dead star of low or medium mass (D).
    WhiteDwarf,
    /// A pulsar or other neutron star (PSR).
    NeutronStar,
    /// A neutron star with an extreme magnetic field (MAG).
    Magnetar,
    /// A substellar object, spectral classes L, T and Y.
    BrownDwarf,
    /// A stellar-mass black hole (BH).
    BlackHole,
    /// A massive star stripped of its hydrogen (WR, WN, WC, WO).
    WolfRayet,
    /// A hypothetical star powered by a black hole inside it (QS).
    QuasiStar,
}

impl HostKind {
    /// Every kind, in the order of the enum.
    pub const ALL: [HostKind; 10] = [
        HostKind::Dwarf,
        HostKind::Subgiant,
        HostKind::Giant,
        HostKind::WhiteDwarf,
        HostKind::NeutronStar,
        HostKind::Magnetar,
        HostKind::BrownDwarf,
        HostKind::BlackHole,
        HostKind::WolfRayet,
        HostKind::QuasiStar,
    ];

    /// Whether the generator models this kind of host.
    pub fn available(self) -> bool {
        self == HostKind::Dwarf
    }

    /// The backlog issue that tracks modelling this kind, `None` for the
    /// kind already modelled.
    pub fn issue(self) -> Option<u32> {
        match self {
            HostKind::Dwarf => None,
            HostKind::Subgiant | HostKind::Giant => Some(576),
            HostKind::WhiteDwarf => Some(577),
            HostKind::NeutronStar => Some(578),
            HostKind::Magnetar => Some(579),
            HostKind::BrownDwarf => Some(580),
            HostKind::BlackHole => Some(581),
            HostKind::WolfRayet => Some(582),
            HostKind::QuasiStar => Some(583),
        }
    }

    /// Why a request for this kind is refused while it is not modelled,
    /// with the issue that tracks it.
    pub fn refusal(self) -> String {
        format!(
            "{} are not modelled yet (see #{})",
            self.name(),
            self.issue().unwrap_or_default()
        )
    }

    /// The plural noun of the kind, for messages ("white dwarfs are not
    /// modelled yet").
    pub fn name(self) -> &'static str {
        match self {
            HostKind::Dwarf => "dwarfs",
            HostKind::Subgiant => "subgiants",
            HostKind::Giant => "giants",
            HostKind::WhiteDwarf => "white dwarfs",
            HostKind::NeutronStar => "neutron stars",
            HostKind::Magnetar => "magnetars",
            HostKind::BrownDwarf => "brown dwarfs",
            HostKind::BlackHole => "black holes",
            HostKind::WolfRayet => "Wolf-Rayet stars",
            HostKind::QuasiStar => "quasi-stars",
        }
    }

    /// Whether the photosphere has a convective envelope, so that it can
    /// carry a dynamo, spots and granulation. A degenerate or hydrogen-free
    /// surface has none.
    pub fn has_convective_photosphere(self) -> bool {
        matches!(
            self,
            HostKind::Dwarf | HostKind::Subgiant | HostKind::Giant | HostKind::BrownDwarf
        )
    }
}

/// The relative share of each kind among the hosts of a galaxy, from the
/// study of host kinds (section 2.1). Each kind's issue confirms its share
/// before enabling it; a kind not [`HostKind::available`] is never drawn.
pub const HOST_KIND_SHARES: [(HostKind, f64); 10] = [
    // Main-sequence stars: the bulk of the stars (study, section 2.1).
    (HostKind::Dwarf, 0.93),
    // Mamajek: white dwarfs among the stars of the solar neighbourhood.
    (HostKind::WhiteDwarf, 0.059),
    // Mamajek: evolved stars 1.1 %, of which red giants 0.33 %.
    (HostKind::Subgiant, 0.0077),
    (HostKind::Giant, 0.0033),
    // Kirkpatrick 2021, a lower bound.
    (HostKind::BrownDwarf, 0.16),
    // Order of one in a thousand (study, estimate).
    (HostKind::NeutronStar, 1e-3),
    // Order of a hundredth of the neutron stars (study, estimate).
    (HostKind::Magnetar, 1e-5),
    // Olejak 2020: about 1.3e8 against about 1e11 stars (study, estimate).
    (HostKind::BlackHole, 1e-3),
    // Rosslowe & Crowther 2015.
    (HostKind::WolfRayet, 1e-8),
    // Hypothetical: left to #583.
    (HostKind::QuasiStar, 0.0),
];

/// The kind of host of the system `system_seed`, from its own stream.
///
/// While only dwarfs are modelled this is always [`HostKind::Dwarf`] and
/// no rng is seeded. Once other kinds are available, a dwarf is drawn with
/// the share of the dwarfs and the others with theirs.
pub fn draw_host_kind(system_seed: u32) -> HostKind {
    use rand::{RngExt, SeedableRng, rngs::StdRng};
    let others: f64 = HOST_KIND_SHARES
        .iter()
        .filter(|(k, _)| *k != HostKind::Dwarf && k.available())
        .map(|(_, s)| s)
        .sum();
    if others <= 0.0 {
        return HostKind::Dwarf;
    }
    let mut rng = StdRng::seed_from_u64(u64::from(crate::noise::stream_seed(
        system_seed,
        crate::noise::stream::HOST_KIND,
    )));
    let total: f64 = HOST_KIND_SHARES
        .iter()
        .filter(|(k, _)| k.available())
        .map(|(_, s)| s)
        .sum();
    let mut x = rng.random::<f64>() * total;
    for (kind, share) in HOST_KIND_SHARES.iter().filter(|(k, _)| k.available()) {
        if x < *share {
            return *kind;
        }
        x -= share;
    }
    HostKind::Dwarf
}

/// A parsed spectral type: the kind of host and its position along that
/// kind's own sequence (for dwarfs, `class × 10 + subtype`, O = 0 … M = 6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostType {
    pub kind: HostKind,
    pub code: f32,
}

const FIRST_ERROR: &str = "starts with one of O B A F G K M";
const SUBTYPE_ERROR: &str = "the subtype is a number from 0 to 9.5";

/// Reads a spectral type of any kind of host (see the grammar in the
/// design of host kinds). Grammar only: whether the kind is modelled is for
/// the caller to check with [`HostKind::available`]. Case-insensitive.
pub fn parse_host(text: &str) -> Result<HostType, String> {
    let text = text.trim();
    let fail = |why: &str| format!("spectral type {text:?}: {why}");
    let upper = text.to_ascii_uppercase();
    let host = |kind, code| Ok(HostType { kind, code });

    match upper.as_str() {
        "PSR" => return host(HostKind::NeutronStar, 0.0),
        "MAG" => return host(HostKind::Magnetar, 0.0),
        "BH" => return host(HostKind::BlackHole, 0.0),
        "WR" => return host(HostKind::WolfRayet, 0.0),
        "QS" => return host(HostKind::QuasiStar, 0.0),
        _ => {}
    }
    if let Some(n) = ["WN", "WC", "WO"]
        .iter()
        .find_map(|p| upper.strip_prefix(p))
    {
        let n = n
            .parse::<f32>()
            .ok()
            .filter(|n| (0.0..10.0).contains(n))
            .ok_or_else(|| fail("a Wolf-Rayet subtype is a number from 0 to 9.5"))?;
        return host(HostKind::WolfRayet, n);
    }
    if let Some(rest) = upper.strip_prefix('D') {
        let mut chars = rest.chars();
        if !chars.next().is_some_and(|c| "ABOQZC".contains(c)) {
            return Err(fail("a white dwarf is D followed by one of A B O Q Z C"));
        }
        let n = chars.as_str();
        let n = if n.is_empty() {
            0.0
        } else {
            n.parse::<f32>()
                .ok()
                .filter(|n| *n >= 0.0)
                .ok_or_else(|| fail("a white dwarf's temperature index is a number"))?
        };
        return host(HostKind::WhiteDwarf, n);
    }

    let mut chars = upper.chars();
    let letter = chars.next();
    let rest = chars.as_str();
    // The subtype is the leading number; what follows is the luminosity
    // class.
    let number_len = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    let (number, suffix) = rest.split_at(number_len);
    let subtype = |default: f32| -> Result<f32, String> {
        if number.is_empty() {
            Ok(default)
        } else {
            number
                .parse::<f32>()
                .ok()
                .filter(|s| (0.0..10.0).contains(s))
                .ok_or_else(|| fail(SUBTYPE_ERROR))
        }
    };

    if let Some(base) = letter.and_then(|c| ["LTY"].iter().find_map(|s| s.find(c))) {
        if !suffix.is_empty() {
            return Err(fail("a brown dwarf is L, T or Y and a subtype"));
        }
        return host(
            HostKind::BrownDwarf,
            70.0 + 10.0 * base as f32 + subtype(5.0)?,
        );
    }

    let class = letter
        .and_then(|c| CLASSES.iter().position(|&k| k == c))
        .ok_or_else(|| fail(FIRST_ERROR))?;
    let kind = match suffix {
        "" | "V" => HostKind::Dwarf,
        "IV" => HostKind::Subgiant,
        "I" | "IA" | "IB" | "IAB" | "II" | "III" => HostKind::Giant,
        _ => {
            return Err(fail(
                "the luminosity class is V, IV, III, II, I, Ia, Ib or Iab",
            ));
        }
    };
    let code = class as f32 * 10.0 + subtype(5.0)?;
    if kind == HostKind::Dwarf && !(TYPE_RANGE.0..=TYPE_RANGE.1).contains(&code) {
        return Err(fail("main-sequence dwarfs go from O3 to M9.5"));
    }
    host(kind, code)
}

/// The physical state of a host of `kind` and `mass` (solar masses): what
/// the rest of the generator reads of a star. `age_gyr` is unused by
/// dwarfs, whose light does not change along the model's span; kinds that
/// evolve will read it. Refused for a kind not modelled yet and for a
/// dwarf mass outside the sequence (about 0.08 to 90 solar masses).
pub fn host_state(kind: HostKind, mass: f32, _age_gyr: f32) -> Result<StarValues, String> {
    if !kind.available() {
        return Err(kind.refusal());
    }
    dwarf_code_for_mass(mass)
        .map(values)
        .ok_or_else(|| format!("no main-sequence dwarf of {mass} solar masses"))
}

/// The spectral code of the main-sequence dwarf of `mass` (solar masses),
/// the inverse of [`values`]; `None` outside the sequence.
pub fn dwarf_code_for_mass(mass: f32) -> Option<f32> {
    // The mass falls as the code rises: `lo` is the heaviest end.
    let (mut lo, mut hi) = TYPE_RANGE;
    if !(values(hi).mass..=values(lo).mass).contains(&mass) {
        return None;
    }
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if values(mid).mass > mass {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// Nearest a planet is placed to any host, AU: the inner edge of a system
/// round a cool dwarf.
pub const MIN_INNER_EDGE_AU: f64 = 0.02;
/// Bulk density, g/cm³, of the rocky planet whose Roche limit sets the
/// edge: Earth's.
pub const ROCKY_DENSITY: f64 = 5.5;
/// Host radii inside which no planet is placed: the closest planets known
/// orbit at 2 to 3 stellar radii (WASP-12 b, about 3).
pub const STELLAR_RADII_EDGE: f64 = 2.0;
/// Solar radius, AU (695 700 km over the IAU astronomical unit).
const SOLAR_RADIUS_AU: f64 = 0.004_650_47;
/// Solar radius, km.
const SOLAR_RADIUS_KM: f64 = 695_700.0;
/// Mean density of the Sun, g/cm³.
const SOLAR_DENSITY: f64 = 1.41;

/// The inner edge of a system round `star`, AU: the largest of
/// [`MIN_INNER_EDGE_AU`], the fluid Roche limit of a rocky planet
/// ([`ROCKY_DENSITY`]) and [`STELLAR_RADII_EDGE`] stellar radii. A fixed
/// edge would put planets inside an O or hot B star (an O3V is 0.06 AU in
/// radius), while the closest planets known orbit at 2 to 3 radii of
/// their star. Dwarfs from A1 to M keep 0.02 AU, so their systems do not
/// change; O to A0 dwarfs (radius above 2.15 R☉) get the larger edge.
pub fn inner_edge_au(star: &StarValues) -> f64 {
    let radius = f64::from(star.radius);
    let density = SOLAR_DENSITY * f64::from(star.mass) / radius.powi(3);
    let roche_au = crate::tides::roche_km(radius * SOLAR_RADIUS_KM, density, ROCKY_DENSITY) / AU_KM;
    MIN_INNER_EDGE_AU
        .max(roche_au)
        .max(STELLAR_RADII_EDGE * radius * SOLAR_RADIUS_AU)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_kind_but_dwarfs_is_drawn_yet() {
        for seed in 0..2_000 {
            assert_eq!(draw_host_kind(seed), HostKind::Dwarf);
        }
    }

    #[test]
    fn shares_cover_every_kind_once() {
        let mut kinds: Vec<_> = HOST_KIND_SHARES.iter().map(|(k, _)| *k).collect();
        kinds.dedup();
        assert_eq!(kinds.len(), HostKind::ALL.len());
        assert!(HOST_KIND_SHARES.iter().all(|(_, s)| *s >= 0.0));
    }

    #[test]
    fn dwarf_state_inverts_the_sequence() {
        for code in [3.0, 15.0, 42.0, 55.5, 64.5, 69.5] {
            let v = values(code);
            let s = host_state(HostKind::Dwarf, v.mass, 4.6).unwrap();
            assert!((s.teff_k - v.teff_k).abs() / v.teff_k < 1e-3, "{code}");
            assert!(
                (s.luminosity - v.luminosity).abs() / v.luminosity < 1e-3,
                "{code}"
            );
        }
        assert!(host_state(HostKind::Dwarf, 500.0, 4.6).is_err());
        assert!(
            host_state(HostKind::WhiteDwarf, 0.6, 4.6)
                .unwrap_err()
                .contains("#577")
        );
    }

    #[test]
    fn dwarf_notations_keep_their_codes() {
        for (t, c) in [
            ("G2V", 42.0),
            ("g2", 42.0),
            ("m4.5", 64.5),
            ("K", 55.0),
            ("M", 65.0),
            ("B", 15.0),
        ] {
            assert_eq!(
                parse_host(t).unwrap(),
                HostType {
                    kind: HostKind::Dwarf,
                    code: c
                },
                "{t}"
            );
        }
    }

    #[test]
    fn other_kinds_are_recognised() {
        use HostKind::*;
        for (t, k) in [
            ("G8IV", Subgiant),
            ("K3III", Giant),
            ("M2Iab", Giant),
            ("B1Ia", Giant),
            ("DA", WhiteDwarf),
            ("DA2.4", WhiteDwarf),
            ("DZ", WhiteDwarf),
            ("L5", BrownDwarf),
            ("T7", BrownDwarf),
            ("Y0", BrownDwarf),
            ("PSR", NeutronStar),
            ("MAG", Magnetar),
            ("BH", BlackHole),
            ("WR", WolfRayet),
            ("WN7", WolfRayet),
            ("QS", QuasiStar),
        ] {
            assert_eq!(parse_host(t).unwrap().kind, k, "{t}");
        }
    }

    #[test]
    fn malformed_types_are_refused() {
        for t in ["X2", "G12", "O1", "D", "L12", "G2VI", ""] {
            assert!(parse_host(t).is_err(), "{t}");
        }
    }

    #[test]
    fn only_dwarfs_are_available() {
        for k in HostKind::ALL {
            assert_eq!(k.available(), k == HostKind::Dwarf);
            assert_eq!(k.issue().is_none(), k == HostKind::Dwarf);
        }
    }

    #[test]
    fn cool_dwarfs_keep_the_old_edge() {
        for code in [25.0, 30.0, 42.0, 69.5] {
            assert_eq!(inner_edge_au(&values(code)), MIN_INNER_EDGE_AU, "{code}");
        }
    }

    /// The 2 radii term still exceeds 0.02 AU for the B7 to A0 dwarfs
    /// (R > 2.15 R☉): their edge is the stellar one, below 0.03 AU.
    #[test]
    fn late_b_dwarfs_get_a_slightly_wider_edge() {
        let edge = inner_edge_au(&values(17.0));
        assert!(edge > MIN_INNER_EDGE_AU && edge < 0.03, "{edge}");
    }

    #[test]
    fn hot_stars_push_the_edge_outside_themselves() {
        let o3 = values(3.0);
        let r_au = f64::from(o3.radius) * 0.004_650_47;
        assert!(inner_edge_au(&o3) >= 2.0 * r_au - 1e-9);
        assert!(inner_edge_au(&o3) > 0.1); // exercises the hot-Jupiter clamp
    }
}
