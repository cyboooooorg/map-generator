//! The stars lighting a planet at its orbit: bolometric flux, equilibrium
//! temperature, and the X-ray, extreme-ultraviolet and Lyman α light.
//!
//! One source for now, the host star: the star's high-energy history is
//! computed once ([`light`]) and serves every planet around it.

use crate::SystemError;
use crate::star::Star;
use science::orbit::{circular_flux, equilibrium, mean_flux_factor};
use science::units::{AstronomicalUnit, EarthFlux, Fraction, WattPerSquareMetre};
use science::xuv::{Exposure, History, LYMAN_ALPHA, XUV, history};
use timeline::{BandLight, Code, StellarSource, StellarSources, Trace, Traced};

/// Cause codes of the source's nodes.
mod codes {
    pub(crate) const FLUX: &str = "system.draw.flux";
    pub(crate) const EQUILIBRIUM: &str = "system.draw.equilibrium";
    pub(crate) const XUV_FLUX: &str = "system.draw.xuv-flux";
    pub(crate) const XUV_FLUENCE: &str = "system.draw.xuv-fluence";
    pub(crate) const LYMAN_ALPHA_FLUX: &str = "system.draw.lyman-alpha-flux";
    pub(crate) const LYMAN_ALPHA_FLUENCE: &str = "system.draw.lyman-alpha-fluence";
}

/// The high-energy history of a star, built once and shared by every
/// planet around it.
pub(crate) struct Light {
    history: History,
}

/// The [`Light`] of `star`.
pub(crate) fn light(star: &Star) -> Result<Light, SystemError> {
    let state = &star.state;
    Ok(Light {
        history: history(
            *state.mass.value(),
            *state.metallicity.value(),
            *star.age.value(),
        )?,
    })
}

