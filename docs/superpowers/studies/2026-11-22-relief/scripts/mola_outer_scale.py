"""MOLA MEGDR mean Haar fluctuation from 100 km to half the circumference, whole surface and per province.

The outer scale of the topography is the lag beyond which the mean Haar
fluctuation stops growing. Profiles are whole meridian great circles (a
meridian and its antimeridian, cyclic), so lags reach half the
circumference. A window counts for a province when every sample in it is
in the highlands (above -1000 m) or the lowlands (below -3000 m), as in
mola_haar_split.py.

Data (not in the repository), fetched with curl -g:
  https://pds-geosciences.wustl.edu/mgs/mgs-m-mola-5-megdr-l3-v1/mgsl_300x/meg004/megt90n000cb.img

Usage: python3 -I mola_outer_scale.py <data file>
Standard library only. Study: ../README.md (relief study, 2026-11-22).
"""
import sys, array, math
z = array.array('h'); z.frombytes(open(sys.argv[1],'rb').read()); z.byteswap()
NLAT, NLON = 720, 1440
step_km = 3389.5*math.pi/NLAT
LAGS = (8, 14, 28, 40, 54, 68, 80, 96, 108, 122, 136, 162, 190, 216, 244, 270, 324, 378, 432, 540, 720)
L = 2*NLAT
acc = {lag: {'all':[0.0,0], 'high':[0.0,0], 'low':[0.0,0]} for lag in LAGS}
for col in range(NLON//2):
    # North along `col`, south along the antimeridian: one great circle.
    loop = [z[r*NLON+col] for r in range(NLAT)] + [z[r*NLON+col+NLON//2] for r in reversed(range(NLAT))]
    prof = loop + loop
    cs=[0.0]; ch=[0]; cl=[0]
    for v in prof:
        cs.append(cs[-1]+v); ch.append(ch[-1]+(v>-1000)); cl.append(cl[-1]+(v<-3000))
    for lag in LAGS:
        h = lag//2; a_ = acc[lag]
        for a in range(L):
            f = abs((cs[a+lag]-cs[a+h])-(cs[a+h]-cs[a]))/h
            a_['all'][0]+=f; a_['all'][1]+=1
            if ch[a+lag]-ch[a]==lag: a_['high'][0]+=f; a_['high'][1]+=1
            if cl[a+lag]-cl[a]==lag: a_['low'][0]+=f; a_['low'][1]+=1
print('lag_km all_m high_m high_n low_m low_n')
for lag in LAGS:
    a_ = acc[lag]
    fmt = lambda k: '%.0f %d' % (a_[k][0]/a_[k][1], a_[k][1]) if a_[k][1] else '- 0'
    print('%.0f %.0f %s %s' % (lag*step_km, a_['all'][0]/a_['all'][1], fmt('high'), fmt('low')))
