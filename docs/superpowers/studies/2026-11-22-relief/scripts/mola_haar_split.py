"""MOLA MEGDR Haar fluctuations split into highland (above -1000 m) and lowland (below -3000 m) windows.

Data (not in the repository), fetched with curl -g:
  https://pds-geosciences.wustl.edu/mgs/mgs-m-mola-5-megdr-l3-v1/mgsl_300x/meg004/megt90n000cb.img

Usage: python3 -I mola_haar_split.py <data file>
Standard library only. Study: ../README.md (relief study, 2026-11-22).
"""
import sys, array, math
z = array.array('h'); z.frombytes(open(sys.argv[1],'rb').read()); z.byteswap()
NLAT, NLON = 720, 1440
step_km = 3389.5*math.pi/NLAT
for lag in (6, 8, 14, 68):
    h = lag//2; acc = {'all':[0,0], 'high(>-1000)':[0,0], 'low(<-3000)':[0,0]}
    for col in range(NLON):
        cs=[0.0]; ch=[0]; cl=[0]
        for r in range(NLAT):
            v = z[r*NLON+col]; cs.append(cs[-1]+v); ch.append(ch[-1]+(v>-1000)); cl.append(cl[-1]+(v<-3000))
        for a in range(0, NLAT-lag+1):
            f = abs((cs[a+lag]-cs[a+h])-(cs[a+h]-cs[a]))/h
            acc['all'][0]+=f; acc['all'][1]+=1
            if ch[a+lag]-ch[a]==lag: acc['high(>-1000)'][0]+=f; acc['high(>-1000)'][1]+=1
            if cl[a+lag]-cl[a]==lag: acc['low(<-3000)'][0]+=f; acc['low(<-3000)'][1]+=1
    print('lag %.0f km: ' % (lag*step_km) + ', '.join('%s %.0f m (n=%d)' % (k, s/max(c,1), c) for k,(s,c) in acc.items()))
