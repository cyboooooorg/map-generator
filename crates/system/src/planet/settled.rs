//! The planet chained from its draw to its climate: [`bare_with`]'s
//! ground and air, then the climate the solver settles on them
//! ([`climate::branches::solve`]) recorded in the cause trace
//! ([`climate::record::record`]), and the reducing-air fallback that reads
//! it. A separate step, so that `bare` and `bare_with` keep their cost and
//! their tests do not run the climate. Ported from
//! `legacy-final:crates/mapgen/src/planet/mod.rs` (`bare`, l.880-893).
//!
//! Not wired into the generation chain yet (2d-4). Not in it: `boils`
//! and the steam envelope (2d-3b), life, the class, the volcanic SO2
//! frost (#678) and the non-thermal loss (#658).

use super::bare::{Bare, Ground, bare_air, bare_ground};
use super::climate::ClimateInputs;
use super::climate::Rotation;
use super::climate::branches::solve;
use super::climate::record::{Climate, record, record_titan_like};
use super::draws::{LABELS, draw, snowball_draw};
use super::interior::InteriorError;
use crate::Trial;
use science::units::Bar;
use timeline::{Seed, Trace};

use super::draws::Drawn;

/// A planet chained to its climate.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Settled {
    /// The planet and its bare air: the air the climate was solved on.
    pub bare: Bare,
    /// The climate; `None` for a planet that kept an envelope.
    pub climate: Option<Climate>,
}

/// The planet `trial` drawn from `seed` ([`draw`]), chained to its bare
/// air and settled ([`settled_with`]), the snowball draw coming from
/// `seed`'s `planet/snowball` sub-seed.
///
/// # Errors
///
/// The errors of [`draw`] and [`settled_with`].
pub(crate) fn settled(
    trial: &Trial,
    seed: Seed,
    rotation: &Rotation,
    trace: &mut Trace,
) -> Result<Settled, InteriorError> {
    let drawn = draw(seed, trial, trace)?;
    settled_with(trial, &drawn, rotation, seed.derive(LABELS.snowball), trace)
}

/// The planet `trial` of composition `drawn`, chained to its air
/// ([`bare_ground`], then [`bare_air`] with a reducing air's CO2 held
/// back) and settled under `rotation`, with the snowball draw of
/// `snowball`.
///
/// A reducing air whose climate leaves no Titan-like air (a Triton's thin
/// nitrogen, `titan_like_air`) keeps its CO2: the air is made again
/// without holding it back ([`bare_air`], on the same ground) and the
/// climate settled again (legacy `mod` l.889-893). The first pass's
/// nodes stay in the trace, as 2c's rebuilt bodies do; the
/// `planet.climate.titan-like` node records the decision.
///
/// # Errors
///
/// The errors of each step.
pub(crate) fn settled_with(
    trial: &Trial,
    drawn: &Drawn,
    rotation: &Rotation,
    snowball: Seed,
    trace: &mut Trace,
) -> Result<Settled, InteriorError> {
    let ground = match bare_ground(trial, drawn, trace)? {
        Ground::Enveloped(bare) => {
            return Ok(Settled {
                bare: *bare,
                climate: None,
            });
        }
        Ground::Surfaced(ground) => ground,
    };
    let draw = snowball_draw(snowball);
    let held = bare_air(&ground, true, trace)?;
    let (climate, air) = climate_of(&held, rotation, draw, trace)?;
    let surface = held.surface.as_ref().expect("a ground has a surface");
    if !*surface.reducing.value() {
        return Ok(Settled {
            bare: held,
            climate: Some(climate),
        });
    }
    let titan_like = record_titan_like(&climate, &air, &surface.reducing, trace)?;
    if *titan_like.value() {
        return Ok(Settled {
            bare: held,
            climate: Some(climate),
        });
    }
    let bare = bare_air(&ground, false, trace)?;
    let (climate, _) = climate_of(&bare, rotation, draw, trace)?;
    Ok(Settled {
        bare,
        climate: Some(climate),
    })
}

/// The climate of `bare` under `rotation` and the air it leaves: solved
/// ([`solve`]) and recorded ([`record`]). `ClimateInputs::of` takes no
/// SO2 frost: the volcanic SO2 frost has no port (#678).
pub(crate) fn climate_of(
    bare: &Bare,
    rotation: &Rotation,
    draw: f64,
    trace: &mut Trace,
) -> Result<(Climate, science::radiation::gases::PartialPressures), InteriorError> {
    let inputs = ClimateInputs::of(bare, rotation, Bar(0.0)).expect("a ground has a surface");
    let solved = solve(&inputs, draw);
    let climate = record(&solved, bare, rotation, draw, trace)?;
    Ok((climate, solved.settle.air))
}
