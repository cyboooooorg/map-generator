import time, json, sys, numpy as np
import eos, solve
E = eos.build_all()
ME, RE = 5.9722e24, 6371.0e3   # NASA Earth fact sheet
# name: (mass kg, target R km, target C, fractions (core, mantle, water), note)
B = {
 'Earth':      (5.9722e24, 6371.0, 0.3307, (0.325, 0.675, 0.0)),
 'Venus':      (4.8673e24, 6051.8, None,   (0.30, 0.70, 0.0)),
 'Mars(0.21)': (0.64169e24, 3389.5, 0.3638, (0.21, 0.79, 0.0)),
 'Mars(0.25)': (0.64169e24, 3389.5, 0.3638, (0.25, 0.75, 0.0)),
 'Mercury':    (0.33010e24, 2439.7, 0.346,  (0.7136, 0.2864, 0.0)),
 'Moon':       (0.07346e24, 1737.4, 0.3927, (0.0168, 0.9832, 0.0)),
 'Ganymede':   (1.4819e23, 2634.1, 0.3115, (0.085, 0.46, 0.455)),
 'Titan':      (1.3455e23, 2574.7, 0.3414, (0.0, 0.545, 0.455)),
 '5ME':        (5*ME, None, None, (0.325, 0.675, 0.0)),
 '5ME_50%H2O': (5*ME, None, None, (0.1625, 0.3375, 0.5)),
 '10ME':       (10*ME, None, None, (0.325, 0.675, 0.0)),
}
def layers_for(cand, fr):
    c, m, w = SETS[cand]
    out = []
    for key, f in zip((c, m, w), fr):
        if f > 0: out.append((solve.make_rho(E[key]), f))
    return out
SETS = eos.SETS
def run(cand, body, N=400, keep=False):
    M, R0, C0, fr = B[body]
    t = time.perf_counter()
    res = solve.solve(M, layers_for(cand, fr), N=N, keep=keep)
    res['time'] = time.perf_counter() - t
    return res
if __name__ == '__main__':
    out = {}
    for body in B:
        for cand in SETS:
            r = run(cand, body)
            rr = r['R'] / 1e3
            nl = len(r['info'])
            cmb = r['info'][0] if (nl > 1 and B[body][3][0] > 0) else (None, None)
            out[f'{body}|{cand}'] = dict(R=rr, C=r['C'], Pc=r['Pc']/1e9, rcore=(cmb[0]/1e3 if cmb[0] else None),
                                         Pcmb=(cmb[1]/1e9 if cmb[1] else None), t=r['time'])
            print(body, cand, f"R={rr:.1f} C={r['C']:.4f} Pc={r['Pc']/1e9:.1f} rc={out[f'{body}|{cand}']['rcore']} t={r['time']:.2f}", flush=True)
    json.dump(out, open('results.json', 'w'), indent=1)
