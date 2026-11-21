"""Ganymede and Titan with the water laws of study section 2.7 (water.py).
Candidate D1 core and B mantle; only the water layer changes. Run: python water_run.py"""
import numpy as np
import eos, solve, water as W

E = eos.build_all()
PG = eos.PG
VII = E['H2O_B']           # Seager 2007 ice VII Vinet above the VI/VII boundary
B = {'Ganymede': (1.4819e23, 2634.1, 0.3115, (0.085, 0.46, 0.455)),
     'Titan': (1.3455e23, 2574.7, 0.3414, (0.0, 0.545, 0.455))}
JOURNAUX = dict(W.ICE)
ACUNA = {'III': (1139.0, 8.5e9, 5.7), 'V': (1235.0, 13.3e9, 5.2), 'VI': (1270.0, 14.05e9, 4.0)}


def bisect(f, lo, hi, n=100):
    for _ in range(n):
        m = 0.5 * (lo + hi)
        if f(m) < 0: lo = m
        else: hi = m
    return 0.5 * (lo + hi)


def melting_path(p):
    if p < 208.566e6:
        t = 273.16 if p <= 611.657 else bisect(lambda t: p - W.pm_ih(t), 251.165, 273.16)
        return W.rho_ih(t, p)
    for name, top in (('III', 350.1e6), ('V', 632.4e6), ('VI', 2216e6)):
        if p < top: return W.bm3_rho(p, *W.ICE[name])
    return None


def isotherm(t):
    pm, p67 = W.pm_vi(t), bisect(lambda p: W.t67(p) - t, 0.5e9, 2.216e9)
    return lambda p: W.rho95(t, p) if p < pm else (W.bm3_rho(p, *W.ICE['VI']) if p < p67 else None)


def table(f):
    out = VII.copy()
    for i in np.nonzero(PG < 2.216e9)[0]:
        r = f(PG[i])
        if r is not None: out[i] = r
    return out


for sname, ice in (('Journaux', JOURNAUX), ('Acuna', ACUNA)):
    W.ICE.clear(); W.ICE.update(ice)
    paths = {'melting curve': melting_path, 'isotherm 273.31 K': isotherm(273.31)}
    if sname == 'Journaux': paths['isotherm 300 K'] = isotherm(300.0)
    for pname, f in paths.items():
        water = table(f)
        for body, (M, R0, C0, fr) in B.items():
            layers = [(solve.make_rho(E[k] if k else water), w)
                      for k, w in zip(('Fe_D1', 'Mantle_B', None), fr) if w > 0]
            r = solve.solve(M, layers, N=400)
            R = r['R'] / 1e3
            print(f"{sname:8s} {pname:18s} {body:9s} R={R:.1f} ({(R/R0-1)*100:+.2f} %) "
                  f"C={r['C']:.4f} ({(r['C']/C0-1)*100:+.2f} %)", flush=True)
