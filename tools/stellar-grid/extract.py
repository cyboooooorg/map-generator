"""Extract the stellar tables of `crates/science/data` from the YREC grid run.

Usage:
    python extract.py --grid OUT_DIR --bhac15 FILE --dartmouth DIR --b23 FILE \
        [--data DIR] [--workers N]

--grid       directory of the `.track` and `.store` files of `run_grid.py`
--bhac15     `BHAC15_tracks+structure` of Baraffe et al. 2015
--dartmouth  directory of the Dartmouth `m0NNfeh*afep0.trk` tracks (Kiauhoku)
--b23        a B23 standard solar model structure file (`struct+nu_SF3_*.dat`)
--data       where to write the tables (default: crates/science/data)

Writes `stellar-tracks.tsv`, `stellar-profiles.tsv` and
`stellar-low-mass.tsv`, and prints the checks that `STELLAR.md` quotes:
the solar model against B23, the mass-step interpolation error above
2 Msun, the 0.3 Msun seam and the file sizes. See `STELLAR.md` for the
formats and how every number is derived.
"""

import argparse
import math
import os
import re
import sys
from multiprocessing import Pool

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))

FEHS = [(-1.0, "-1p0"), (-0.5, "-0p5"), (0.0, "+0p0"), (0.5, "+0p5")]
GRID_MASSES = [m / 10 for m in range(3, 81)]
# Central hydrogen of the profile snapshots, as a fraction of the ZAMS value.
SNAPSHOT_XC = [1.0, 0.9, 0.7, 0.5, 0.3, 0.1, 0.01]
# Rows of a track after its ZAMS row, evenly spaced in central hydrogen.
TRACK_ROWS = 40
TAIL_XC = [0.01, 0.003, 0.001]
# Profile radii (fraction of the photospheric radius), denser near the
# surface where density and pressure fall by decades.
R_GRID = np.round(
    np.concatenate(
        [
            np.arange(0.0, 0.9, 0.025),
            np.arange(0.9, 0.98, 0.01),
            [0.98, 0.985, 0.99, 0.995, 0.998, 1.0],
        ]
    ),
    6,
)
# Layers thinner than this (fraction of R) are below the table's radial
# resolution and are merged into their inner neighbour.
MIN_LAYER = 1e-3
# Mass steps tried above 2 Msun: each splits 2-8 Msun evenly and puts its
# mid-points on the 0.1 Msun grid.
STEPS = [0.2, 0.4, 0.6, 1.0, 1.2, 2.0]
# Steps kept (see STELLAR.md): tracks by 0.2 Msun above 2 Msun; profiles
# by 0.1 Msun up to 2.6 Msun (the envelope convection zone retreats
# between 2.0 and 2.5 Msun), then these masses.
TRACK_STEP = 0.2
PROFILE_HIGH = [2.6, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
# Ages (Gyr) kept from the Baraffe tracks, nearest row, plus the last row.
LOW_MASS_AGES = [0.05, 0.07, 0.1, 0.15, 0.2, 0.3, 0.5, 0.7, 1, 1.5, 2, 3, 5, 7, 10]
LOW_MASSES = [0.08, 0.09, 0.1, 0.11, 0.13, 0.15, 0.17, 0.2, 0.3, 0.4]
DARTMOUTH_MASSES = [0.15, 0.2, 0.25, 0.3]
DARTMOUTH_FEHS = [(-1.0, "m10"), (-0.5, "m05"), (0.5, "p05")]
SOLAR_AGE = 4.57  # Gyr, Bahcall et al. 1995 / B23
B23_RADII = [0.05, 0.1, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95]
RSUN_CM = 6.957e10  # IAU 2015 B3 nominal solar radius


def g(x):
    return f"{x:.6g}"


def name(mass, feh_tag):
    return f"Grid_m{int(mass)}p{int(round(mass * 10)) % 10}feh{feh_tag}GS98_std25"


# ---------------------------------------------------------------- YREC


def read_track(path):
    cols, rows = None, []
    for line in open(path):
        s = line.split()
        if not s or line.startswith("#"):
            continue
        if s[0] == "Step":
            cols = s
            continue
        rows.append([float(x) for x in s[: len(cols)]])
    a = np.array(rows)
    t = {c: a[:, i] for i, c in enumerate(cols)}
    t["Step"] = t["Step"].astype(int)
    return t


def zams_index(t):
    """The ZAMS (`Definition:` in STELLAR.md): the first model whose nuclear
    power is >= 99 % of L and whose radius has stopped shrinking (the next
    model is not smaller).

    The radius condition steps over the out-of-equilibrium burning of C12
    on the approach to the main sequence, which gives >= 99 % for a few
    models while the star still contracts. Nuclear power alone does not
    mark the main sequence after that: CN and He3 approach equilibrium for
    a long time, and L_nuc / L wavers around 0.97-1.05 (at 0.4 Msun and
    +0.5 it touches 0.990 at 4.6 Gyr).
    """
    nuc = t["ppI_lsun"] + t["ppII_lsun"] + t["ppIII_lsun"] + t["CNO_lsun"] + t["3a_lsun"]
    share = nuc / 10 ** t["LogL_lsun"]
    lr = t["LogR_rsun"]
    hit = np.nonzero((share[:-1] >= 0.99) & (lr[1:] >= lr[:-1]))[0]
    assert len(hit), "no main sequence found"
    return int(hit[0])


# Fortran output can fuse two fields ("3.97-10.03") and drop the E of a
# three-digit exponent ("0.67-101"): split such lines by pattern.
TOKEN = re.compile(r"[TF](?=\s|$)|[+-]?\d+\.\d*(?:[EeDd][+-]?\d+|[+-]\d{3}(?![\d.]))?|[+-]?\d+")
BARE_EXP = re.compile(r"^([+-]?\d+\.\d*)([+-]\d{3})$")


def num(tok):
    try:
        return float(tok)
    except ValueError:
        return float(BARE_EXP.sub(r"\1E\2", tok))


def read_store(path, first_step):
    """Stored models from `first_step` on: {step: dict of shell arrays}."""
    cols, raw, width = None, {}, None
    for line in open(path):
        if line.startswith("MODEL"):
            cols = {c: i for i, c in enumerate(line.split())}
            continue
        head = line[:12].split()
        if not head or not head[0].isdigit() or int(head[0]) < first_step:
            continue
        tok = line.split()
        if width is None:
            width = len(tok)
        if len(tok) != width:
            tok = TOKEN.findall(line)
            assert len(tok) == width, (path, line[:80])
        raw.setdefault(int(head[0]), []).append(tok)
    want = ["MASS", "RADIUS", "PRESSURE", "TEMPERATURE", "DENSITY", "E_NUC"]
    out = {}
    for step, rows in raw.items():
        m = {c: np.array([num(r[cols[c]]) for r in rows]) for c in want}
        m["CONVECTIVE"] = np.array([r[cols["CONVECTIVE"]] == "T" for r in rows])
        atm = np.array([r[cols["ATM_PT"]] == "T" for r in rows])
        cut = int(np.argmax(atm)) if atm.any() else len(rows)
        out[step] = {k: v[:cut] for k, v in m.items()}
    return out


def layers_of(r, m, conv):
    """Transport layers [(r_outer, m_outer, convective)], thin ones merged."""
    runs = []  # [start, end] shell indices
    i, n = 0, len(r)
    while i < n:
        j = i
        while j + 1 < n and conv[j + 1] == conv[i]:
            j += 1
        runs.append([i, j, bool(conv[i])])
        i = j + 1

    def bounds(k):
        lo = 0.0 if k == 0 else 0.5 * (r[runs[k - 1][1]] + r[runs[k][0]])
        hi = 1.0 if k == len(runs) - 1 else 0.5 * (r[runs[k][1]] + r[runs[k + 1][0]])
        return lo, hi

    while len(runs) > 1:
        width = [bounds(k)[1] - bounds(k)[0] for k in range(len(runs))]
        k = int(np.argmin(width))
        if width[k] >= MIN_LAYER:
            break
        into = k - 1 if k > 0 else 1
        lo, hi = min(k, into), max(k, into)
        runs[lo] = [runs[lo][0], runs[hi][1], runs[into][2]]
        del runs[hi]
        # coalesce neighbours that now share a transport
        merged = [runs[0]]
        for run in runs[1:]:
            if run[2] == merged[-1][2]:
                merged[-1][1] = run[1]
            else:
                merged.append(run)
        runs = merged
    out = []
    for k, run in enumerate(runs):
        if k == len(runs) - 1:
            out.append((1.0, 1.0, run[2]))
        else:
            a, b = run[1], runs[k + 1][0]
            out.append((0.5 * (r[a] + r[b]), 0.5 * (m[a] + m[b]), run[2]))
    return out


def reduce_model(shells, centre):
    """A stored model on R_GRID: samples, layers, and a few raw values."""
    r = 10 ** shells["RADIUS"]
    m = 10 ** shells["MASS"]
    R, M = r[-1], m[-1]
    x, q = r / R, m / M
    # fusion share: running integral of E_NUC dm from the centre
    e = shells["E_NUC"]
    dm = np.diff(np.concatenate([[0.0], m]))
    e_mid = np.concatenate([[e[0]], 0.5 * (e[1:] + e[:-1])])
    cum = np.cumsum(e_mid * dm)
    share = np.maximum.accumulate(np.clip(cum / cum[-1], 0.0, 1.0))
    share[-1] = 1.0
    xs = np.concatenate([[0.0], x])
    lrho = np.concatenate([[centre["rho"]], shells["DENSITY"]])
    lp = np.concatenate([[centre["p"]], shells["PRESSURE"]])
    lt = np.concatenate([[centre["t"]], shells["TEMPERATURE"]])
    qs = np.concatenate([[0.0], q])
    sh = np.concatenate([[0.0], share])
    layers = layers_of(x, q, shells["CONVECTIVE"])
    bounds = np.array([b for b, _, _ in layers])
    conv = [layers[min(int(np.searchsorted(bounds, v, side="left")), len(layers) - 1)][2]
            for v in R_GRID]
    samples = np.column_stack(
        [
            R_GRID,
            np.interp(R_GRID, xs, qs),
            np.interp(R_GRID, xs, lrho),
            np.interp(R_GRID, xs, lp),
            np.interp(R_GRID, xs, lt),
            np.interp(R_GRID, xs, sh),
        ]
    )
    return {
        "samples": samples,
        "conv": conv,
        "layers": layers,
        "R_cm": R,
        "M_g": M,
        "L_nuc_erg_s": cum[-1],
        "raw": (xs, qs, lrho, lp, lt, shells["CONVECTIVE"]),
    }


def process_run(args):
    grid, mass, feh, tag = args
    base = os.path.join(grid, name(mass, tag))
    t = read_track(base + ".track")
    iz = zams_index(t)
    xz = t["X_cen"][iz]
    xc = t["X_cen"] / xz
    steps = t["Step"]
    row = {s: i for i, s in enumerate(steps)}
    store = read_store(base + ".store", steps[iz])
    models = {}
    for s, shells in sorted(store.items()):
        i = row[s]
        centre = {"rho": t["LogD_cen"][i], "p": t["logP_cen"][i], "t": t["LogT_cen"][i]}
        red = reduce_model(shells, centre)
        red.update(
            step=s,
            age=t["Age_gyr"][i],
            xc=xc[i],
            log_l=t["LogL_lsun"][i],
            log_r=t["LogR_rsun"][i],
            log_teff=t["log_Teff"][i],
        )
        if not (abs(mass - 1.0) < 1e-9 and feh == 0.0):
            red.pop("raw")
        models[s] = red
    # A run ends at X_c = 1e-4 (TAMS) or 20 Gyr. One that hit the model limit
    # instead stalled near TAMS: it is cut where X_c first reached its final
    # level, flagged, and kept out of the tables.
    tams = t["X_cen"][-1] <= 1.0001e-4
    failed = not tams and t["Age_gyr"][-1] < 19.99
    last = len(steps) - 1
    if failed:
        last = int(np.argmax(t["X_cen"] <= 1.05 * t["X_cen"][-1]))
        models = {s: m for s, m in models.items() if s <= steps[last]}
    # track rows: ZAMS, then TRACK_ROWS evenly spaced in xc_frac to the end,
    # plus TAIL_XC where the hook at the end of the main sequence is fast
    post = np.arange(iz, last + 1)
    targets = list(np.linspace(1.0, xc[last], TRACK_ROWS + 1)[1:])
    targets = sorted(targets + [x for x in TAIL_XC if x > xc[last]], reverse=True)
    picks = [iz]
    for target in targets:
        k = post[int(np.argmin(np.abs(xc[post] - target)))]
        if k > picks[-1]:
            picks.append(int(k))
    if picks[-1] != last:
        picks.append(int(last))
    track = [
        (t["Age_gyr"][k], t["LogL_lsun"][k], t["LogR_rsun"][k], t["log_Teff"][k], xc[k])
        for k in picks
    ]
    return {
        "mass": mass,
        "feh": feh,
        "failed": failed,
        "zams_age": t["Age_gyr"][iz],
        "xc_zams": xz,
        "end_xc": xc[last],
        "end_age": t["Age_gyr"][last],
        "track": track,
        "models": models,
    }


def snapshots(run):
    """Stored models chosen as profile snapshots."""
    ms = sorted(run["models"].values(), key=lambda m: m["step"])
    end_xc = ms[-1]["xc"]
    chosen = []
    for target in SNAPSHOT_XC:
        if target < 1.0 and end_xc > target:
            break
        m = ms[0] if target == 1.0 else min(ms, key=lambda m: abs(m["xc"] - target))
        if not chosen or m["step"] > chosen[-1]["step"]:
            chosen.append(m)
    if end_xc > SNAPSHOT_XC[-1] and ms[-1]["step"] > chosen[-1]["step"]:
        chosen.append(ms[-1])  # run stopped at 20 Gyr: keep its last state
    return chosen


def at_xc(run, x):
    """Samples (log rho, log P, log T) at central hydrogen x, linear between stored models."""
    ms = sorted(run["models"].values(), key=lambda m: -m["xc"])
    xs = np.array([m["xc"] for m in ms])
    if x > xs[0] or x < xs[-1]:
        return None
    k = int(np.searchsorted(-xs, -x))
    k = min(max(k, 1), len(ms) - 1)
    a, b = ms[k - 1], ms[k]
    w = 0.0 if xs[k - 1] == xs[k] else (xs[k - 1] - x) / (xs[k - 1] - xs[k])
    return (1 - w) * a["samples"][:, 2:5] + w * b["samples"][:, 2:5]


def mid_points(masses):
    """(lo, mid, hi, weight) for each interval of a mass list whose
    mid-point is a grid mass: weight linear in log mass."""
    out = []
    for lo, hi in zip(masses, masses[1:]):
        mid = round(0.5 * (lo + hi), 1)
        if abs(mid - 0.5 * (lo + hi)) < 1e-9 and lo < mid < hi:
            out.append((lo, mid, hi, math.log(mid / lo) / math.log(hi / lo)))
    return out


def stepped(step, lo_mass=2.0, hi_mass=8.0):
    """Masses from lo_mass to hi_mass by step."""
    return [round(lo_mass + k * step, 1) for k in range(int(round((hi_mass - lo_mass) / step)) + 1)]


def profile_errors(runs, masses):
    """Max |interpolated - computed| (dex) of log rho, log P, log T at the
    grid mid-points of the intervals of `masses`, at equal xc_frac and
    r_frac, over every metallicity and snapshot: for all radii and for
    r <= 0.9 R, with where the worst is.
    """
    worst = {"all": 0.0, "r09": 0.0, "where": None}
    for lo, mid, hi, w in mid_points(masses):
        for feh, _ in FEHS:
            if any(runs[(m, feh)]["failed"] for m in (lo, mid, hi)):
                continue
            for x in SNAPSHOT_XC:
                pl, ph, pm = (at_xc(runs[(m, feh)], x) for m in (lo, hi, mid))
                if pl is None or ph is None or pm is None:
                    continue
                err = np.abs((1 - w) * pl + w * ph - pm)
                e_all, e_in = float(err.max()), float(err[R_GRID <= 0.9].max())
                if e_all > worst["all"]:
                    j = np.unravel_index(int(np.argmax(err)), err.shape)
                    worst["where"] = (mid, feh, x, float(R_GRID[j[0]]), ["rho", "P", "T"][j[1]])
                worst["all"] = max(worst["all"], e_all)
                worst["r09"] = max(worst["r09"], e_in)
    return worst


def track_errors(runs, masses):
    """Max |interpolated - computed| (dex) of log L, log R, log T_eff at the
    grid mid-points of the intervals of `masses`, at equal fractional
    main-sequence age tau (as Task 2 interpolates), tau in 0..1 by 0.01,
    over every metallicity; then the same without the ZAMS row (tau > 0),
    and the (mid, feh, tau) of the worst log L."""
    worst, later, where = np.zeros(3), np.zeros(3), None
    for lo, mid, hi, w in mid_points(masses):
        for feh, _ in FEHS:
            if any(runs[(m, feh)]["failed"] for m in (lo, mid, hi)):
                continue
            ts = [np.array(runs[(m, feh)]["track"]) for m in (lo, mid, hi)]
            for tau in np.linspace(0.0, 1.0, 101):
                v = []
                for tr in ts:
                    age = tr[0, 0] + tau * (tr[-1, 0] - tr[0, 0])
                    v.append([np.interp(age, tr[:, 0], tr[:, c]) for c in (1, 2, 3)])
                v = np.array(v)
                err = np.abs((1 - w) * v[0] + w * v[2] - v[1])
                if err[0] > worst[0]:
                    where = (mid, feh, round(float(tau), 2))
                worst = np.maximum(worst, err)
                if tau > 0:
                    later = np.maximum(later, err)
    return worst, later, where


# ---------------------------------------------------------------- Baraffe, Dartmouth


def read_bhac15(path):
    rows = {}
    for line in open(path):
        s = line.split()
        if len(s) != 13 or line.lstrip().startswith("!"):
            continue
        try:
            v = [float(x) for x in s]
        except ValueError:
            continue
        rows.setdefault(round(v[0], 3), []).append(v)
    return {m: np.array(r) for m, r in rows.items()}


def read_dartmouth(path):
    a = np.loadtxt(path, comments="#")
    return {"age": a[:, 0], "logt": a[:, 1], "logl": a[:, 3], "logr": a[:, 4]}


def dartmouth_at(d, age_yr):
    return {k: float(np.interp(age_yr, d["age"], d[k])) for k in ("logt", "logl", "logr")}


def dartmouth_rows(directory):
    """The D rows: offsets at 5 Gyr of each [Fe/H] from [Fe/H] 0."""
    dart = {}
    for mass in DARTMOUTH_MASSES:
        for tag in ("m10", "m05", "p00", "p05"):
            path = os.path.join(directory, f"m{int(round(mass * 100)):03d}feh{tag}afep0.trk")
            dart[(mass, tag)] = read_dartmouth(path)
    rows = []
    for mass in DARTMOUTH_MASSES:
        ref = dartmouth_at(dart[(mass, "p00")], 5e9)
        for feh, tag in DARTMOUTH_FEHS:
            v = dartmouth_at(dart[(mass, tag)], 5e9)
            rows.append((mass, feh, v["logt"] - ref["logt"], v["logl"] - ref["logl"],
                         v["logr"] - ref["logr"]))
    return rows


def bhac_at(rows, age_gyr):
    lt = math.log10(age_gyr * 1e9)
    return {
        "teff": float(np.interp(lt, rows[:, 1], rows[:, 2])),
        "logl": float(np.interp(lt, rows[:, 1], rows[:, 3])),
        "r": float(np.interp(lt, rows[:, 1], rows[:, 5])),
        "logrhoc": float(np.interp(lt, rows[:, 1], rows[:, 8])),
    }


def track_at(run, age):
    a = np.array([r[0] for r in run["track"]])
    return {k: float(np.interp(age, a, [r[i] for r in run["track"]]))
            for i, k in ((1, "logl"), (2, "logr"), (3, "logteff"))}


# ---------------------------------------------------------------- writers


def header(f, lines):
    for line in lines:
        f.write(f"# {line}\n" if line else "#\n")


def write_tracks(path, runs, masses, step):
    with open(path, "w") as f:
        header(
            f,
            [
                "Main-sequence tracks, YREC grid 0.3-8 Msun (tools/stellar-grid).",
                "Source: YREC (Pinsonneault et al. 2025, Zenodo 17981745), std25 grid",
                "physics, GS98 mixture, no diffusion, no rotation; computed for this",
                "repository, derived numbers only. See STELLAR.md.",
                f"Masses: 0.1 Msun steps to 2 Msun, then {g(step)} Msun steps.",
                "Rows per (mass, feh): the ZAMS row (first model with nuclear power >= 99 % of L",
                "whose radius has stopped shrinking; see STELLAR.md),",
                f"then about {TRACK_ROWS} rows evenly spaced in xc_frac to the end of the run",
                "and rows near xc_frac 0.01, 0.003, 0.001 for the hook at the end",
                "(TAMS, X_c = 1e-4: last xc_frac <= 1.6e-4; or 20 Gyr: last xc_frac >= 1.6e-3).",
                "Units: age Gyr; log_l log10(L/Lsun); log_r log10(R/Rsun); log_teff log10(K);",
                "xc_frac central hydrogen / its ZAMS value.",
                "columns: mass_msun feh age_gyr log_l log_r log_teff xc_frac",
            ],
        )
        for mass in masses:
            for feh, _ in FEHS:
                for row in runs[(mass, feh)]["track"]:
                    f.write("\t".join(g(v) for v in (mass, feh) + tuple(row)) + "\n")


def write_profiles(path, runs, masses):
    with open(path, "w") as f:
        header(
            f,
            [
                "Main-sequence interior profiles, YREC grid (tools/stellar-grid).",
                "Source: YREC (Pinsonneault et al. 2025, Zenodo 17981745), std25 grid",
                "physics, GS98 mixture; computed for this repository. See STELLAR.md.",
                "Snapshots at xc_frac 1, 0.9, 0.7, 0.5, 0.3, 0.1, 0.01 as reached (nearest",
                "stored model; a run stopped at 20 Gyr adds its last model). Masses: 0.1 Msun",
                "steps to 2.6 Msun, then 3, 4, 5, 6, 7, 8 Msun.",
                "P rows: mass_msun feh age_gyr xc_frac n_layers n_samples",
                "L rows (centre out): r_outer_frac m_outer_frac transport (R radiative, C convective)",
                "S rows (centre out): r_frac m_frac log_rho_cgs log_p_cgs log_t fusion_share transport",
                "Units: rho g/cm3, P dyn/cm2, T K, all log10; r and m fractions of the",
                "photospheric radius and the star's mass; fusion_share = nuclear power",
                "inside r / total.",
            ],
        )
        for mass in masses:
            for feh, _ in FEHS:
                for s in snapshots(runs[(mass, feh)]):
                    lay, smp = s["layers"], s["samples"]
                    f.write(
                        "\t".join(["P", g(mass), g(feh), g(s["age"]), g(s["xc"]),
                                   str(len(lay)), str(len(smp))]) + "\n"
                    )
                    for ro, mo, c in lay:
                        f.write(f"L\t{g(ro)}\t{g(mo)}\t{'C' if c else 'R'}\n")
                    for v, c in zip(smp, s["conv"]):
                        f.write("S\t" + "\t".join(g(x) for x in v) + f"\t{'C' if c else 'R'}\n")


def write_low_mass(path, bh, dart_rows):
    with open(path, "w") as f:
        header(
            f,
            [
                "Low-mass stars 0.08-0.4 Msun at solar metallicity, and metallicity offsets.",
                "Source (rows without prefix): an extract of the published tables of",
                "Baraffe, Homeier, Allard & Chabrier 2015, A&A 577, A42 (BHAC15 tracks),",
                "cited; no licence is stated on the authors' site; the file itself is not",
                "redistributed. Rows as tabulated at the nearest tabulated ages, every",
                "track to its end (10 Gyr). Lsun = 3.839e33 erg/s, Rsun = 6.96e10 cm.",
                "columns: mass_msun age_gyr teff log_l r_rsun log_tc log_rhoc",
                "(log_tc: central T in K, log_rhoc: central density in g/cm3, log10)",
                "Source (D rows): Dartmouth (Dotter et al. 2008, ApJS 178, 89) tracks via",
                "Kiauhoku (Claytor et al. 2020; Zenodo 14908017, CC-BY 4.0), [a/Fe] = 0:",
                "value at feh minus value at feh 0, at 5 Gyr, linear in age.",
                "columns: D mass_msun feh dlog_teff dlog_l dlog_r",
            ],
        )
        for mass in LOW_MASSES:
            rows = bh[round(mass, 3)]
            idx = []
            for age in LOW_MASS_AGES:
                k = int(np.argmin(np.abs(rows[:, 1] - math.log10(age * 1e9))))
                if not idx or k > idx[-1]:
                    idx.append(k)
            if idx[-1] != len(rows) - 1:
                idx.append(len(rows) - 1)
            for k in idx:
                v = rows[k]
                f.write("\t".join(g(x) for x in (mass, 10 ** v[1] / 1e9, v[2], v[3], v[5], v[7], v[8])) + "\n")
        for row in dart_rows:
            f.write("D\t" + "\t".join(g(x) for x in row) + "\n")


# ---------------------------------------------------------------- main


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--grid", required=True)
    p.add_argument("--bhac15", required=True)
    p.add_argument("--dartmouth", required=True)
    p.add_argument("--b23", required=True)
    p.add_argument("--data", default=os.path.join(REPO, "crates", "science", "data"))
    p.add_argument("--workers", type=int, default=8)
    p.add_argument("--track-step", type=float, default=TRACK_STEP,
                   help="mass step of the tracks above 2 Msun (default %(default)s)")
    a = p.parse_args()

    jobs = [(a.grid, m, feh, tag) for m in GRID_MASSES for feh, tag in FEHS]
    with Pool(a.workers) as pool:
        runs = {(r["mass"], r["feh"]): r for r in pool.map(process_run, jobs)}

    def prof_line(label, masses):
        e = profile_errors(runs, masses)
        print(f"  {label}: {e['all']:.3f} | {e['r09']:.3f}; worst at (mid, feh, xc, r, var) {e['where']}")

    def track_line(label, masses):
        e, later, where = track_errors(runs, masses)
        print(f"  {label}: " + " ".join(f"{v:.4f}" for v in e) + "; tau > 0: "
              + " ".join(f"{v:.4f}" for v in later) + f"; worst log L at {where}")

    print("== mass steps above 2 Msun: max |interpolated - computed| at mid-points (dex)")
    print("profiles (log rho, P, T): all radii | r <= 0.9 R")
    for step in STEPS:
        prof_line(f"uniform step {step}", stepped(step))
    print("tracks (log L, log R, log Teff) at equal tau")
    for step in STEPS:
        track_line(f"uniform step {step}", stepped(step))
    print("== the tables' own grids")
    print("profiles, 2.0-2.6 Msun: the 0.1 step has no grid mid-point; 0.2 step bounds it")
    prof_line("2.0-2.6 by 0.2", stepped(0.2, 2.0, 2.6))
    for lo, hi in zip(PROFILE_HIGH, PROFILE_HIGH[1:]):
        prof_line(f"{lo}-{hi}", [lo, hi])
    print(f"tracks, step {a.track_step} above 2 Msun, per interval")
    hi_tracks = stepped(a.track_step)
    for lo, hi in zip(hi_tracks, hi_tracks[1:]):
        track_line(f"{lo}-{hi}", [lo, hi])
    track_line("all", hi_tracks)
    failed = [k for k, r in runs.items() if r["failed"]]
    print("failed runs (stalled, cut and left out):", failed)

    def checked(out):
        for m in out:
            for feh, _ in FEHS:
                assert not runs[(m, feh)]["failed"], (m, feh)
        return out

    track_masses = checked([m / 10 for m in range(3, 20)] + stepped(a.track_step))
    prof_masses = checked([m / 10 for m in range(3, 26)] + PROFILE_HIGH)

    os.makedirs(a.data, exist_ok=True)
    write_tracks(os.path.join(a.data, "stellar-tracks.tsv"), runs, track_masses, a.track_step)
    write_profiles(os.path.join(a.data, "stellar-profiles.tsv"), runs, prof_masses)

    bh = read_bhac15(a.bhac15)
    dart_rows = dartmouth_rows(a.dartmouth)
    write_low_mass(os.path.join(a.data, "stellar-low-mass.tsv"), bh, dart_rows)

    # ---- checks quoted in STELLAR.md
    sun = runs[(1.0, 0.0)]
    best = min(sun["models"].values(), key=lambda m: abs(m["age"] - SOLAR_AGE))
    env = [lay for lay in best["layers"]]
    k = len(env) - 1
    while k > 0 and not env[k][2]:
        k -= 1
    env_base = env[k - 1][0] if k > 0 else 0.0
    print("== Sun: 1 Msun [Fe/H] 0, stored model nearest 4.57 Gyr")
    print(f"step {best['step']} age {best['age']:.4f} Gyr xc_frac {best['xc']:.4f}")
    print(f"R {best['R_cm'] / RSUN_CM:.4f} Rsun (track {10 ** best['log_r']:.4f}), "
          f"L {10 ** best['log_l']:.4f} Lsun, Teff {10 ** best['log_teff']:.1f} K")
    print(f"rho_c {10 ** best['samples'][0, 2]:.1f} g/cm3, T_c {10 ** best['samples'][0, 4]:.4g} K, "
          f"envelope base {env_base:.4f} R, layers {[(round(r, 4), c) for r, _, c in best['layers']]}")
    cv = best["raw"][5]
    k = len(cv) - 1
    while k > 0 and not cv[k]:
        k -= 1
    while k > 0 and cv[k - 1]:
        k -= 1
    print(f"envelope base as first shell of the outer convective run: {best['raw'][0][k + 1]:.4f} R")
    print(f"fusion share at r 0.25: {np.interp(0.25, R_GRID, best['samples'][:, 5]):.5f}")

    b = np.loadtxt(a.b23, comments="#")
    print(f"B23 {os.path.basename(a.b23)}: rho_c {10 ** b[0, 6]:.1f} T_c {10 ** b[0, 4]:.4g}")
    xs, qs, lrho, lp, lt, _ = best["raw"]
    # the table's view of the Sun: snapshots interpolated in xc_frac
    tab = at_xc_snap(sun, best["xc"])
    print("== B23 comparison: the table at the Sun's xc_frac (interpolated between")
    print("snapshots) / B23; last column: largest |raw shells / table - 1| of rho, P, T")
    print("| r/R | rho (g/cm3) | B23 | P (dyn/cm2) | B23 | T (K) | B23 | m/M | B23 "
          "| fusion share | B23 L(r)/L | raw vs table |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    lb = b[:, 2] / b[-1, 2]
    for x in [0.0] + B23_RADII:
        y = [10 ** np.interp(x, xs, v) for v in (lrho, lp, lt)]
        t = [10 ** np.interp(x, R_GRID, tab[:, c]) for c in (2, 3, 4)]
        t += [np.interp(x, R_GRID, tab[:, c]) for c in (1, 5)]
        z = [10 ** np.interp(x, b[:, 0], b[:, c]) for c in (6, 5, 4)]
        z += [np.interp(x, b[:, 0], b[:, 1]), np.interp(x, b[:, 0], lb)]
        cells = [f"{t[i]:.4g} | {z[i]:.4g} ({100 * (t[i] / z[i] - 1):+.1f} %)" for i in range(3)]
        cells += [f"{t[i]:.4f} | {z[i]:.4f}" for i in (3, 4)]
        raw = max(abs(y[i] / t[i] - 1) for i in range(3))
        print(f"| {x:.2f} | " + " | ".join(cells) + f" | {100 * raw:.1f} % |")

    print("== seam at 0.3 Msun, [Fe/H] 0: Teff K, log L, R Rsun")
    y = runs[(0.3, 0.0)]
    for age in (1.0, 5.0):
        v = track_at(y, age)
        bv = bhac_at(bh[0.3], age)
        dv = dartmouth_at(dart[(0.3, "p00")], age * 1e9)
        print(f"{age} Gyr: YREC {10 ** v['logteff']:.0f} {v['logl']:.4f} {10 ** v['logr']:.4f} | "
              f"Baraffe {bv['teff']:.0f} {bv['logl']:.4f} {bv['r']:.4f} | "
              f"Dartmouth {10 ** dv['logt']:.0f} {dv['logl']:.4f} {10 ** dv['logr']:.4f}")
    print("== Baraffe central density at 5 Gyr (g/cm3)")
    for mass in LOW_MASSES:
        print(f"{mass}: {10 ** bhac_at(bh[round(mass, 3)], 5.0)['logrhoc']:.1f}")
    print("== YREC ZAMS ages and ends (Gyr)")
    for mass in (0.3, 0.5, 1.0, 2.0, 3.0, 5.0, 8.0):
        r = runs[(mass, 0.0)]
        print(f"{mass}: zams {r['zams_age']:.4g}, end {r['end_age']:.4g} at xc_frac {r['end_xc']:.3g}")
    print("== sizes")
    for f in ("stellar-tracks.tsv", "stellar-profiles.tsv", "stellar-low-mass.tsv"):
        print(f, os.path.getsize(os.path.join(a.data, f)))


def at_xc_snap(run, x):
    """Full samples at xc_frac x, linear between the two bracketing snapshots."""
    snaps = snapshots(run)
    xs = [s["xc"] for s in snaps]
    for k in range(1, len(snaps)):
        if xs[k] <= x <= xs[k - 1]:
            w = (xs[k - 1] - x) / (xs[k - 1] - xs[k])
            return (1 - w) * snaps[k - 1]["samples"] + w * snaps[k]["samples"]
    return snaps[0]["samples"]


if __name__ == "__main__":
    sys.exit(main())
