//! The filesystem store, in temporary directories only.

use std::fs;
use std::path::Path;

use adapters::{FsStore, StoreError};
use catalogue::{
    CacheKey, CatalogueError, Engines, StoredSystem, SystemId, SystemRepository, TimelineCache,
    WorldCache, WorldKey,
};
use system::{ArchetypeId, BodyRequest, INPUTS_SCHEMA, Inputs, Request};
use tempfile::TempDir;
use timeline::{Code, Designation, EngineVersion, Seed, SystemNumber};
use world::{Grid, World};

const ENGINE: EngineVersion = EngineVersion(1);

fn engines(system: EngineVersion) -> Engines {
    Engines {
        system,
        world: EngineVersion(1),
    }
}

fn stored(seed: u64) -> StoredSystem {
    StoredSystem::new(
        Inputs {
            schema: INPUTS_SCHEMA,
            seed: Seed::new(seed),
            present: None,
            request: Request::Body(BodyRequest {
                host_mass: None,
                metallicity: None,
                archetype: Some(ArchetypeId(Code::of("earth-like"))),
            }),
        },
        ENGINE,
    )
}

fn stored_id(seed: u64) -> SystemId {
    stored(seed).id
}

fn store(dir: &TempDir) -> FsStore {
    FsStore::new(dir.path().to_path_buf(), u64::MAX)
}

fn files_under(dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(&path, out);
        } else {
            out.push(path.to_string_lossy().into_owned());
        }
    }
}

#[test]
fn a_system_round_trips() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let mut system = stored(7);
    system.name = Some("Home".to_string());
    SystemRepository::put(&store, &system).unwrap();
    assert_eq!(
        SystemRepository::get(&store, system.id).unwrap(),
        Some(system)
    );
    let missing = stored(8).id;
    assert_eq!(SystemRepository::get(&store, missing).unwrap(), None);
}

#[test]
fn list_is_sorted_and_remove_reports() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    assert!(SystemRepository::list(&store).unwrap().is_empty());
    let systems = [stored(1), stored(2), stored(3)];
    for system in &systems {
        SystemRepository::put(&store, system).unwrap();
    }
    let mut expected: Vec<SystemId> = systems.iter().map(|s| s.id).collect();
    expected.sort();
    assert_eq!(SystemRepository::list(&store).unwrap(), expected);
    assert!(SystemRepository::remove(&store, expected[0]).unwrap());
    assert!(!SystemRepository::remove(&store, expected[0]).unwrap());
    assert_eq!(
        SystemRepository::list(&store).unwrap(),
        expected[1..].to_vec()
    );
}

#[test]
fn an_unknown_schema_on_disk_is_an_error() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let system = stored(7);
    SystemRepository::put(&store, &system).unwrap();
    let path = dir
        .path()
        .join("systems")
        .join(system.id.to_string())
        .join("inputs.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value["inputs"]["schema"] = (INPUTS_SCHEMA + 1).into();
    fs::write(&path, value.to_string()).unwrap();
    let error = SystemRepository::get(&store, system.id).unwrap_err();
    assert!(matches!(
        error,
        StoreError::Catalogue(CatalogueError::UnknownSchema { .. })
    ));
}

#[test]
fn an_edited_inputs_file_no_longer_matches_its_id() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let system = stored(7);
    SystemRepository::put(&store, &system).unwrap();
    let path = dir
        .path()
        .join("systems")
        .join(system.id.to_string())
        .join("inputs.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value["inputs"]["seed"] = serde_json::to_value(Seed::new(8)).unwrap();
    fs::write(&path, value.to_string()).unwrap();
    let error = SystemRepository::get(&store, system.id).unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::Catalogue(CatalogueError::IdMismatch { stored, computed })
                if stored == system.id && computed == stored_id(8)
        ),
        "{error}"
    );
}

#[test]
fn a_record_with_the_wrong_id_is_refused() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let mut system = stored(7);
    system.id = stored(8).id;
    let error = SystemRepository::put(&store, &system).unwrap_err();
    assert!(matches!(
        error,
        StoreError::Catalogue(CatalogueError::IdMismatch { .. })
    ));
    assert!(SystemRepository::list(&store).unwrap().is_empty());
}

#[test]
fn a_cache_hit_survives_an_unwritable_ledger() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let id = stored(7).id;
    TimelineCache::put(&store, id, ENGINE, &[]).unwrap();
    // A directory where the ledger should be: it can be neither read nor
    // replaced.
    let ledger = dir.path().join("cache/uses.json");
    fs::remove_file(&ledger).unwrap();
    fs::create_dir(&ledger).unwrap();
    assert_eq!(
        TimelineCache::get(&store, id, ENGINE).unwrap(),
        Some(Vec::new())
    );
}

#[test]
fn concurrent_writers_do_not_trip_over_each_other() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let system = stored(7);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..40 {
                    SystemRepository::put(&store, &system).unwrap();
                    TimelineCache::put(&store, system.id, ENGINE, &[]).unwrap();
                }
            });
        }
    });
    assert_eq!(
        SystemRepository::get(&store, system.id).unwrap(),
        Some(system)
    );
}

#[test]
fn eviction_removes_leftover_temporary_files() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let system = stored(7);
    SystemRepository::put(&store, &system).unwrap();
    TimelineCache::put(&store, system.id, EngineVersion(1), &[]).unwrap();
    let entry = dir.path().join("cache/1").join(system.id.to_string());
    fs::write(entry.join("timelines.json.4242-0.tmp"), b"half").unwrap();
    store.evict(engines(EngineVersion(2))).unwrap();
    assert!(!dir.path().join("cache/1").exists());
}

