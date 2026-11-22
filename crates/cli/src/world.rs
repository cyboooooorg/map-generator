//! The world pipeline: stored inputs to a cached world and its export.
//!
//! The timelines come from an injected producer so the pipeline is tested
//! without running the system engine; the `world` command wires the real
//! one in.

use std::path::PathBuf;

use adapters::{Exported, FsStore, Ramp, export};
use catalogue::{
    CacheKey, Engines, StoredSystem, SystemId, SystemRepository, TimelineCache, WorldCache,
    WorldKey,
};
use science::units::Metre;
use system::{ENGINE_VERSION, Inputs};
use timeline::{BodyState, Designation, Timeline};
use world::relief::{self, HEIGHT};
use world::{FieldId, Grid, World};

use crate::error::CliError;

/// What to build and where to write it.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by the world command, pull request B")
)]
#[derive(Clone, Debug)]
pub struct WorldRequest {
    pub body: Designation,
    /// The cell size, [`world::DEFAULT_CELL`] by default.
    pub cell: Metre,
    /// The field to project, `relief.height` by default.
    pub field: FieldId,
    /// The PNG width in pixels: even and above 0, 2048 by default.
    pub width: u32,
    pub out: PathBuf,
}

/// What a run did.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by the world command, pull request B")
)]
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
/// A store, producer, relief or export failure, or a body that is
/// missing, is a star, or has no keyframe at its present.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by the world command, pull request B")
)]
pub fn run<E>(
    store: &FsStore,
    inputs: &Inputs,
    request: &WorldRequest,
    timelines: impl FnOnce(&Inputs) -> Result<Vec<Timeline>, E>,
) -> Result<WorldRun, CliError>
where
    CliError: From<E>,
{
    let id = SystemId::of(inputs);
    if SystemRepository::get(store, id)?.is_none() {
        SystemRepository::put(store, &StoredSystem::new(inputs.clone(), ENGINE_VERSION))?;
    }
    let (all, timelines_cached) = match TimelineCache::get(store, id, ENGINE_VERSION)? {
        Some(all) => (all, true),
        None => {
            let all = timelines(inputs)?;
            TimelineCache::put(store, id, ENGINE_VERSION, &all)?;
            (all, false)
        }
    };
    let timeline = all
        .iter()
        .find(|timeline| timeline.body == request.body)
        .ok_or(CliError::NoSuchBody(request.body))?;
    let index = timeline
        .keyframes
        .iter()
        .position(|keyframe| keyframe.date() == timeline.present)
        .ok_or(CliError::NoPresentKeyframe(request.body))?;
    let BodyState::Planet(state) = timeline.keyframes[index].state() else {
        return Err(CliError::NotAPlanet(request.body));
    };
    let grid = Grid::for_radius(Metre::from(*state.bulk.radius.value()), request.cell);
    let key = WorldKey {
        system: id,
        body: request.body,
        keyframe: u32::try_from(index).expect("a timeline has fewer than 2^32 keyframes"),
        grid: grid.n(),
        engine: ENGINE_VERSION,
        world: world::ENGINE_VERSION,
    };
    let (world, world_cached) = match WorldCache::get(store, &key)? {
        Some(world) => (world, true),
        None => {
            let field = relief::generate(state, timeline.seed, &grid)?;
            let mut world = World::new(grid, key.body, key.keyframe, key.engine, key.world);
            world
                .insert(FieldId(timeline::Code::of(HEIGHT)), field)
                .map_err(|error| CliError::Export(error.into()))?;
            WorldCache::put(store, &key, &world)?;
            // The cache stores the fields quantised: export what a later
            // run will read back, so a hit and a miss write the same bytes.
            (WorldCache::get(store, &key)?.unwrap_or(world), false)
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

/// The relief ramp for the height; a grey ramp over the field's min and
/// max otherwise (an absent field is left to the export to refuse).
fn ramp_for(world: &World, field: &FieldId) -> Ramp {
    if field.0.as_str() == HEIGHT {
        return Ramp::relief();
    }
    let Some(field) = world.field(field) else {
        return Ramp::relief();
    };
    let (min, max) = (0..field.values.len())
        .map(|k| field.values.get_f32(k))
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    Ramp::grey(min, max)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::path::Path;

    use science::units::Gyr;
    use system::{BodyRequest, INPUTS_SCHEMA, Request};
    use tempfile::TempDir;
    use timeline::fixtures::earth_like_fixture;
    use timeline::{Code, ContractError, Keyframe, Seed, SystemNumber, Trace};

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

    fn designation(rank: u32) -> Designation {
        Designation::planet(SystemNumber::new(10_001).unwrap(), rank).unwrap()
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

    fn request(out: &Path, rank: u32) -> WorldRequest {
        WorldRequest {
            body: designation(rank),
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
    fn first_run_generates_then_second_hits_caches() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), 1);
        let first = run(&store, &inputs(), &request, |_| timelines(false)).unwrap();
        assert!(!first.timelines_cached && !first.world_cached);
        assert_eq!(first.system, first.key.system);
        assert_eq!(first.key.grid, 5, "the fixture radius gives n = 5");
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
    fn unknown_body_is_refused() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), 2);
        let error = run(&store, &inputs(), &request, |_| timelines(false)).unwrap_err();
        assert!(matches!(error, CliError::NoSuchBody(_)), "{error}");
        assert_eq!(count(&request.out), 0);
    }

    #[test]
    fn surfaceless_planet_is_refused() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), u64::MAX);
        let request = request(dir.path(), 1);
        let error = run(&store, &inputs(), &request, |_| timelines(true)).unwrap_err();
        assert!(
            matches!(error, CliError::Relief(relief::ReliefError::NoSurface)),
            "{error}"
        );
        assert_eq!(count(&request.out), 0);
    }

    #[test]
    fn zero_budget_still_exports() {
        let dir = TempDir::new().unwrap();
        let store = FsStore::new(dir.path().join("store"), 0);
        let request = request(dir.path(), 1);
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
        let request = request(dir.path(), 1);
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
    }
}
