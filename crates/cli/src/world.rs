//! The `world` command: stored inputs to a cached world and its export.
//!
//! The pipeline [`run`] takes the timelines from an injected producer so
//! it is tested without running the system engine; [`command`] wires the
//! real one in.

use std::path::PathBuf;

use clap::Args;
use serde::Serialize;

use adapters::{Exported, FsStore, Ramp, export};
use catalogue::{
    CacheKey, Engines, StoredSystem, SystemId, SystemRepository, TimelineCache, WorldCache,
    WorldKey,
};
use science::units::Metre;
use system::{ENGINE_VERSION, Inputs};
use timeline::{BodyState, Code, Designation, EngineVersion, Seed, Timeline};
use world::relief::{self, HEIGHT};
use world::{FieldId, Grid, World};

use crate::args::{GlobalArgs, SystemArgs};
use crate::error::CliError;

/// Definition: metres in a kilometre, for `--cell-km`.
const METRES_PER_KM: f64 = 1000.0;

/// `map-generator world`: the draw's flags, the body, and the export.
#[derive(Args, Debug)]
pub struct WorldCommand {
    #[command(flatten)]
    pub system: SystemArgs,
    /// The body: the last part of its designation, `b`, `c`… for the
    /// planets in order from the star, `A` for the star (refused: it has
    /// no surface).
    #[arg(long)]
    pub body: String,
    /// The grid's cell size in km, above 0.
    #[arg(
        long,
        value_name = "KM",
        allow_negative_numbers = true,
        default_value_t = world::DEFAULT_CELL.0 / METRES_PER_KM,
        value_parser = parse_cell_km
    )]
    pub cell_km: f64,
    /// The field to project.
    #[arg(long, value_name = "CODE", default_value = HEIGHT, value_parser = parse_field)]
    pub field: FieldId,
    /// The PNG width in pixels, even and above 0; the height is half.
    #[arg(long, value_name = "PIXELS", default_value_t = 2048, allow_negative_numbers = true, value_parser = parse_width)]
    pub width: u32,
    /// The directory the world file and the PNG are written to.
    #[arg(long, value_name = "DIR")]
    pub out: PathBuf,
}

/// Parses `--cell-km`: a finite cell size above 0 km.
pub fn parse_cell_km(text: &str) -> Result<f64, String> {
    let km = text
        .parse::<f64>()
        .map_err(|error| format!("the cell size {text:?} is not a number: {error}"))?;
    if km.is_finite() && km > 0.0 {
        Ok(km)
    } else {
        Err(format!(
            "the cell size must be finite and above 0 km, got {text}"
        ))
    }
}

/// Parses `--width`: an even number of pixels above 0, as the 2:1
/// projection needs.
pub fn parse_width(text: &str) -> Result<u32, String> {
    let width = text
        .parse::<u32>()
        .map_err(|error| format!("the width {text:?} is not a number of pixels: {error}"))?;
    if width == 0 || !width.is_multiple_of(2) {
        return Err(format!("the width must be even and above 0, got {width}"));
    }
    Ok(width)
}

/// Parses `--field`: the shape of a code only; the export refuses a field
/// the world lacks.
pub fn parse_field(text: &str) -> Result<FieldId, String> {
    Code::new(text.to_owned())
        .map(FieldId)
        .map_err(|error| format!("{error}"))
}

/// What the `world` command prints.
#[derive(Serialize, Debug)]
struct Summary {
    system: SystemId,
    body: Designation,
    keyframe: u32,
    /// The grid's subdivision count.
    grid: u32,
    /// The system engine the timeline comes from.
    engine: EngineVersion,
    /// The map generator that made the world.
    world_engine: EngineVersion,
    timelines_cached: bool,
    world_cached: bool,
    /// How many cache entries the eviction removed after the run.
    evicted: usize,
    world: PathBuf,
    png: PathBuf,
}

