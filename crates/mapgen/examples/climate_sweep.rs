//! Measures the climate model on lone planets at forced orbits, asked
//! moons and drawn systems. It prints the acceptance numbers of the
//! climate pass (Earth, Venus and Mars, the Earth-like distance sweep with
//! its warm and frozen branches, Titan and Triton, the methane worlds and
//! the snowballs of 500 drawn systems by insolation).
//!
//! ```text
//! cargo run --release -p mapgen --example climate_sweep [sweep|moons|all]
//! ```
//!
//! `sweep` is section 1 (distance sweep, Solar System bodies), `moons`
//! section 3 (seeds 1 to 10 of `titan-like` and `triton-like`); with no
//! argument or `all` every section runs, which takes a few minutes. The
//! planet census (`planet_census 500`) prints the class mix and the same
//! snowball and methane checks.
use std::collections::BTreeMap;

use mapgen::planet::{self, PlanetModel, SurfaceLiquid, interim, outgassed};
use mapgen::request::WorldRequest;
use mapgen::star::StarInfo;
use mapgen::system::{BodyKind, SystemRequest, WholeSystemRequest, draw};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// An insolation bin of the census: label, bounds and its plate planets
/// (temperature, liquid water, frozen, CO2, snowball).
type Bin<'a> = (&'a str, f64, f64, Vec<(f64, bool, bool, f64, bool)>);

