/// Biome selection: Whittaker-style multi-factor diagram plus planet remapping.
///
/// The pipeline for each tile is:
///  1. [`planet_offsets`]    — climate deltas driven by planet archetype.
///  2. [`choose_biome`]      — altitude/temperature/moisture → base biome.
///  3. [`apply_volcanic`]    — optionally overrides biome with volcanic terrain.
///  4. [`apply_planet_type`] — final remap to planet-exclusive biomes.
use crate::world::{Biome, PlanetType};

// ── Planet climate offsets ────────────────────────────────────────────────────

/// Returns `(Δtemperature, Δmoisture, Δvolcanic_zone)` for the given planet
/// archetype.  The caller clamps the resulting values to their valid ranges.
pub fn planet_offsets(pt: PlanetType) -> (f32, f32, f32) {
    match pt {
        PlanetType::Terran => (0.00, 0.00, 0.00),
        // Scorching hot, bone-dry, heavily volcanic
        PlanetType::Volcanic => (0.45, -0.55, 0.50),
        // Perpetually cold, slightly more frozen precipitation
        PlanetType::Frozen => (-0.55, 0.15, -0.30),
        // Mildly warm, saturated with caustic moisture
        PlanetType::Caustic => (0.10, 0.55, 0.00),
        // Arid lifeless rock, no volcanic activity
        PlanetType::Barren => (0.00, -0.65, -0.40),
    }
}

// ── Top-level biome selector ──────────────────────────────────────────────────

/// Selects the final biome for a tile by running the full pipeline:
/// altitude band → volcanic override → planet remapping.
///
/// - `e`   elevation     in `[-1, 1]`
/// - `m`   moisture      in `[-1, 1]`  (`> 0` is wet)
/// - `t`   temperature   in `[ 0, 1]`  (`0` = polar, `1` = equatorial)
/// - `vz`  volcanic_zone in `[ 0, 1]`  (`0` = inert, `1` = fully volcanic)
/// - `pt`  planet archetype — governs the final biome remapping pass
pub fn choose_biome(e: f32, m: f32, t: f32, vz: f32, pt: PlanetType) -> Biome {
    let base = if e < -0.15 {
        ocean_biome(e)
    } else if e < 0.0 {
        shore_biome(t, m)
    } else if e > 0.7 {
        highland_biome(e, t)
    } else {
        land_biome(t, m)
    };
    let after_volcano = apply_volcanic(base, e, vz);
    apply_planet_type(after_volcano, pt)
}

// ── Altitude bands ────────────────────────────────────────────────────────────

fn ocean_biome(e: f32) -> Biome {
    if e < -0.45 {
        Biome::DeepOcean
    } else {
        Biome::Ocean
    }
}

fn shore_biome(t: f32, m: f32) -> Biome {
    if t < 0.15 {
        Biome::IceCap // frozen shore / pack ice
    } else if m > 0.3 {
        Biome::Wetland // mangroves / marshes
    } else {
        Biome::Beach
    }
}

fn highland_biome(e: f32, t: f32) -> Biome {
    if t < 0.35 || e > 0.88 {
        Biome::Snow
    } else {
        Biome::Mountain
    }
}

// ── Land: temperature zones ───────────────────────────────────────────────────

fn land_biome(t: f32, m: f32) -> Biome {
    if t < 0.15 {
        return Biome::IceCap; // polar
    }
    if t < 0.30 {
        return boreal_biome(m);
    }
    if t < 0.55 {
        return temperate_biome(m);
    }
    tropical_biome(m)
}

fn boreal_biome(m: f32) -> Biome {
    if m > 0.2 { Biome::Taiga } else { Biome::Tundra }
}

fn temperate_biome(m: f32) -> Biome {
    if m < -0.1 {
        Biome::Shrubland
    } else if m > 0.35 {
        Biome::Forest
    } else {
        Biome::Plain
    }
}

fn tropical_biome(m: f32) -> Biome {
    if m < -0.05 {
        Biome::Desert
    } else if m < 0.30 {
        Biome::Savanna
    } else {
        Biome::Jungle
    }
}

// ── Volcanic modifier ─────────────────────────────────────────────────────────