/// Runs the command with the real system engine and returns the JSON
/// summary for stdout. `main` resolves the seed first.
///
/// # Errors
///
/// Those of [`run`], and a summary that cannot be written as JSON.
pub fn command(
    command: &WorldCommand,
    global: &GlobalArgs,
    seed: Seed,
) -> Result<String, CliError> {
    let request = WorldRequest {
        body: command.body.clone(),
        cell: Metre(command.cell_km * METRES_PER_KM),
        field: command.field.clone(),
        width: command.width,
        out: command.out.clone(),
    };
    let done = run(
        &global.store(),
        &command.system.inputs(seed),
        &request,
        system::generate,
    )?;
    let summary = Summary {
        system: done.system,
        body: done.key.body,
        keyframe: done.key.keyframe,
        grid: done.key.grid,
        engine: done.key.engine,
        world_engine: done.key.world,
        timelines_cached: done.timelines_cached,
        world_cached: done.world_cached,
        evicted: done.evicted.len(),
        world: done.exported.world,
        png: done.exported.png,
    };
    Ok(serde_json::to_string_pretty(&summary)?)
}

/// What to build and where to write it.
#[derive(Clone, Debug)]
pub struct WorldRequest {
    /// The body part of the designation: `b`, `c`… for the planets, `A`
    /// for the star. The system number comes from the drawn system.
    pub body: String,
    /// The cell size, [`world::DEFAULT_CELL`] by default.
    pub cell: Metre,
    /// The field to project, `relief.height` by default.
    pub field: FieldId,
    /// The PNG width in pixels: even and above 0, 2048 by default.
    pub width: u32,
    pub out: PathBuf,
}

/// What a run did.
#[derive(Clone, Debug)]
pub struct WorldRun {
    pub system: SystemId,
    pub key: WorldKey,
    /// Whether the timelines came from the cache in this run.
    pub timelines_cached: bool,
    /// Whether the world came from the cache in this run.
    pub world_cached: bool,
    /// The cache entries the eviction removed after the run.
    pub evicted: Vec<CacheKey>,
    pub exported: Exported,
}

/// Builds (or loads) the world of `request.body` at its present, then
/// exports it. Nothing is written to `request.out` before the world
/// exists. `timelines` is called only on a timeline cache miss.
///
/// # Errors
///
/// A body text that is not a designation's body part (refused before the
/// producer runs), an odd or zero width, a store, producer, relief or
/// export failure, or a body that is missing, is a star, or has no
/// keyframe at its present.
pub fn run<E>(
    store: &FsStore,
    inputs: &Inputs,
    request: &WorldRequest,
    timelines: impl FnOnce(&Inputs) -> Result<Vec<Timeline>, E>,
) -> Result<WorldRun, CliError>
where
    CliError: From<E>,
{
    // Any valid system number: the body part is checked before the engine
    // runs, the real number is known only once the timelines exist.
    if format!("MGS 10000 {}", request.body)
        .parse::<Designation>()
        .is_err()
    {
        return Err(CliError::InvalidBody(request.body.clone()));
    }
    if request.width == 0 || !request.width.is_multiple_of(2) {
        return Err(CliError::Width(request.width));
    }
    let id = SystemId::of(inputs);
    let (all, timelines_cached) = match TimelineCache::get(store, id, ENGINE_VERSION)? {
        Some(all) => (all, true),
        None => (timelines(inputs)?, false),
    };
    // The inputs become stored truth only once the engine has accepted
    // them, and before their cache entry (eviction drops the caches of
    // systems no longer stored).
    if SystemRepository::get(store, id)?.is_none() {
        SystemRepository::put(store, &StoredSystem::new(inputs.clone(), ENGINE_VERSION))?;
    }
    if !timelines_cached {
        TimelineCache::put(store, id, ENGINE_VERSION, &all)?;
    }
    let no_such_body = || CliError::NoSuchBody(request.body.clone());
    let number = all.first().ok_or_else(no_such_body)?.body.system.get();
    let body: Designation = format!("MGS {number} {}", request.body)
        .parse()
        .map_err(|_| CliError::InvalidBody(request.body.clone()))?;
    let timeline = all
        .iter()
        .find(|timeline| timeline.body == body)
        .ok_or_else(no_such_body)?;
    let index = timeline
        .keyframes
        .iter()
        .position(|keyframe| keyframe.date() == timeline.present)
        .ok_or(CliError::NoPresentKeyframe(body))?;
    let BodyState::Planet(state) = timeline.keyframes[index].state() else {
        return Err(CliError::NotAPlanet(body));
    };
    let grid = Grid::for_radius(Metre::from(*state.bulk.radius.value()), request.cell);
    let key = WorldKey {
        system: id,
        body,
        keyframe: u32::try_from(index).expect("a timeline has fewer than 2^32 keyframes"),
        grid: grid.n(),
        engine: timeline.engine,
        world: world::ENGINE_VERSION,
    };
    let (world, world_cached) = match WorldCache::get(store, &key)? {
        Some(world) => (world, true),
        None => {
            let field = relief::generate(state, timeline.seed, &grid)?;
            let mut world = World::new(grid, key.body, key.keyframe, key.engine, key.world);
            world
                .insert(FieldId(Code::of(HEIGHT)), field)
                .map_err(CliError::World)?;
            WorldCache::put(store, &key, &world)?;
            // The cache stores the fields quantised: export what a later
            // run will read back, so a hit and a miss write the same bytes.
            let world = adapters::decode(&adapters::encode(&world))?;
            (world, false)
        }
    };
    let evicted = store.evict(Engines {
        system: ENGINE_VERSION,
        world: world::ENGINE_VERSION,
    })?;
    let ramp = ramp_for(&world, &request.field);
    let exported = export(&request.out, &world, &request.field, request.width, &ramp)?;
    Ok(WorldRun {
        system: id,
        key,
        timelines_cached,
        world_cached,
        evicted,
        exported,
    })
}

