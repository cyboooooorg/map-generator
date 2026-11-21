use super::*;
use timeline::{Designation, SystemNumber};

const NOW: EngineVersion = EngineVersion(5);

fn system(n: u64) -> SystemId {
    format!("{n:016x}").parse().expect("valid id")
}

fn timelines(n: u64, engine: EngineVersion, bytes: u64, last_use: u64) -> CacheEntry {
    CacheEntry {
        key: CacheKey::Timelines {
            system: system(n),
            engine,
        },
        bytes,
        last_use,
    }
}

fn world(n: u64, engine: EngineVersion, bytes: u64, last_use: u64) -> CacheEntry {
    let body = Designation::star(SystemNumber::new(10_000).expect("in range"));
    CacheEntry {
        key: CacheKey::World(WorldKey {
            system: system(n),
            body,
            keyframe: 0,
            grid: 64,
            engine,
        }),
        bytes,
        last_use,
    }
}

#[test]
fn stale_entries_always_go() {
    let stale = timelines(1, EngineVersion(4), 10, 9);
    let current = timelines(2, NOW, 10, 1);
    let gone = evict(&[current, stale.clone()], NOW, u64::MAX);
    assert_eq!(gone, vec![stale.key]);
}

#[test]
fn under_budget_nothing_current_goes() {
    let entries = [timelines(1, NOW, 100, 1), world(2, NOW, 100, 2)];
    assert!(evict(&entries, NOW, 200).is_empty());
}

#[test]
fn least_recently_used_go_first_until_under_budget() {
    let a = timelines(1, NOW, 100, 3);
    let b = timelines(2, NOW, 100, 1);
    let c = timelines(3, NOW, 100, 2);
    let gone = evict(&[a, b.clone(), c.clone()], NOW, 150);
    assert_eq!(gone, vec![b.key, c.key]);
}

#[test]
fn ties_break_by_key_order() {
    let low = timelines(1, NOW, 100, 7);
    let high = timelines(2, NOW, 100, 7);
    let gone = evict(&[high, low.clone()], NOW, 100);
    assert_eq!(gone, vec![low.key]);
}

#[test]
fn a_zero_budget_drops_everything() {
    let stale = world(1, EngineVersion(1), 5, 1);
    let a = timelines(2, NOW, 100, 2);
    let b = world(3, NOW, 100, 1);
    let gone = evict(&[a.clone(), stale.clone(), b.clone()], NOW, 0);
    assert_eq!(gone, vec![stale.key, b.key, a.key]);
}

#[test]
fn the_key_reports_its_engine() {
    assert_eq!(
        timelines(1, EngineVersion(2), 1, 1).key.engine(),
        EngineVersion(2)
    );
    assert_eq!(
        world(1, EngineVersion(3), 1, 1).key.engine(),
        EngineVersion(3)
    );
}
