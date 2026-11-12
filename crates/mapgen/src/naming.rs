//! Scientific names of a world's system, after the IAU's conventions for
//! exoplanets: the star carries a catalogue designation, its planets that
//! designation and a lowercase letter, and moons their planet's name and a
//! Roman numeral.
//!
//! Real letters follow the order of discovery; here every planet of the
//! system is known, so they follow the distance to the star, from `b` (the
//! star itself is `a`). Moons are numbered from their planet outwards, as
//! Galileo's and Saturn's were before their names were given.
//!
//! The catalogue is fictional, the *Map Generator Survey* (`MGS`): its
//! number comes from the world's seed and star, so a system keeps its
//! name, and two worlds of one system (the same seed and star) share it.

/// Catalogue prefix of the stars.
pub const CATALOGUE: &str = "MGS";

/// What naming needs to know of a system: the stored fields of
/// `world.json`.
#[derive(Clone, Debug, Default)]
pub struct Layout<'a> {
    pub seed: u32,
    /// Spectral type of the star (`G2V`).
    pub star: &'a str,
    /// The world's orbit around the star, in AU; for the moon of a giant,
    /// the giant's.
    pub orbit_au: f32,
    /// Orbits of the system's other giants, in AU, in their stored order.
    pub giant_orbits_au: Vec<f32>,
    /// Number of the world's moons.
    pub moons: usize,
    /// When the world is the moon of a giant: its distance to the giant and
    /// those of its siblings, in km, in their stored order.
    pub host: Option<HostLayout>,
}

/// The moons of a giant whose moon the world is.
#[derive(Clone, Debug, Default)]
pub struct HostLayout {
    pub distance_km: f32,
    pub sibling_distances_km: Vec<f32>,
}

/// The names of a system, each a full designation (`MGS 48213 c II`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Names {
    pub star: String,
    /// The world's.
    pub world: String,
    /// The other giants', in the order of `Layout::giant_orbits_au`.
    pub giants: Vec<String>,
    /// The world's moons', from the closest.
    pub moons: Vec<String>,
    /// The giant whose moon the world is.
    pub host: Option<String>,
    /// The giant's other moons', in the order of
    /// `HostLayout::sibling_distances_km`.
    pub siblings: Vec<String>,
}

/// The catalogue designation of the star of `seed` and spectral type
/// `star`: `MGS` and a five-digit number.
pub fn star_name(seed: u32, star: &str) -> String {
    // FNV-1a: stable across versions and platforms, unlike std's hasher.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in seed.to_le_bytes().iter().chain(star.as_bytes()) {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("{CATALOGUE} {}", 10_000 + hash % 90_000)
}

/// The letter of the planet at `rank` from the star (0 is the closest):
/// `b`, `c`, … `z`, then `ba`, `bb`… should a system ever hold more.
pub fn planet_letter(rank: usize) -> String {
    const LETTERS: usize = 25;
    let letter = |k: usize| char::from(b'b' + k as u8);
    if rank < LETTERS {
        letter(rank).to_string()
    } else {
        format!("{}{}", letter(rank / LETTERS - 1), letter(rank % LETTERS))
    }
}

/// `n` in Roman numerals (1 is `I`).
pub fn roman(mut n: usize) -> String {
    let mut s = String::new();
    for (value, digit) in [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ] {
        while n >= value {
            s.push_str(digit);
            n -= value;
        }
    }
    s
}

/// The rank of each of `values` once sorted, smallest first; ties keep
/// their order.
fn ranks(values: &[f32]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut rank = vec![0; values.len()];
    for (r, &i) in order.iter().enumerate() {
        rank[i] = r;
    }
    rank
}