/// What a planet at semi-major axis `a` and eccentricity `e` receives from
/// `star`: one source. The mean flux is the circular flux times
/// [`mean_flux_factor`]; the equilibrium temperature carries the
/// eccentricity through its own factor, from the circular flux.
pub(crate) fn sources(
    star: &Star,
    light: &Light,
    a: &Traced<AstronomicalUnit>,
    e: &Traced<Fraction>,
    trace: &mut Trace,
) -> Result<StellarSources, SystemError> {
    let state = &star.state;
    let circular = circular_flux(*state.luminosity.value(), *a.value());
    let orbit = [a.node(), e.node(), state.luminosity.node()];
    let flux = trace.record(
        Code::of(codes::FLUX),
        EarthFlux(circular.0 * mean_flux_factor(*e.value())),
        &orbit,
    )?;
    let equilibrium = trace.record(
        Code::of(codes::EQUILIBRIUM),
        equilibrium(circular, *e.value()),
        &orbit,
    )?;
    let band_causes = [
        flux.node(),
        state.mass.node(),
        state.metallicity.node(),
        star.age.node(),
    ];
    let watts = WattPerSquareMetre::from(*flux.value());
    let mut band = |exposure: Exposure, flux_code: &'static str, fluence_code: &'static str| {
        Ok::<_, SystemError>(BandLight {
            flux: trace.record(Code::of(flux_code), exposure.flux(watts), &band_causes)?,
            fluence: trace.record(
                Code::of(fluence_code),
                exposure.fluence(watts),
                &band_causes,
            )?,
        })
    };
    let xuv = band(
        light.history.exposure(XUV),
        codes::XUV_FLUX,
        codes::XUV_FLUENCE,
    )?;
    let lyman_alpha = band(
        light.history.exposure(LYMAN_ALPHA),
        codes::LYMAN_ALPHA_FLUX,
        codes::LYMAN_ALPHA_FLUENCE,
    )?;
    Ok(StellarSources::new(vec![StellarSource {
        flux,
        equilibrium,
        effective_temperature: state.effective_temperature,
        mass: state.mass,
        luminosity: state.luminosity,
        xuv,
        lyman_alpha,
    }])?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::star::star;
    use science::units::{Dex, Gyr, SolarMass};
    use science::xuv::EARTH_XUV_FLUENCE;
    use timeline::{Origin, Provenance, Seed};

    fn sun(trace: &mut Trace) -> Star {
        star(
            Some(SolarMass(1.0)),
            Some(Dex(0.0)),
            Some(Gyr(4.5673)),
            Seed::new(1),
            trace,
        )
        .unwrap()
    }

    fn orbit(a: f64, e: f64, trace: &mut Trace) -> (Traced<AstronomicalUnit>, Traced<Fraction>) {
        (
            trace
                .leaf(Code::of("test.a"), AstronomicalUnit(a), Provenance::Drawn)
                .unwrap(),
            trace
                .leaf(
                    Code::of("test.e"),
                    Fraction::new(e).unwrap(),
                    Provenance::Drawn,
                )
                .unwrap(),
        )
    }

    fn one(star: &Star, a: f64, e: f64, trace: &mut Trace) -> StellarSource {
        let light = light(star).unwrap();
        let (a, e) = orbit(a, e, trace);
        sources(star, &light, &a, &e, trace)
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .clone()
    }

    #[test]
    fn sun_at_one_au_gives_earth() {
        let mut trace = Trace::new();
        let star = sun(&mut trace);
        let l = star.state.luminosity.value().0;
        let source = one(&star, 1.0, 0.0, &mut trace);
        assert!((source.flux.value().0 - l).abs() < 1e-9);
        assert!((source.equilibrium.value().0 - 255.0 * l.powf(0.25)).abs() < 0.01);
        let xuv = source.xuv.flux.value().0;
        let expected = 4.6e-3 * l;
        assert!((xuv / expected - 1.0).abs() < 0.01, "{xuv} vs {expected}");
        let fluence = source.xuv.fluence.value().0;
        let expected = EARTH_XUV_FLUENCE.0 * l;
        assert!(
            (fluence / expected - 1.0).abs() < 0.01,
            "{fluence} vs {expected}"
        );
    }

    #[test]
    fn eccentricity_raises_the_flux() {
        let mut trace = Trace::new();
        let star = sun(&mut trace);
        let round = one(&star, 1.0, 0.0, &mut trace);
        let eccentric = one(&star, 1.0, 0.3, &mut trace);
        let ratio = eccentric.flux.value().0 / round.flux.value().0;
        assert!((ratio - 1.0483).abs() < 1e-4, "{ratio}");
        assert!(eccentric.equilibrium.value().0 < round.equilibrium.value().0);
    }

    #[test]
    fn an_a_star_has_no_xuv_but_lyman_alpha() {
        let mut trace = Trace::new();
        let star = star(
            Some(SolarMass(2.5)),
            Some(Dex(0.0)),
            Some(Gyr(0.3)),
            Seed::new(1),
            &mut trace,
        )
        .unwrap();
        let source = one(&star, 5.0, 0.0, &mut trace);
        assert_eq!(source.xuv.flux.value().0, 0.0);
        assert_eq!(source.xuv.fluence.value().0, 0.0);
        assert!(source.lyman_alpha.flux.value().0 > 0.0);
        assert!(source.lyman_alpha.fluence.value().0 > 0.0);
    }

    #[test]
    fn the_flux_node_cites_the_orbit_and_the_star() {
        let mut trace = Trace::new();
        let star = sun(&mut trace);
        let light = light(&star).unwrap();
        let (a, e) = orbit(1.0, 0.1, &mut trace);
        let list = sources(&star, &light, &a, &e, &mut trace).unwrap();
        let source = list.iter().next().unwrap();
        let causes = |id| match &trace.node(id).unwrap().origin {
            Origin::Derived { from } => from.clone(),
            _ => panic!("not computed"),
        };
        let flux = causes(source.flux.node());
        for cause in [a.node(), e.node(), star.state.luminosity.node()] {
            assert!(flux.contains(&cause));
        }
        let band = causes(source.xuv.fluence.node());
        for cause in [
            source.flux.node(),
            star.state.mass.node(),
            star.state.metallicity.node(),
            star.age.node(),
        ] {
            assert!(band.contains(&cause));
        }
        assert_eq!(source.mass, star.state.mass);
    }
}
