# Tide spheres in the planet view: design

Decided with the user on 2026-11-17 (this conversation).

## Intent

The planet view already draws the Hill sphere of the host round which a
body's moons can orbit (`setHillSphere` in
`web/src/lib/viewer/PlanetView.ts:2650`, toggled by "Sphère de Hill" in
`web/src/components/Viewer.svelte:1911`). The user wants to show the
host's tides the same way: as nested spheres, like Russian dolls, each one
the boundary of a tidal regime, with the Hill sphere as the outermost
doll. Looking at a moon, one should see at a glance whether it sits where
its planet would tear it apart, heat it or lock it, and why.

The Hill sphere is itself a tidal boundary (where the star's tide beats
the planet's gravity), so all spheres go under one name: **Marées**
(**Tides** in English).

## Goals

- Four nested spheres centred on the host, in the planet view (the globe)
  only, from outside in:
  1. **Hill sphere**: as today, the host's, at periapsis.
  2. **Locking sphere**: inside it, the host's tides have despun the body
     within the system's age.
  3. **Heating sphere**: inside it, the tidal heat flux of the body
     exceeds its radiogenic heat flux.
  4. **Roche sphere**: the fluid Roche limit; inside it, a fluid body of
     the body's density is torn apart.
- Every sphere is drawn like today's Hill sphere: a grid and a faint
  shell that lights towards its limb, each in its own colour.
- The "Sphère de Hill" toggle becomes one toggle **Marées** with one
  square per sphere, on the existing `parted` pattern
  (`Viewer.svelte:1819`, as for the sky labels and the small bodies).
- **All the physics is in Rust.** One function in `mapgen` computes the
  four radii, the server adds them to the system response at read time,
  and TypeScript only converts km to the view's units and draws. The
  Hill sphere moves to that function too.

## Non-goals

- The star view: its spheres would be too small to see. It keeps its
  Hill circles (see the builder's calls below about where their radius
  comes from).
- Rigid Roche limit, circularisation radius, or any other sphere.
- Glyphs of the tidal field's direction (stretched test spheres).
- Drawing the zones of every moon of a planet at once.

## Anti-goals

- Tidal physics recomputed in TypeScript (`hillRadiusKm` in
  `web/src/lib/systemSky.ts:372` is the kind of code this removes).
- A change to the stored `system.json`, a `SYSTEM_FORMAT_VERSION` bump,
  or any change to the generated worlds: the radii are derived at read
  time, so systems already stored get the spheres without being
  regenerated.

## Approaches

Considered for where the radii are computed:

- **A. Rust, computed by the server at read time (chosen).** One
  function in `mapgen` reuses the generator's formulas and constants;
  the server adds its result to the system response, ts-rs exports the
  type. One source for the physics (Q, k₂, rigidity, k₂/Q), and stored
  systems work unchanged. Costs a change in Rust, the server and the
  types.
- **B. Written by the generator into `system.json`.** Same result for
  the viewer, but stored systems lack the spheres until regenerated, and
  the format version goes up. Dropped.
- **C. TypeScript, like `hillRadiusKm` today.** No Rust change, but the
  Love number, rigidity and Q calibrations would be copied and drift the
  day a science fix changes them in Rust. Dropped.

The user picked A and asked that Hill go through the same function, so
that TypeScript computes as little as it can.

## Which body, which host

The spheres belong to **the body shown** (decided with the user): their
radii depend on the body that feels the tide (its density, mass, radius,
eccentricity, composition).

- **A moon shown**: centred on its host planet. Hill is the planet's
  Hill sphere (as today); Roche, locking and heating are the moon's own,
  round that planet.
- **A planet shown** (no host): centred on the planet itself. Hill is
  its own Hill sphere (as today). The other spheres are those of a
  typical satellite, since no moon is selected:
  - Roche: two spheres would muddle the square; use one, for rock
    (builder's call: rock density `3.0` g/cm³, or the density of the
    planet's largest moon if it has moons; pick one and document it);
  - locking: a Moon-like satellite (the Moon's mass, radius, density);
  - heating: hidden (a typical satellite has no eccentricity).
- "Has a host" is the rule `hillSphere` uses today
  (`web/src/lib/systemSky.ts:381`): the body has a `parent`. The primary
  of the Hill sphere is the host's own `parent` if any, else the star
  (`star.info.mass`).
- A sphere is hidden, and its square too, when its radius is absent
  (heating on a circular orbit, `e` below about `1e-4`) or not larger
  than the host's radius.

## Physics

Every radius comes from a formula the generator already uses; none is
new science. Where the generator solves for a time or a power at a given
distance, the new function inverts it for the distance, analytically:

- **Hill**: `a (1 - e) cbrt(m / 3M)` at periapsis, the formula of
  `hillRadiusKm` today (`crates/mapgen/src/rings.rs:858` `hill_radii`
  has no eccentricity term: keep the periapsis version, the one drawn
  today). `M` is the host's primary: the star, or the planet's own
  parent if any.
- **Roche**: `fluid_roche(host_density, body_density)`
  (`crates/mapgen/src/rings.rs:236`), in host radii.
- **Locking**: the despinning time `despin_gyr`
  (`crates/mapgen/src/moons.rs:829`) goes as `a⁶` (the Love number does
  not depend on `a`), so the radius where it equals the system's age
  (`star.info.age_gyr`) is `a₀ (age / t(a₀))^(1/6)` for any `a₀`. Use
  the moon inertia `0.4` for a moon, as the generator does
  (`moons.rs:1983`). `despin_gyr` already fixes its assumptions (first
  day 12 h, `Q = 100`, the body's own Love number from `love_number`,
  capped at 0.3): keep them, so the sphere agrees with the `locked` flag
  the generator stored. The typical satellite of a planet shown goes
  through the same function with the Moon's mass, radius and density.
- **Orbit**: `a` and `e` are the stored orbital elements of the body
  (`Body.orbit.semi_major_axis_km`, `eccentricity`), and for Hill those
  of the host round its primary.
- **Heating**: `tidal_power_w` (`crates/mapgen/src/planet/heat.rs:70`)
  goes as `a^(-15/2)` at fixed `e`, so the radius where it equals the
  body's radiogenic power is found in closed form too. Inputs:
  - `e`: the body's orbital eccentricity;
  - `k₂/Q`: `heat::k2_over_q(envelope, water)` from the body's model
    when it has one; for a moon without a model, `ICY_K2_OVER_Q` for an
    `icy-moon` and `ROCKY_K2_OVER_Q` for a `rocky-moon` (`body_type`);
  - radiogenic power: `model.heat.radiogenic_w` when stored; for a moon
    without a model, `heat::radiogenic_w` on its silicate mass (its rock
    share from `physical.bulk`, else the body type's), at the system's
    age, with the default abundance.

  Comparing powers is comparing fluxes, since both cross the same
  surface.

Calibration checks the tests should hold, on hand-built fixtures with
the real bodies' numbers (not generated systems), within about 10 %
where a value is given:

- Earth's Hill radius about 1.5 million km (235 Earth radii); Jupiter's
  about 53 million km.
- The Moon lies inside Earth's locking sphere (it is locked); Io,
  Europa, Ganymede and Callisto inside Jupiter's.
- Io lies inside Jupiter's heating sphere (tidal heat ~1e14 W, far above
  its radiogenic heat).
- Saturn's fluid Roche limit for ice about 2.2 Saturn radii (the test at
  `rings.rs:925` gives the expected values).

## Server and types

- A new function in `mapgen` (builder's call where: a new
  `crates/mapgen/src/tides.rs` fits) takes the system and a body id and
  returns `Option<TideZones>`: the host's id and the four radii in km,
  each `Option<f64>`.
- The server adds it at read time, as a map of its own next to the
  existing read-time `extras`: `tides: BTreeMap<String, TideZones>`
  keyed by body id, on `SystemDetail` (`GET /api/systems/{id}`,
  `crates/server/src/api.rs:488`). Not on `BodyExtras`:
  `body_extras` (`crates/server/src/app.rs:909`) skips the bodies
  without a model, and small moons, which have none, need their spheres.
  Every body with a host or a primary gets an entry. (Decided while
  planning: `ResolvedSystem` stays as it is, since only the form's
  preview reads it and it draws no spheres.)
- `cargo test` regenerates `web/src/lib/api/types`; CI checks they are
  committed.

## Viewer

- `hillRadiusKm` and `hillSphere` in `systemSky.ts` go; the viewer reads
  the host id and the radii from `extras[shown].tides`. `hillReach`
  stays (camera geometry, not physics) and is applied to the largest
  sphere drawn.
- `PlanetView.setHillSphere` becomes a method that takes a list of
  spheres (centre, radius in radii of the body shown, colour). Every
  sphere has its grid and its limb-lit shell, as today. Centre and units
  follow today's `hillDrawn` (`Viewer.svelte:1561`): from a moon, the
  host planet's actual position in the moon's sky (`sysSky.bodies[k]
  .position`); from a planet, the origin; radii in radii of the body
  shown. All spheres share the centre, so they share today's depth
  handling (held at the far plane, the globe hides them).
- Colours (builder may tune them for contrast): Hill keeps its turquoise
  `(0.55, 0.85, 0.8)`; locking blue, heating orange, Roche red.
- The toggle: `parted({ key: 'tides', label: Marées … }, [hill, locking,
  heating, roche])`. Each square's help gives the sphere's meaning, its
  radius in km and in host radii, like `hillSphereHelp` today. A square
  is shown only when its sphere exists. With a single sphere available,
  `parted` already collapses to a plain toggle.
- Saved settings: the saved `hillSphere` key (`Viewer.svelte:205, :541`)
  stays the Hill square's, so a saved choice carries over; the other
  squares get keys of their own.
- i18n in `web/src/lib/i18n/en.ts` and `fr.ts`.

## Left to the builder

- Where the Rust function lives and how it is split.
- The typical satellite's Roche density for a planet shown: decided
  while planning, a fixed 3.0 g/cm³.
- Decided while planning: the star view's Hill circles stay in
  TypeScript for now (a backlog issue). Its giants are not keyed by body
  id. Originally left open: whether the star view's Hill circles (`hillRadiusAu`,
  `web/src/lib/viewer/starSystem.ts:391`, called at `StarView.ts:1209`)
  also read the server's Hill radius. Prefer it, in the spirit of the
  user's request, if the star view's giants can be matched to body ids
  without contortions; otherwise leave it and file an issue.
- Grid density and shell opacity per sphere, so that four nested grids
  stay readable from inside.
- Documentation: the viewer's section of `README.md` (find it with
  `grep -n '^#' README.md`), the API types' docs, and the changelog if
  the project keeps one.