fn med(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

fn mode(v: &[String]) -> String {
    let mut m: BTreeMap<&str, u32> = BTreeMap::new();
    for s in v {
        *m.entry(s).or_default() += 1;
    }
    m.iter()
        .map(|(k, n)| format!("{k}x{n}"))
        .collect::<Vec<_>>()
        .join("/")
}

struct Row {
    teq: f64,
    s: f64,
    albedo: f64,
    t: f64,
    p: f64,
    g: [f64; 7],
    out_co2: f64,
    flux: f64,
    volc: f64,
    cls: String,
    liquid: String,
    regime: String,
    base: String,
    cloud: String,
    haze: f64,
    cover: f64,
    sky_green: f64,
    regulated: bool,
    snowball: bool,
}

fn row(model: &PlanetModel, volc: f64, s: f64) -> Option<Row> {
    let surf = model.surface.as_ref()?;
    let sky = interim::sky_of(model);
    let g = &surf.gases;
    let out = outgassed(&model.inputs, &surf.water, surf.regime, &model.interior);
    let teq = surf.climate.equilibrium_k;
    Some(Row {
        teq,
        s,
        albedo: surf.climate.bond_albedo,
        t: surf.climate.temperature_k,
        p: g.total(),
        g: [g.co2, g.n2, g.h2o, g.ch4, g.so2, g.nh3, g.o2],
        out_co2: out.co2,
        flux: model.heat.flux_w_m2(model.interior.radius_earth),
        volc,
        cls: model.class.key().into(),
        liquid: format!("{:?}", surf.climate.liquid),
        regime: format!("{:?}", surf.regime),
        base: interim::base_of(model).into(),
        cloud: format!("{:?}", sky.clouds),
        haze: f64::from(sky.haze),
        cover: f64::from(sky.cloud_cover),
        sky_green: f64::from(sky.greenhouse),
        regulated: surf.climate.regulated,
        snowball: surf.climate.snowball,
    })
}

/// A lone planet of `arch` at `au`; `snowball` overrides its snowball
/// pin (`Some(true)`: the frozen branch wherever it exists).
fn lone(arch: &str, au: f32, seed: u32, snowball: Option<bool>) -> Option<Row> {
    let req = WorldRequest {
        planet: Some(arch.into()),
        snowball,
        seed: Some(seed),
        star: Some("G2V".into()),
        orbit_au: Some(au),
        age_gyr: Some(4.57),
        alone: Some(true),
        width: Some(64),
        height: Some(32),
        ..WorldRequest::default()
    };
    let r = req
        .resolve(&mut StdRng::seed_from_u64(u64::from(seed)))
        .ok()?;
    let p = &r.params;
    let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
    row(
        r.model(),
        f64::from(p.volcanic_intensity),
        f64::from(star.insolation),
    )
}

fn summarize(label: &str, rows: &[Row]) {
    if rows.is_empty() {
        println!("{label}: none");
        return;
    }
    let m = |f: &dyn Fn(&Row) -> f64| {
        let mut v: Vec<f64> = rows.iter().map(f).collect();
        let (lo, hi) = (
            v.iter().cloned().fold(f64::MAX, f64::min),
            v.iter().cloned().fold(f64::MIN, f64::max),
        );
        (med(&mut v), lo, hi)
    };
    let (t, tl, th) = m(&|r| r.t);
    let (p, pl, ph) = m(&|r| r.p);
    let gs: Vec<f64> = (0..7).map(|i| m(&|r| r.g[i]).0).collect();
    println!(
        "{label:<29} Teq {:.0} A {:.2} S {:.2} | T {t:.0} [{tl:.0}-{th:.0}] P {p:.3} [{pl:.3}-{ph:.3}] | co2 {:.3e} n2 {:.3e} h2o {:.2e} ch4 {:.2e} so2 {:.2e} nh3 {:.1e} o2 {:.2e} | outCO2 {:.2e} reg {} | flux {:.3} volc {:.2} | {} | {} | base {} cloud {} haze {:.2} cover {:.2} gh {:.1} | {}",
        m(&|r| r.teq).0,
        m(&|r| r.albedo).0,
        m(&|r| r.s).0,
        gs[0],
        gs[1],
        gs[2],
        gs[3],
        gs[4],
        gs[5],
        gs[6],
        m(&|r| r.out_co2).0,
        rows.iter().filter(|r| r.regulated).count(),
        m(&|r| r.flux).0,
        m(&|r| r.volc).0,
        mode(&rows.iter().map(|r| r.cls.clone()).collect::<Vec<_>>()),
        mode(&rows.iter().map(|r| r.liquid.clone()).collect::<Vec<_>>()),
        mode(&rows.iter().map(|r| r.base.clone()).collect::<Vec<_>>()),
        mode(&rows.iter().map(|r| r.cloud.clone()).collect::<Vec<_>>()),
        m(&|r| r.haze).0,
        m(&|r| r.cover).0,
        m(&|r| r.sky_green).0,
        mode(&rows.iter().map(|r| r.regime.clone()).collect::<Vec<_>>()),
    );
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    let all = which == "all";
    if all || which == "sweep" {
        println!("== 1. distance sweep (lone, G2V, 4.57 Gyr, seeds 1-5)");
        let mut cases: Vec<(&str, f32)> = [
            0.7, 0.8, 0.9, 0.95, 1.0, 1.2, 1.4, 1.5, 1.6, 1.67, 1.8, 2.0, 2.5, 5.0, 10.0,
        ]
        .iter()
        .map(|&a| ("earth-like", a))
        .collect();
        cases.push(("venus-like", 0.72));
        for a in [1.52, 5.0, 10.0] {
            cases.push(("mars-like", a));
        }
        for (arch, au) in cases {
            let rows: Vec<Row> = (1..=5).filter_map(|s| lone(arch, au, s, None)).collect();
            summarize(&format!("{arch} {au}"), &rows);
            // The frozen branch, where the climate also has one.
            let frozen: Vec<Row> = (1..=5)
                .filter_map(|s| lone(arch, au, s, Some(true)))
                .filter(|r| r.snowball)
                .collect();
            if !frozen.is_empty() {
                summarize(&format!("  frozen {arch} {au}"), &frozen);
            }
        }
    }
    if all || which == "moons" {
        println!("\n== 3. asked moons (seeds 1-10)");
        for arch in ["titan-like", "triton-like"] {
            for seed in 1..=10u32 {
                let req = SystemRequest::System(WholeSystemRequest {
                    star: Some("G2V".into()),
                    metallicity: Some(0.0),
                    planets: Some(BTreeMap::from([(arch.to_string(), 1)])),
                    seed: Some(seed),
                    ..WholeSystemRequest::default()
                });
                let Ok(sys) = draw(&req, &mut StdRng::seed_from_u64(u64::from(seed))) else {
                    println!("{arch} {seed}: draw failed");
                    continue;
                };
                for b in sys.bodies.iter().filter(|b| {
                    b.kind == BodyKind::Moon && b.params.archetype.as_deref() == Some(arch)
                }) {
                    let m = planet::body_model(&sys, b).unwrap();
                    let s = m.surface.as_ref().unwrap();
                    let sky = interim::sky_of(&m);
                    println!(
                        "{arch} seed {seed}: Teq {:.1} T {:.1} P {:.3e} n2 {:.3e} ch4 {:.3e} co2 {:.1e} liquid {:?} class {} base {} haze {:.2} clouds {:?}",
                        s.climate.equilibrium_k,
                        s.climate.temperature_k,
                        s.gases.total(),
                        s.gases.n2,
                        s.gases.ch4,
                        s.gases.co2,
                        s.climate.liquid,
                        m.class.key(),
                        interim::base_of(&m),
                        sky.haze,
                        sky.clouds
                    );
                }
            }
        }
    }
    if all || which == "census" {
        let mut hydro = (0u32, f64::MAX);
        let mut bins: Vec<Bin> = vec![
            (">1.5", 1.5, f64::MAX, vec![]),
            ("1.1-1.5", 1.1, 1.5, vec![]),
            ("0.9-1.1", 0.9, 1.1, vec![]),
            ("0.7-0.9", 0.7, 0.9, vec![]),
            ("0.5-0.7", 0.5, 0.7, vec![]),
            ("0.36-0.5", 0.36, 0.5, vec![]),
            ("0.2-0.36", 0.2, 0.36, vec![]),
            ("<0.2", 0.0, 0.2, vec![]),
        ];
        println!("\n== 4. methane-rich surfaced planets (CH4 > 0.01 bar), seeds 1-500");
        let mut surfaced = 0u32;
        for seed in 1..=500u32 {
            let req = SystemRequest::System(WholeSystemRequest {
                seed: Some(seed),
                ..WholeSystemRequest::default()
            });
            let sys = draw(&req, &mut StdRng::seed_from_u64(u64::from(seed))).unwrap();
            for b in sys.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
                let m = planet::body_model(&sys, b).unwrap();
                let Some(s) = &m.surface else { continue };
                surfaced += 1;
                let p = &b.params;
                let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
                let sol = f64::from(star.insolation);
                if s.gases.ch4 > 0.01 {
                    println!(
                        "seed {seed} {} a {:.2} Teq {:.0} T {:.0} P {:.3} ch4 {:.3} liquid {:?} class {}",
                        b.id,
                        p.orbit_au,
                        s.climate.equilibrium_k,
                        s.climate.temperature_k,
                        s.gases.total(),
                        s.gases.ch4,
                        s.climate.liquid,
                        m.class.key()
                    );
                }
                if s.climate.liquid == SurfaceLiquid::Hydrocarbons {
                    hydro.0 += 1;
                    hydro.1 = hydro.1.min(s.gases.ch4);
                }
                if s.regime == mapgen::geology::Regime::Plates {
                    for bin in bins.iter_mut() {
                        if sol >= bin.1 && sol < bin.2 {
                            bin.3.push((
                                s.climate.temperature_k,
                                s.climate.liquid == SurfaceLiquid::Water,
                                s.climate.temperature_k < 273.15,
                                s.gases.co2,
                                s.climate.snowball,
                            ));
                        }
                    }
                }
            }
        }
        println!(
            "surfaced planets {surfaced}; Hydrocarbons liquid: {} planets, min CH4 {:.4}",
            hydro.0, hydro.1
        );
        println!("\n== 5. plate planets by insolation");
        for (label, _, _, v) in &mut bins {
            let n = v.len();
            let mut t: Vec<f64> = v.iter().map(|x| x.0).collect();
            let mut c: Vec<f64> = v.iter().map(|x| x.3).collect();
            let w = v.iter().filter(|x| x.1).count();
            let f = v.iter().filter(|x| x.2).count();
            let snowballs = v.iter().filter(|x| x.4).count();
            println!(
                "S {label:<9} n {n:>4} medT {:.0} liquidWater {:.0}% frozen(<273K) {:.0}% snowball {snowballs} medCO2 {:.2e} bar",
                med(&mut t),
                100.0 * w as f64 / n.max(1) as f64,
                100.0 * f as f64 / n.max(1) as f64,
                med(&mut c)
            );
        }
    }
}
