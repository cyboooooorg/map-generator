# Planet model PR 2a: trace gaps, wider draws, envelopes and giants

Decided with the user on 2026-11-21. First of the four pull requests of
the planet model's PR 2 ([split](2026-11-21-planet-model-pr2-split-design.md)).
Slice 1, #602. Closes #618.

## Intent

Make the composition draws aim for variety, give every planet its
hydrogen-helium envelope, and give giants and mini-Neptunes a radius and
a layered cross-section, so that PR 2c (escape) and 2d (class, public
`class_of`) have every body's bulk. Close the trace gaps PR 1 left.

## Goals

1. **Trace gaps of #617**, all in `crates/system/src/planet/`:
   - the water fraction becomes a drawn leaf;
   - the central pressure records every input it depends on;
   - the mantle's solid state is a constant of the engine, not derived;
   - the migration flag is drawn and recorded only inside the ice line;
   - the ice-line flag is computed once;
   - the k2/Q choice reads the water's state (water outside the
     structure is not icy) and the envelope fraction;
   - 355 K is no longer read as a steam criterion: it is only the upper
     edge of `science::water`'s domain. A water world hotter than that
     keeps its water outside the structure, recorded as "beyond the
     water law" rather than as steam, and its bulk stays consistent
     (the dry mass with the dry radius, the water mass apart) instead
     of mixing the whole mass with the dry radius. The radius of steam
     and supercritical water worlds moves to PR 2d, which has the
     climate their criterion needs (study decision 3).
2. **Wider draws (#618):** each composition draw is **uniform between
   published bounds** (log-uniform where the quantity spans decades),
   like the star draw: core mass fraction, dry water fraction,
   radiogenic abundance. The envelope fraction draw follows the same
   rule *(my call)*. The observed distributions are not kept behind a
   setting; their sourced parameters stay in the science page for the
   future galactic-map mode.
3. **Envelope fraction:** drawn for every planet from its own sub-seed,
   with bounds that depend on mass (no envelope below a sourced mass,
   mini-Neptune, ice-giant and gas-giant ranges), each bound sourced.
4. **Giants and mini-Neptunes (approach A):**
   - radius from published tabulated models: Fortney, Marley & Barnes
     2007 for giants, Lopez & Fortney 2014 for mini-Neptunes; the radius
     depends on core mass, age and irradiation;
   - the cross-section's layers come from the same model: the core
     integrated with `science::structure` (#617), the envelope filling
     the rest, the molecular to metallic hydrogen transition at a
     published pressure;
   - no hot-Jupiter inflation: it goes to #621 (study decision 1).

## Non-goals

- Integrated hydrogen-helium structure and cooling: #621 (T4).
- Escape of the envelope: 2c. The envelope drawn here is the primordial
  one; 2c applies the XUV loss.
- The class and public `class_of`: 2d.

## Approaches

For the draws: uniform between published bounds (chosen, consistent with
the star draw) against a mixture of observed and uniform (rejected: the
mixture's share would be an invented number).

For the giants: tabulated published models with a layered cross-section
(chosen) against a full hydrogen-helium integration (deferred to #621:
the radius needs the cooling history).

## Constraints

- No invented data: a study comes first, in
  `docs/superpowers/studies/2026-11-21-planet-model-2a/`, sourcing the
  bounds of each draw, the envelope bounds, the giant and mini-Neptune
  tables (with their size and how they are read) and the transition
  pressure: `draw-bounds.md` and `giants-and-envelopes.md`, each with
  the user's decisions. Missing science stops the work.
- The tables are ported as data with their provenance, read by
  `science` with no file access at run time (`include_bytes!` or
  constants), as the stellar tables are.
- One code path: the radius and the layer boundaries of a giant come
  from one computation.
- Every draw from its own `Seed::derive` sub-seed.

## Left to the builder

- Module names and how the tables are encoded.
- Tier placement of the new tests within the tier-1 budget.