/// The names of the system `layout`.
pub fn names(layout: &Layout) -> Names {
    let star = star_name(layout.seed, layout.star);
    // The world (or its giant) last, so it comes after a giant at the very
    // same orbit.
    let mut orbits = layout.giant_orbits_au.clone();
    orbits.push(layout.orbit_au);
    let rank = ranks(&orbits);
    let planet = |r: usize| format!("{star} {}", planet_letter(r));
    let giants = rank[..layout.giant_orbits_au.len()]
        .iter()
        .map(|&r| planet(r))
        .collect();
    let own = planet(rank[layout.giant_orbits_au.len()]);
    let moon = |of: &str, n: usize| format!("{of} {}", roman(n));
    let (world, host, siblings) = match &layout.host {
        None => (own, None, Vec::new()),
        Some(h) => {
            let mut distances = h.sibling_distances_km.clone();
            distances.push(h.distance_km);
            let rank = ranks(&distances);
            let siblings = rank[..h.sibling_distances_km.len()]
                .iter()
                .map(|&r| moon(&own, r + 1))
                .collect();
            let world = moon(&own, rank[h.sibling_distances_km.len()] + 1);
            (world, Some(own), siblings)
        }
    };
    let moons = (1..=layout.moons).map(|n| moon(&world, n)).collect();
    Names {
        star,
        world,
        giants,
        moons,
        host,
        siblings,
    }
}

/// The names of the system of world `p`, as `world.json` will describe it.
pub fn of_params(p: &crate::world::WorldParams) -> Names {
    let star = crate::star::StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
    names(&Layout {
        seed: p.seed,
        star: &star.kind,
        orbit_au: p.orbit_au,
        giant_orbits_au: crate::giants::GiantSystem::draw(p)
            .map(|s| s.giants.iter().map(|g| g.orbit_au).collect())
            .unwrap_or_default(),
        moons: crate::moons::MoonSystem::draw(p).map_or(0, |s| s.moons.len()),
        host: crate::exomoon::HostGiant::draw(p).map(|h| HostLayout {
            distance_km: h.distance_km,
            sibling_distances_km: h.siblings.iter().map(|s| s.distance_km).collect(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_keep_their_name() {
        assert_eq!(star_name(42, "G2V"), star_name(42, "G2V"));
        assert_ne!(star_name(42, "G2V"), star_name(43, "G2V"));
        assert_ne!(star_name(42, "G2V"), star_name(42, "M4V"));
        let name = star_name(7, "K1V");
        assert!(name.starts_with("MGS "), "{name}");
        assert_eq!(name.len(), "MGS 12345".len(), "{name}");
    }

    #[test]
    fn letters_and_numerals() {
        assert_eq!(planet_letter(0), "b");
        assert_eq!(planet_letter(24), "z");
        assert_eq!(planet_letter(25), "bb");
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(14), "XIV");
        assert_eq!(roman(1999), "MCMXCIX");
    }

    #[test]
    fn planets_are_lettered_from_the_star() {
        let names = names(&Layout {
            seed: 1,
            star: "G2V",
            orbit_au: 1.0,
            giant_orbits_au: vec![5.2, 0.05, 30.0],
            moons: 2,
            host: None,
        });
        let s = &names.star;
        assert_eq!(names.world, format!("{s} c"));
        assert_eq!(
            names.giants,
            [format!("{s} d"), format!("{s} b"), format!("{s} e")]
        );
        assert_eq!(names.moons, [format!("{s} c I"), format!("{s} c II")]);
        assert_eq!(names.host, None);
    }

    #[test]
    fn a_giants_moon_is_numbered_among_its_siblings() {
        let names = names(&Layout {
            seed: 9,
            star: "K1V",
            orbit_au: 5.0,
            giant_orbits_au: vec![12.0],
            moons: 0,
            host: Some(HostLayout {
                distance_km: 670_000.0,
                sibling_distances_km: vec![1_070_000.0, 420_000.0],
            }),
        });
        let s = &names.star;
        assert_eq!(names.host.as_deref(), Some(format!("{s} b").as_str()));
        assert_eq!(names.giants, [format!("{s} c")]);
        assert_eq!(names.world, format!("{s} b II"));
        assert_eq!(names.siblings, [format!("{s} b III"), format!("{s} b I")]);
    }
}