/// The relief ramp for the height; a grey ramp over the min and max of
/// the field's finite values otherwise (0 to 1 when it has none) (an absent field is left to the export to refuse).
fn ramp_for(world: &World, field: &FieldId) -> Ramp {
    if field.0.as_str() == HEIGHT {
        return Ramp::relief();
    }
    let Some(field) = world.field(field) else {
        return Ramp::relief();
    };
    let (min, max) = (0..field.values.len())
        .map(|k| field.values.get_f32(k))
        .filter(|v| v.is_finite())
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    if min > max {
        return Ramp::grey(0.0, 1.0);
    }
    Ramp::grey(min, max)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::path::Path;

    use science::units::Gyr;
    use science::units::{Dex, Kelvin, SolarLuminosity, SolarMass, SolarRadius};
    use system::{BodyRequest, INPUTS_SCHEMA, Request};
    use tempfile::TempDir;
    use timeline::fixtures::{earth_like_fixture, leaf};
    use timeline::{ContractError, Keyframe, StarPhase, StarState, SystemNumber, Trace};

    use super::*;

    const WIDTH: u32 = 64;

    fn inputs() -> Inputs {
        Inputs {
            schema: INPUTS_SCHEMA,
            seed: Seed::new(7),
            present: Some(Gyr(4.57)),
            request: Request::Body(BodyRequest {
                host_mass: None,
                metallicity: None,
                archetype: None,
            }),
        }
    }

    fn number() -> SystemNumber {
        SystemNumber::new(10_001).unwrap()
    }

    fn designation(rank: u32) -> Designation {
        Designation::planet(number(), rank).unwrap()
    }

    /// The Sun's nominal values, as in `timeline`'s keyframe tests.
    fn sun(trace: &mut Trace) -> StarState {
        StarState {
            phase: StarPhase::MainSequence,
            mass: leaf(trace, "star.mass", SolarMass(1.0)),
            radius: leaf(trace, "star.radius", SolarRadius(1.0)),
            luminosity: leaf(trace, "star.luminosity", SolarLuminosity(1.0)),
            effective_temperature: leaf(trace, "star.teff", Kelvin(5772.0)),
            metallicity: leaf(trace, "star.metallicity", Dex(0.0)),
            interior: None,
        }
    }

    /// A star timeline followed by the planet of [`timelines`].
    fn star_and_planet() -> Result<Vec<Timeline>, ContractError> {
        let mut trace = Trace::new();
        let state = sun(&mut trace);
        let keyframe = Keyframe::new(Gyr(4.57), BodyState::Star(state), &trace)?;
        let star = Timeline::new(
            Designation::star(number()),
            Seed::new(1),
            ENGINE_VERSION,
            Gyr(4.57),
            vec![keyframe],
        )?;
        let mut all = vec![star];
        all.extend(timelines(false)?);
        Ok(all)
    }

    fn timelines(surfaceless: bool) -> Result<Vec<Timeline>, ContractError> {
        let mut trace = Trace::new();
        let mut state = earth_like_fixture(&mut trace, 4.57);
        if surfaceless {
            state.surface = None;
        }
        let keyframe = Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace)?;
        Ok(vec![Timeline::new(
            designation(1),
            Seed::new(1),
            ENGINE_VERSION,
            Gyr(4.57),
            vec![keyframe],
        )?])
    }

    fn two_keyframes(present: f64) -> Result<Vec<Timeline>, ContractError> {
        let mut keyframes = Vec::new();
        for age in [4.0, 4.57] {
            let mut trace = Trace::new();
            let state = earth_like_fixture(&mut trace, age);
            keyframes.push(Keyframe::new(Gyr(age), BodyState::Planet(state), &trace)?);
        }
        Ok(vec![Timeline::new(
            designation(1),
            Seed::new(1),
            ENGINE_VERSION,
            Gyr(present),
            keyframes,
        )?])
    }

    fn request(out: &Path, body: &str) -> WorldRequest {
        WorldRequest {
            body: body.to_owned(),
            cell: Metre(2_000_000.0),
            field: FieldId(Code::of(HEIGHT)),
            width: WIDTH,
            out: out.join("out"),
        }
    }

    fn count(dir: &Path) -> usize {
        fs::read_dir(dir).map_or(0, Iterator::count)
    }

    #[test]
    #[ignore = "heavy: generates a world"]
    fn first_run_generates_then_second_hits_caches() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), "c");
        let first = run(&store, &inputs(), &request, |_| timelines(false)).unwrap();
        assert!(!first.timelines_cached && !first.world_cached);
        assert_eq!(first.system, first.key.system);
        let bytes = fs::read(&first.exported.png).unwrap();

        let second = run(
            &store,
            &inputs(),
            &request,
            |_| -> Result<_, ContractError> { panic!("the timelines are cached") },
        )
        .unwrap();
        assert!(second.timelines_cached && second.world_cached);
        assert_eq!(second.key, first.key);
        assert!(fs::read(&second.exported.png).unwrap() == bytes);
    }

    #[test]
    fn malformed_body_is_refused_before_the_engine_runs() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        for bad in ["", "b c", "aa", "MGS 10001 b"] {
            let request = request(dir.path(), bad);
            let error = run(
                &store,
                &inputs(),
                &request,
                |_| -> Result<_, ContractError> { panic!("the engine must not run") },
            )
            .unwrap_err();
            assert!(
                matches!(error, CliError::InvalidBody(_)),
                "{bad:?}: {error}"
            );
        }
    }

    #[test]
    fn star_is_refused() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), "A");
        let error = run(&store, &inputs(), &request, |_| star_and_planet()).unwrap_err();
        assert!(
            matches!(error, CliError::NotAPlanet(d) if d == Designation::star(number())),
            "{error}"
        );
        assert_eq!(count(&request.out), 0);
    }

    #[test]
    fn unknown_body_is_refused() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), "d");
        let error = run(&store, &inputs(), &request, |_| timelines(false)).unwrap_err();
        assert!(matches!(error, CliError::NoSuchBody(_)), "{error}");
        assert_eq!(count(&request.out), 0);
    }

    #[test]
    fn surfaceless_planet_is_refused() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), "c");
        let error = run(&store, &inputs(), &request, |_| timelines(true)).unwrap_err();
        assert!(
            matches!(error, CliError::Relief(relief::ReliefError::NoSurface)),
            "{error}"
        );
        assert_eq!(count(&request.out), 0);
    }

    #[test]
    #[ignore = "heavy: generates a world"]
    fn zero_budget_still_exports() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), 0);
        let request = request(dir.path(), "c");
        let run = run(&store, &inputs(), &request, |_| timelines(false)).unwrap();
        assert!(run.exported.world.exists() && run.exported.png.exists());
        assert!(run.evicted.contains(&CacheKey::World(run.key.clone())));
        assert!(
            run.evicted
                .iter()
                .any(|key| matches!(key, CacheKey::Timelines { .. }))
        );
        let mut leftovers = Vec::new();
        let mut stack = vec![dir.path().join("store")];
        while let Some(path) = stack.pop() {
            for entry in fs::read_dir(&path).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "world")
                    || path.file_name().is_some_and(|n| n == "timelines.json")
                {
                    leftovers.push(path);
                }
            }
        }
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn producer_error_is_passed_through() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), "c");
        let called = Cell::new(false);
        let error = run(&store, &inputs(), &request, |_| {
            called.set(true);
            Err(ContractError::NoKeyframe)
        })
        .unwrap_err();
        assert!(called.get());
        assert!(matches!(
            error,
            CliError::Contract(ContractError::NoKeyframe)
        ));
        // Inputs the engine refuses are not stored truth.
        assert!(
            SystemRepository::get(&store, SystemId::of(&inputs()))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn odd_width_is_refused() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = WorldRequest {
            width: 63,
            ..request(dir.path(), "c")
        };
        let error = run(
            &store,
            &inputs(),
            &request,
            |_| -> Result<_, ContractError> { panic!("refused before the producer") },
        )
        .unwrap_err();
        assert!(matches!(error, CliError::Width(63)), "{error}");
        assert!(error.to_string().starts_with("cli.world.width: 63"));
        assert!(!dir.path().join("store").exists());
        assert_eq!(count(&request.out), 0);
    }

    #[test]
    fn no_present_keyframe_is_refused() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), "c");
        let error = run(&store, &inputs(), &request, |_| two_keyframes(4.3)).unwrap_err();
        assert!(matches!(error, CliError::NoPresentKeyframe(_)), "{error}");
        assert_eq!(count(&request.out), 0);
    }

    #[test]
    fn a_ramp_over_non_finite_values_does_not_panic() {
        let grid = Grid::new(1);
        let mut world = World::new(
            grid,
            designation(1),
            0,
            ENGINE_VERSION,
            world::ENGINE_VERSION,
        );
        let id = FieldId(Code::of("test.nan"));
        let cells = world.grid().cell_count();
        let field = |values: Vec<f32>| world::Field {
            values: world::Values::F32(values),
            categorical: false,
            storage: world::Storage::Exact,
        };
        world
            .insert(id.clone(), field(vec![f32::NAN; cells]))
            .unwrap();
        let ramp = ramp_for(&world, &id);
        assert_eq!(ramp.colour(0.0), [0, 0, 0]);
        assert_eq!(ramp.colour(1.0), [255, 255, 255]);
        let mut mixed = vec![f32::NAN; cells];
        mixed[0] = -2.0;
        mixed[1] = 5.0;
        let other = FieldId(Code::of("test.mixed"));
        world.insert(other.clone(), field(mixed)).unwrap();
        let ramp = ramp_for(&world, &other);
        assert_eq!(ramp.colour(-2.0), [0, 0, 0]);
        assert_eq!(ramp.colour(5.0), [255, 255, 255]);
    }
}
