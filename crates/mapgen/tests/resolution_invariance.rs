//! The same request generated at two map sizes must give the same
//! large-scale planet: the resolution only adds detail.

use mapgen::biome::Biome;
use mapgen::generation::generate;
use mapgen::preset::Preset;
use mapgen::request::WorldRequest;
use mapgen::world::{World, WorldParams};
use rand::SeedableRng;
use rand::rngs::StdRng;

const COARSE: (u32, u32) = (256, 128);
const MIN_LAND_AGREEMENT: f64 = 0.97;
const MAX_BIOME_SHARE_DELTA: f64 = 0.02;

fn params(seed: u32) -> WorldParams {
    let request = WorldRequest {
        seed: Some(seed),
        width: Some(1024),
        height: Some(512),
        ..Default::default()
    };
    request
        .resolve_with_preset(
            Preset::load("terran").unwrap(),
            "terran".to_string(),
            &mut StdRng::seed_from_u64(u64::from(seed)),
        )
        .unwrap()
        .params
}

/// Land mask downsampled to `COARSE` by majority per block (ties: sea).
/// Land is what the biomes call land: relief (`elevation - sea_level`) at
/// or above the preset's waterline.
fn land_mask(world: &World) -> Vec<bool> {
    let waterline = world.params.sea_level + world.params.planet.rules.waterline;
    let (cw, ch) = COARSE;
    let (bw, bh) = (world.width / cw, world.height / ch);
    let mut out = Vec::with_capacity((cw * ch) as usize);
    for cy in 0..ch {
        for cx in 0..cw {
            let mut land = 0;
            for y in cy * bh..(cy + 1) * bh {
                for x in cx * bw..(cx + 1) * bw {
                    let i = (y * world.width + x) as usize;
                    land += usize::from(world.elevation[i] >= waterline);
                }
            }
            out.push(land * 2 > (bw * bh) as usize);
        }
    }
    out
}

/// Weight of an image row of a `height`-row map: cos(latitude).
fn row_weight(y: u32, height: u32) -> f64 {
    let lat = std::f64::consts::FRAC_PI_2
        - (f64::from(y) + 0.5) / f64::from(height) * std::f64::consts::PI;
    lat.cos()
}

/// Area-weighted share of each biome, indexed by `Biome::id`.
fn biome_shares(world: &World) -> Vec<f64> {
    let mut sums = vec![0.0; Biome::ALL.len()];
    let mut total = 0.0;
    for y in 0..world.height {
        let w = row_weight(y, world.height);
        for x in 0..world.width {
            sums[world.biomes[(y * world.width + x) as usize].id() as usize] += w;
            total += w;
        }
    }
    sums.iter().map(|s| s / total).collect()
}

fn check(seed: u32) {
    let low = params(seed);
    let mut high = low.clone();
    high.width = 2048;
    high.height = 1024;
    let (a, b) = (generate(&low).world, generate(&high).world);

    let (ma, mb) = (land_mask(&a), land_mask(&b));
    let (mut agree, mut total) = (0.0, 0.0);
    for (i, (la, lb)) in ma.iter().zip(&mb).enumerate() {
        let w = row_weight(i as u32 / COARSE.0, COARSE.1);
        total += w;
        if la == lb {
            agree += w;
        }
    }
    let agreement = agree / total;

    let (sa, sb) = (biome_shares(&a), biome_shares(&b));
    let (worst, delta) = Biome::ALL
        .iter()
        .map(|b| (*b, (sa[b.id() as usize] - sb[b.id() as usize]).abs()))
        .max_by(|x, y| x.1.total_cmp(&y.1))
        .unwrap();
    eprintln!("seed {seed}: land agreement {agreement:.4}, worst biome {worst:?} delta {delta:.4}");

    assert!(
        agreement >= MIN_LAND_AGREEMENT,
        "seed {seed}: land agreement {agreement:.4} < {MIN_LAND_AGREEMENT}"
    );
    assert!(
        delta <= MAX_BIOME_SHARE_DELTA,
        "seed {seed}: biome {worst:?} share differs by {delta:.4} > {MAX_BIOME_SHARE_DELTA}"
    );
}

#[test]
#[ignore = "slow: run with mise run test:invariance"]
fn same_planet_at_1024_and_2048() {
    for seed in [42, 7, 1234] {
        check(seed);
    }
}
