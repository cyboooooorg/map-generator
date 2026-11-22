//! The world codec and the world cache, in temporary directories only.

use std::fs;
use std::path::PathBuf;

use adapters::{CodecError, FsStore, decode, encode};
use catalogue::{CacheKey, StoredSystem, SystemId, SystemRepository, WorldCache, WorldKey};
use system::{ArchetypeId, BodyRequest, INPUTS_SCHEMA, Inputs, Request};
use tempfile::TempDir;
use timeline::{Code, Designation, EngineVersion, Seed, SystemNumber};
use world::{Field, FieldId, Grid, MAX_N, Storage, Values, World};

const ENGINE: EngineVersion = EngineVersion(1);
const WORLD_ENGINE: EngineVersion = EngineVersion(1);

fn designation() -> Designation {
    Designation::planet(SystemNumber::new(10_001).unwrap(), 1).unwrap()
}

fn field(values: Values, categorical: bool, storage: Storage) -> Field {
    Field {
        values,
        categorical,
        storage,
    }
}

fn sample() -> World {
    sample_with(ENGINE, WORLD_ENGINE)
}

fn sample_with(engine: EngineVersion, world_engine: EngineVersion) -> World {
    let grid = Grid::new(4);
    let cells = grid.cell_count();
    let mut world = World::new(grid, designation(), 3, engine, world_engine);
    let heights: Vec<f32> = (0..cells).map(|k| k as f32 * 0.5 - 7.0).collect();
    let ids: Vec<u16> = (0..cells).map(|k| (k * 700) as u16).collect();
    let flags: Vec<u8> = (0..cells).map(|k| (k % 5) as u8).collect();
    for (code, f) in [
        (
            "relief.height",
            field(Values::F32(heights), false, Storage::Exact),
        ),
        ("surface.id", field(Values::U16(ids), true, Storage::Exact)),
        (
            "surface.flag",
            field(Values::U8(flags), true, Storage::Exact),
        ),
    ] {
        world
            .insert(FieldId(Code::new(code.to_string()).unwrap()), f)
            .unwrap();
    }
    world
}

fn key(system: SystemId) -> WorldKey {
    WorldKey {
        system,
        body: designation(),
        keyframe: 3,
        grid: 4,
        engine: ENGINE,
    }
}