/// Optionally overrides a biome when it sits inside an active volcanic zone.
///
/// - `vz` volcanic_zone in `[0, 1]`: `0` = inert, higher = stronger activity.
/// - `e`  biome elevation, distinguishes caldera from flank from foothill.
///
/// Override ladder (strongest condition wins):
/// - **Volcano**   — summit/caldera : `Mountain|Snow`, `e > 0.80`, `vz > 0.55`
/// - **LavaField** — flanks         : `Mountain|Snow`,             `vz > 0.30`
/// - **AshLand**   — lower slopes   : `Mountain|Snow|Shrubland|Plain|Tundra`, `e > 0.30`, `vz > 0.15`
pub fn apply_volcanic(biome: Biome, e: f32, vz: f32) -> Biome {
    if vz <= 0.0 {
        return biome;
    }
    match biome {
        // Summit / caldera → active vent
        Biome::Mountain | Biome::Snow if e > 0.80 && vz > 0.55 => Biome::Volcano,
        // Volcanic flanks → cooling lava flows
        Biome::Mountain | Biome::Snow if vz > 0.30 => Biome::LavaField,
        // Lower slopes and surrounding terrain → ash wasteland
        Biome::Mountain | Biome::Snow | Biome::Shrubland | Biome::Plain | Biome::Tundra
            if e > 0.30 && vz > 0.15 =>
        {
            Biome::AshLand
        }
        other => other,
    }
}

// ── Planet-type biome remapping ───────────────────────────────────────────────

/// Final pass that converts standard biomes into planet-exclusive ones.
///
/// Called after [`apply_volcanic`] so volcanic modifiers are visible here.
/// [`PlanetType::Terran`] is a no-op; all other archetypes remap some or all biomes.
pub fn apply_planet_type(biome: Biome, pt: PlanetType) -> Biome {
    match pt {
        PlanetType::Terran => biome,

        // ── Volcanic world ────────────────────────────────────────────────────
        // Ocean basins fill with magma; lowlands are scoured to bare rock.
        PlanetType::Volcanic => match biome {
            Biome::DeepOcean | Biome::Ocean => Biome::MagmaSea,
            Biome::Beach | Biome::Wetland => Biome::AshLand,
            Biome::Plain | Biome::Shrubland | Biome::Savanna | Biome::Desert => {
                Biome::ScorchedWaste
            }
            Biome::Forest | Biome::Jungle | Biome::Taiga => Biome::AshLand,
            Biome::IceCap | Biome::Tundra | Biome::Snow | Biome::GlacialPlain => {
                Biome::ScorchedWaste
            }
            other => other, // Mountain, LavaField, AshLand, Volcano — keep as-is
        },

        // ── Frozen world ──────────────────────────────────────────────────────
        // Oceans are sealed under ice; temperate zones become permafrost plains.
        PlanetType::Frozen => match biome {
            Biome::DeepOcean | Biome::Ocean | Biome::MagmaSea => Biome::FrozenOcean,
            Biome::Beach | Biome::Wetland => Biome::IceCap,
            Biome::Plain | Biome::Shrubland => Biome::GlacialPlain,
            Biome::Forest | Biome::Jungle => Biome::Taiga,
            Biome::Savanna | Biome::Desert => Biome::GlacialPlain,
            Biome::LavaField | Biome::AshLand | Biome::ScorchedWaste => Biome::GlacialPlain,
            other => other, // Tundra, IceCap, Taiga, Snow, Mountain — keep as-is
        },

        // ── Caustic world ─────────────────────────────────────────────────────
        // Oceans become acid seas; vegetation zones drown in toxic runoff.
        PlanetType::Caustic => match biome {
            Biome::DeepOcean | Biome::Ocean => Biome::CausticLake,
            Biome::Beach | Biome::Wetland | Biome::Forest | Biome::Jungle | Biome::Taiga => {
                Biome::ToxicSwamp
            }
            Biome::Plain | Biome::Shrubland | Biome::Savanna | Biome::Tundra | Biome::Desert => {
                Biome::AcidFlatland
            }
            Biome::IceCap | Biome::Snow | Biome::GlacialPlain => Biome::AcidFlatland,
            other => other, // Mountain, LavaField, AshLand, Volcano — keep as-is
        },

        // ── Barren world ──────────────────────────────────────────────────────
        // No liquid water; all life extinct; only rock and dust remain.
        PlanetType::Barren => match biome {
            Biome::DeepOcean | Biome::Ocean | Biome::CausticLake | Biome::FrozenOcean => {
                Biome::RockyWaste
            }
            Biome::Beach | Biome::Wetland | Biome::ToxicSwamp => Biome::RockyWaste,
            Biome::Plain
            | Biome::Shrubland
            | Biome::Savanna
            | Biome::Desert
            | Biome::Tundra
            | Biome::IceCap
            | Biome::GlacialPlain
            | Biome::AcidFlatland => Biome::DustPlain,
            Biome::Forest | Biome::Jungle | Biome::Taiga => Biome::DustPlain,
            Biome::LavaField | Biome::AshLand | Biome::ScorchedWaste | Biome::Snow => {
                Biome::RockyWaste
            }
            other => other, // Mountain, Volcano — keep as-is
        },
    }
}
