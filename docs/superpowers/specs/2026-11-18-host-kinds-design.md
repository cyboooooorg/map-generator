# Host kinds: design

Issue #569, first step of the host-kinds and multiple-systems track
(#585). Approved by the user on 2026-11-18. Study:
`docs/analysis/17-host-stars.md` (sections 2.1, 3 and 4).

## Intent

The generator knows one kind of host: a main-sequence dwarf, O to M,
described by its spectral code alone (`star::values(code)`). Every later
step of #585 needs more: evolved hosts (#576), white dwarfs (#577),
neutron stars (#578, #579), brown dwarfs (#580), planetless showcases
(#581 to #584) and stellar companions (#570 onwards). This change adds
the data model they all build on, without adding the physics of any new
kind.

## Goals

- A `HostKind` and an extended spectral-type parser.
- A state entry point `(kind, mass, age)` and a `StarInfo` built from
  explicit values (also needed by the companions of #570).
- A host-dependent inner edge for orbits and a quiet-star branch in the
  star's activity.
- The kind stored in `system.json`, and the draw-weight table of the
  study's section 2.1 with its own seeded stream.

## Non-goals

- The physics of any new kind (tracks, cooling, heating channels): each
  kind's own issue, #576 to #583.
- Form and viewer changes: only dwarfs exist after this change.
- Deciding how quasi-stars are reached (form only, or a tiny weight):
  left to #583.

## Anti-goals

- No new kind drawn or offered with placeholder physics: a world must
  never show a white dwarf computed as a dwarf.
- No change to existing seeds: every system drawn today must come out
  identical, apart from the format number, the new field and the hosts
  whose inner edge moves (item 5).
- No change to the signature or meaning of `habitable_fluxes`,
  `StarInfo.teff_k` or `StarInfo.luminosity` (read by the climate of
  lot 3C-2, owned by another session).

## Approaches

The open choice was what the default random draw does once this change
lands.

- **A. Model only.** The draw is untouched; each kind's issue adds the
  kind to the draw and to the form. The weight table and the stream are
  designed piecemeal, issue by issue.
- **B. Every kind in the draw now**, with placeholder states. About 6 %
  of systems would become white dwarfs drawn and rendered as dwarfs
  until #577: incoherent worlds.
- **C. Full table, gated by availability** (chosen by the user). This
  change sets the whole table of section 2.1, but a kind is drawn, and
  offered by the parser, only once its physics has landed. After this
  change every new kind has a zero effective share; each kind's issue
  turns its own on.

## Design

1. **`HostKind`** (in `star.rs` or a new `host.rs`, builder's call):
   dwarf, subgiant, giant, white dwarf, neutron star, magnetar, brown
   dwarf, black hole, Wolf-Rayet, quasi-star. Each kind knows whether it
   is available; only `Dwarf` is, here. Each kind's issue flips its own.
2. **Extended `parse_type`**: recognises `G2V` and `G2` (dwarf), `G8IV`
   (subgiant), `K3III` (giant), `DA` and its subtypes (white dwarf), `L5`,
   `T7`, `Y0` (brown dwarf), `PSR` (neutron star), `MAG` (magnetar),
   `BH` (black hole), `WR` (Wolf-Rayet), `QS` (quasi-star), and returns
   the kind with the code. An unavailable kind is a clear error naming
   its issue ("white dwarfs are not modelled yet, see #577"), never a
   fake system. Existing callers that only handle dwarfs keep their
   behaviour.
3. **State entry point**: `host_state(kind, mass, age)` returns the
   values the rest of the code reads (temperature, luminosity, radius,
   mass). For a dwarf it finds the spectral code from the mass on Pecaut
   & Mamajek's sequence (the main sequence is age-independent in the
   model) and returns `values(code)`. Other kinds return an error until
   their issue. `values(code)` and its outside callers stay as they are.
4. **`StarInfo` from explicit values**: a constructor taking those
   values instead of a code, so a companion (#570) or a non-dwarf host
   can be described. `StarInfo::new` for a dwarf gives the same result
   as before.
5. **Inner edge**: `inner_edge_au(host)` is the largest of today's
   0.02 AU, the host's Roche limit for a rocky body (`tides::roche_km`,
   density 5.5 g/cm³) and two stellar radii (the closest known planets
   orbit at 2 to 3 stellar radii). Today a planet around an O star or a
   hot B star can be placed inside the star (O3V is 13.4 R☉, 0.062 AU,
   against an edge of 0.02 AU). Decided with the user on 2026-11-18:
   fix it here. Systems around O to A0 dwarfs (radius above 2.15 R☉,
   under 0.5 % of default draws) change; every dwarf from A1 to M stays
   byte-identical. `INNER_EDGE_AU` call sites (`system.rs`) use it.
6. **Quiet-star activity**: `star_activity` gets a branch chosen by the
   kind for hosts without a convective photosphere (white dwarfs,
   neutron stars, old brown dwarfs): no spots, no granulation, given
   rotation. Only unit tests reach it in this change.
7. **Draw**: `HOST_KIND_SHARES`, the per-kind weights of section 2.1
   with their sources, and a new stream `stream::HOST_KIND` numbered
   after `GALAXY`. While every non-dwarf kind is unavailable or has a
   zero share, the draw does not consume the stream, so seeds give the
   same systems.
8. **Format**: `SystemStar` gains a `kind` field; `SYSTEM_FORMAT_VERSION`
   goes to 12 (11 is taken by lot 3C-2 PR 2; the second to merge
   rebases and renumbers). `FORMAT_VERSION` (raw data, 30) is bumped
   only if the kind enters the raw data, for instance through
   `StarInfo` (`export/raw_data.rs` embeds it); the builder decides
   where the kind lives and states it in the plan. TypeScript bindings
   are regenerated.

## Proof

- Unit tests for the parser (every notation, unavailable kinds), the
  dwarf state against `values(code)`, the inner edge, the quiet branch
  and the draw's stream not being consumed.
- `world-coherence-checker` on a set of seeds, binary before and after:
  systems and worlds byte-identical apart from the format number and the
  new field.

## Coordination

- mapgen1 (lot 3C-2) is told before the pull request is opened, for the
  format numbers.
- Documentation: README sections on stars and on `system.json`,
  `docs/analysis/17-host-stars.md` follow-ups marked as started, and the
  issue comments.

## Left to the builder

- Module layout (`star.rs` or a new `host.rs`) and where the kind lives
  (`SystemStar` only, or `StarInfo` too).
- The exact parser grammar for white-dwarf subtypes and brown-dwarf
  types, within the notations above.