fn system_id() -> SystemId {
    stored(7).id
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

fn world_file(dir: &TempDir, id: SystemId) -> PathBuf {
    let worlds = dir
        .path()
        .join("cache/1")
        .join(id.to_string())
        .join("worlds");
    let slug = fs::read_dir(&worlds)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::read_dir(slug).unwrap().next().unwrap().unwrap().path()
}

#[test]
fn a_world_round_trips() {
    let world = sample_with(EngineVersion(3), EngineVersion(7));
    let decoded = decode(&encode(&world)).unwrap();
    assert_eq!(decoded.engine(), EngineVersion(3));
    assert_eq!(decoded.world_engine(), EngineVersion(7));
    assert_eq!(decoded, world);
}

#[test]
fn mgworld1_is_refused() {
    let mut bytes = encode(&sample());
    bytes[..8].copy_from_slice(b"MGWORLD1");
    assert!(matches!(decode(&bytes), Err(CodecError::Magic)));
}

#[test]
fn u16_storage_round_trips_within_one_step() {
    let (min, max) = (20.0_f32, 3000.0_f32);
    let grid = Grid::new(1);
    let mut world = World::new(grid, designation(), 0, ENGINE, WORLD_ENGINE);
    let values = vec![20.0, 288.15, 3000.0, 20.0, 100.0, 2999.0];
    world
        .insert(
            FieldId(Code::of("climate.temperature")),
            field(
                Values::F32(values.clone()),
                false,
                Storage::U16 { min, max },
            ),
        )
        .unwrap();
    let back = decode(&encode(&world)).unwrap();
    let field = back
        .field(&FieldId(Code::of("climate.temperature")))
        .unwrap();
    assert_eq!(field.storage, Storage::U16 { min, max });
    let Values::F32(read) = &field.values else {
        panic!("not F32");
    };
    let step = (max - min) / 65535.0;
    for (a, b) in values.iter().zip(read) {
        assert!((a - b).abs() <= step, "{a} vs {b}");
    }
}

#[test]
fn a_truncated_file_fails() {
    let bytes = encode(&sample());
    for len in 0..bytes.len() {
        assert!(
            matches!(decode(&bytes[..len]), Err(CodecError::Truncated)),
            "prefix of {len} bytes"
        );
    }
}

#[test]
fn a_wrong_magic_fails() {
    let mut bytes = encode(&sample());
    bytes[0] = b'X';
    assert!(matches!(decode(&bytes), Err(CodecError::Magic)));
}

#[test]
fn a_world_put_is_got_back() {
    let dir = TempDir::new().unwrap();
    let store = FsStore::new(dir.path().to_path_buf(), u64::MAX);
    let key = key(system_id());
    assert!(WorldCache::get(&store, &key).unwrap().is_none());
    let world = sample();
    WorldCache::put(&store, &key, &world).unwrap();
    assert_eq!(WorldCache::get(&store, &key).unwrap(), Some(world));
    let other = WorldKey {
        engine: EngineVersion(2),
        ..key
    };
    assert!(WorldCache::get(&store, &other).unwrap().is_none());
}

#[test]
fn a_corrupt_world_file_is_a_miss() {
    let dir = TempDir::new().unwrap();
    let store = FsStore::new(dir.path().to_path_buf(), u64::MAX);
    let key = key(system_id());
    WorldCache::put(&store, &key, &sample()).unwrap();
    let path = world_file(&dir, key.system);
    let bytes = fs::read(&path).unwrap();
    fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
    assert!(WorldCache::get(&store, &key).unwrap().is_none());
    fs::write(&path, b"garbage").unwrap();
    assert!(WorldCache::get(&store, &key).unwrap().is_none());
}

#[test]
fn world_files_are_evicted_by_the_policy() {
    let dir = TempDir::new().unwrap();
    let system = stored(7);
    let store = FsStore::new(dir.path().to_path_buf(), 0);
    SystemRepository::put(&store, &system).unwrap();
    let key = key(system.id);
    WorldCache::put(&store, &key, &sample()).unwrap();
    assert_eq!(
        store.evict(ENGINE).unwrap(),
        vec![CacheKey::World(key.clone())]
    );
    assert!(WorldCache::get(&store, &key).unwrap().is_none());
}

#[test]
fn a_world_put_under_another_key_is_refused() {
    let dir = TempDir::new().unwrap();
    let store = FsStore::new(dir.path().to_path_buf(), u64::MAX);
    let good = key(system_id());
    let world = sample();
    let wrong = [
        WorldKey {
            keyframe: 4,
            ..good.clone()
        },
        WorldKey {
            grid: 5,
            ..good.clone()
        },
        WorldKey {
            engine: EngineVersion(2),
            ..good.clone()
        },
        WorldKey {
            body: Designation::planet(SystemNumber::new(10_001).unwrap(), 2).unwrap(),
            ..good.clone()
        },
    ];
    for key in &wrong {
        assert!(WorldCache::put(&store, key, &world).is_err(), "{key:?}");
    }
    assert!(!dir.path().join("cache").exists());
    WorldCache::put(&store, &good, &world).unwrap();
}

#[test]
fn a_grid_above_the_cap_is_refused_on_both_sides() {
    // Decoding: the header of the sample with n patched to MAX_N + 1.
    let mut bytes = encode(&sample());
    let n_at = 8 + 4 + 4 + 2 + designation().to_string().len() + 4;
    bytes[n_at..n_at + 4].copy_from_slice(&(MAX_N + 1).to_le_bytes());
    assert!(matches!(decode(&bytes), Err(CodecError::Invalid(_))));
    bytes[n_at..n_at + 4].copy_from_slice(&0u32.to_le_bytes());
    assert!(matches!(decode(&bytes), Err(CodecError::Invalid(_))));
    // Caching: no unreadable file is written.
    let dir = TempDir::new().unwrap();
    let store = FsStore::new(dir.path().to_path_buf(), u64::MAX);
    let big = World::new(Grid::new(MAX_N + 1), designation(), 3, ENGINE, WORLD_ENGINE);
    let key = WorldKey {
        grid: MAX_N + 1,
        ..key(system_id())
    };
    assert!(WorldCache::put(&store, &key, &big).is_err());
    assert!(!dir.path().join("cache").exists());
}

#[test]
fn a_header_claiming_a_huge_grid_without_payload_is_truncated() {
    // The header of the sample with n patched to the cap, and no values.
    let mut bytes = encode(&sample());
    let header_len = bytes.len() - 96 * (4 + 2 + 1);
    bytes.truncate(header_len);
    let n_at = 8 + 4 + 2 + designation().to_string().len() + 4;
    bytes[n_at..n_at + 4].copy_from_slice(&MAX_N.to_le_bytes());
    assert!(matches!(decode(&bytes), Err(CodecError::Truncated)));
}