#[test]
fn timelines_round_trip_and_another_engine_misses() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let id = stored(7).id;
    TimelineCache::put(&store, id, EngineVersion(1), &[]).unwrap();
    assert_eq!(
        TimelineCache::get(&store, id, EngineVersion(1)).unwrap(),
        Some(Vec::new())
    );
    assert_eq!(
        TimelineCache::get(&store, id, EngineVersion(2)).unwrap(),
        None
    );
}

#[test]
fn a_corrupt_timeline_file_is_a_miss() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let id = stored(7).id;
    TimelineCache::put(&store, id, ENGINE, &[]).unwrap();
    let path = dir
        .path()
        .join("cache/1")
        .join(id.to_string())
        .join("timelines.json");
    fs::write(&path, "{").unwrap();
    assert_eq!(TimelineCache::get(&store, id, ENGINE).unwrap(), None);
}

#[test]
fn writes_leave_no_temporary_file() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let system = stored(7);
    SystemRepository::put(&store, &system).unwrap();
    TimelineCache::put(&store, system.id, ENGINE, &[]).unwrap();
    TimelineCache::get(&store, system.id, ENGINE).unwrap();
    let mut files = Vec::new();
    files_under(dir.path(), &mut files);
    let root = dir.path().to_string_lossy().into_owned();
    let mut relative: Vec<String> = files
        .iter()
        .map(|f| f.strip_prefix(&root).unwrap().to_string())
        .collect();
    relative.sort();
    let id = system.id;
    assert_eq!(
        relative,
        vec![
            "/cache/1/".to_string() + &id.to_string() + "/timelines.json",
            "/cache/uses.json".to_string(),
            "/systems/".to_string() + &id.to_string() + "/inputs.json",
        ]
    );
}

#[test]
fn eviction_drops_stale_then_least_used() {
    let dir = TempDir::new().unwrap();
    let (a, b) = (stored(1).id, stored(2).id);
    let (one, two) = (EngineVersion(1), EngineVersion(2));
    let store = FsStore::new(dir.path().to_path_buf(), 2);
    SystemRepository::put(&store, &stored(1)).unwrap();
    SystemRepository::put(&store, &stored(2)).unwrap();
    TimelineCache::put(&store, a, one, &[]).unwrap();
    TimelineCache::put(&store, a, two, &[]).unwrap();
    TimelineCache::put(&store, b, two, &[]).unwrap();
    // `[]` is two bytes; reading a makes b the least recently used.
    TimelineCache::get(&store, a, two).unwrap();
    let gone = store.evict(engines(two)).unwrap();
    assert_eq!(
        gone,
        vec![
            CacheKey::Timelines {
                system: a,
                engine: one
            },
            CacheKey::Timelines {
                system: b,
                engine: two
            },
        ]
    );
    assert_eq!(TimelineCache::get(&store, a, one).unwrap(), None);
    assert_eq!(TimelineCache::get(&store, b, two).unwrap(), None);
    assert_eq!(
        TimelineCache::get(&store, a, two).unwrap(),
        Some(Vec::new())
    );
    assert!(!dir.path().join("cache/1").exists());
}

#[test]
fn eviction_drops_caches_of_removed_systems() {
    let dir = TempDir::new().unwrap();
    let (kept, removed) = (stored(1), stored(2));
    let store = store(&dir);
    SystemRepository::put(&store, &kept).unwrap();
    SystemRepository::put(&store, &removed).unwrap();
    TimelineCache::put(&store, kept.id, ENGINE, &[]).unwrap();
    TimelineCache::put(&store, removed.id, ENGINE, &[]).unwrap();
    SystemRepository::remove(&store, removed.id).unwrap();
    let gone = store.evict(engines(ENGINE)).unwrap();
    assert_eq!(
        gone,
        vec![CacheKey::Timelines {
            system: removed.id,
            engine: ENGINE
        }]
    );
    assert_eq!(
        TimelineCache::get(&store, kept.id, ENGINE).unwrap(),
        Some(Vec::new())
    );
    assert!(
        !dir.path()
            .join("cache/1")
            .join(removed.id.to_string())
            .exists()
    );
}

fn designation() -> Designation {
    Designation::planet(SystemNumber::new(10_001).unwrap(), 1).unwrap()
}

fn world_key(system: SystemId, world: EngineVersion) -> WorldKey {
    WorldKey {
        system,
        body: designation(),
        keyframe: 0,
        grid: 1,
        engine: ENGINE,
        world,
    }
}

#[test]
fn old_world_names_are_evicted() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let system = stored(7);
    SystemRepository::put(&store, &system).unwrap();
    let slug = dir
        .path()
        .join("cache/1")
        .join(system.id.to_string())
        .join("worlds")
        .join(designation().to_string().replace(' ', "-"));
    fs::create_dir_all(&slug).unwrap();
    let old = slug.join("0-8.world");
    fs::write(&old, b"MGWORLD1").unwrap();
    assert!(
        WorldCache::get(
            &store,
            &WorldKey {
                grid: 8,
                ..world_key(system.id, EngineVersion(1))
            }
        )
        .unwrap()
        .is_none()
    );
    store.evict(engines(ENGINE)).unwrap();
    assert!(!old.exists());
}

#[test]
fn world_key_roundtrip_with_version() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let system = stored(7);
    SystemRepository::put(&store, &system).unwrap();
    let key = world_key(system.id, EngineVersion(2));
    let world = World::new(Grid::new(1), designation(), 0, ENGINE, EngineVersion(2));
    WorldCache::put(&store, &key, &world).unwrap();
    assert_eq!(WorldCache::get(&store, &key).unwrap(), Some(world));
    let other = world_key(system.id, EngineVersion(1));
    assert!(WorldCache::get(&store, &other).unwrap().is_none());
}
