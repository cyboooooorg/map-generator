use super::*;

/// Definition: a CO₂ to thaw, 0.1 bar, for the shape of the law: any
/// positive value would do.
const EXTRA: Bar = Bar(0.1);

/// Definition: a volcanic intensity under Earth's, 0.1, for the shape of
/// the law.
const WEAK_VOLCANISM: f64 = 0.1;

/// Definition: the tolerance of an identity of the law, 1e-12.
const IDENTITY: f64 = 1e-12;

fn fraction(value: f64) -> Fraction {
    Fraction::new(value).unwrap()
}

#[test]
fn snowball_chance_follows_its_law() {
    let earth = fraction(EARTH_VOLCANIC_INTENSITY);
    // No CO₂ thaws it: frozen for all its history.
    assert_eq!(snowball_chance(None, earth).get(), 1.0);
    // Nothing outgasses: frozen for good, and not 0 / 0 (Review Focus 4).
    let dead = snowball_chance(Some(EXTRA), fraction(0.0)).get();
    assert!(!dead.is_nan());
    assert_eq!(dead, 1.0);
    assert_eq!(snowball_chance(None, fraction(0.0)).get(), 1.0);
    assert_eq!(snowball_chance(Some(Bar(0.0)), fraction(0.0)).get(), 1.0);
    // At Earth's volcanism the temperate phase lasts W.
    let d = EXTRA.0;
    let expected = d / (d + SNOWBALL_WARM_BAR.0);
    assert!((snowball_chance(Some(EXTRA), earth).get() - expected).abs() < IDENTITY);
    // It rises with the CO₂ to thaw and falls with the volcanism.
    let more = snowball_chance(Some(Bar(2.0 * d)), earth).get();
    assert!(more > expected);
    let weak = snowball_chance(Some(EXTRA), fraction(WEAK_VOLCANISM)).get();
    assert!(weak > expected);
    let full = snowball_chance(Some(EXTRA), fraction(1.0)).get();
    assert!(full < expected);
    // A thaw already reached needs no frozen time.
    assert_eq!(snowball_chance(Some(Bar(0.0)), earth).get(), 0.0);
}

/// Source: the share of its history Earth spent in dated snowball
/// states, 0.013 to 0.016 (Hoffman et al. 2017, *Sci. Adv.* 3,
/// e1600983, Table 1; derived in `docs/science/planet-atmospheres.md`,
/// "Calibration of `SNOWBALL_WARM_BAR` (refit)").
const HOFFMAN_SHARE: (f64, f64) = (0.013, 0.016);

/// Definition: a test input, the extra CO₂ to thaw that the tier-3 test
/// `earth_snowballs_as_often_as_hoffman_dates` measures on the reference
/// Earth at 1 AU (0.147930 bar, kept to the 0.1479 bar of the fit of
/// [`SNOWBALL_WARM_BAR`]). Not a fact of Earth: the tier-1 test checks
/// only the arithmetic of the law on it.
const EARTH_THAW_EXTRA: Bar = Bar(0.1479);

/// Definition: a test input, the reference Earth's recorded volcanic
/// intensity, 0.506064, that the same tier-3 test reads, as the fit of
/// [`SNOWBALL_WARM_BAR`] took it.
const EARTH_VOLCANISM: f64 = 0.506064;

#[test]
fn the_warm_bar_gives_earth_its_dated_share() {
    // The arithmetic of the law at the fit's inputs; that the solver
    // still measures them is the tier-3 test's.
    let chance = snowball_chance(Some(EARTH_THAW_EXTRA), fraction(EARTH_VOLCANISM)).get();
    assert!(
        (HOFFMAN_SHARE.0..=HOFFMAN_SHARE.1).contains(&chance),
        "{chance}"
    );
}
