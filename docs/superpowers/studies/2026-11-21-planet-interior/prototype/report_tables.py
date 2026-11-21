import json, math, numpy as np
import run, eos, solve
from scipy.optimize import brentq
res = json.load(open('results.json'))
B = run.B
cands = list(eos.SETS)
RE = 6371.0
# reference radii for bodies without a measured R
def zeng(M, cmf=0.325): return (1.07-0.21*cmf)*M**(1/3.7)*RE
def seager23(M, m1=6.41, r1=3.19, k1=-0.20945, k2=0.0804, k3=0.394):
    Ms = M/m1; return 10**(k1+math.log10(Ms)/3-k2*Ms**k3)*r1*RE
refR = {'5ME': zeng(5), '10ME': zeng(10), '5ME_50%H2O': 1.911*RE}
print('Zeng2016 5ME', zeng(5), '10ME', zeng(10), 'Seager eq23 Earth-like (Fe0.3 row r1=3.19 as swapped)', seager23(1), seager23(5), seager23(10))
print('Zeng 2019 R 300K', 1.911*RE, '700K', 1.981*RE)
L = []
hdr = '| Body | ' + ' | '.join(cands) + ' |'
L.append(hdr); L.append('|' + '---|'*(len(cands)+1))
L2 = list(L)
for body, (M, R0, C0, fr) in B.items():
    R0 = R0 or refR.get(body)
    row = f'| {body} (ref R {R0:.0f} km) |'; row2 = f'| {body} (ref C {C0 if C0 else "-"}) |'
    for c in cands:
        r = res[f'{body}|{c}']
        row += f" {r['R']:.0f} ({(r['R']/R0-1)*100:+.2f}%) |"
        row2 += f" {r['C']:.4f}" + (f" ({(r['C']/C0-1)*100:+.2f}%)" if C0 else '') + ' |'
    L.append(row); L2.append(row2)
print('\n'.join(L)); print(); print('\n'.join(L2))
# timing
print('time per model (s): mean/max', np.mean([v['t'] for v in res.values()]), max(v['t'] for v in res.values()))
# Earth structure
print('\nEarth structure vs PREM (rc 3480, Pcmb 135.751, Pc 363.852):')
for c in cands:
    r = res[f'Earth|{c}']; print(c, f"rcmb={r['rcore']:.0f} Pcmb={r['Pcmb']:.1f} Pc={r['Pc']:.1f}")
print('Mercury C vs 0.333:', {c: round((res[f'Mercury|{c}']['C']/0.333-1)*100,2) for c in cands})
print('Venus C vs 0.337 (+-0.024):', {c: round(res[f'Venus|{c}']['C'],4) for c in cands})
# diagnostic: water density scale needed
E = eos.build_all()
for body in ['Ganymede','Titan']:
    M,R0,C0,fr = B[body]
    def f(s):
        lay=[]
        for key,w in zip(('Fe_B','Mantle_B','H2O_B'),fr):
            if w>0:
                tab = E[key]*(s if key=='H2O_B' else 1.0)
                lay.append((solve.make_rho(tab),w))
        # keep pressure behaviour: scale density only
        return solve.solve(M,lay,N=200)['R']/1e3-R0
    s = brentq(f,0.5,1.0)
    lay=[(solve.make_rho(E[k]*(s if k=='H2O_B' else 1)),w) for k,w in zip(('Fe_B','Mantle_B','H2O_B'),fr) if w>0]
    r=solve.solve(M,lay,N=200)
    print(body,'water rho scale to hit R:',round(s,3),'-> C',round(r['C'],4),'target',C0)
