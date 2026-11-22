"""MOLA MEGDR (4 pixels per degree) area mean and Haar fluctuations on north-south profiles.

Data (not in the repository), fetched with curl -g:
  https://pds-geosciences.wustl.edu/mgs/mgs-m-mola-5-megdr-l3-v1/mgsl_300x/meg004/megt90n000cb.img

Usage: python3 -I mola_stats.py <data file>
Standard library only. Study: ../README.md (relief study, 2026-11-22).
"""
import sys, array, math
z = array.array('h'); z.frombytes(open(sys.argv[1],'rb').read()); z.byteswap()
NLAT, NLON = 720, 1440
lat = [90 - 0.25*(r+0.5) for r in range(NLAT)]
w = [math.cos(math.radians(l)) for l in lat]
tot = sum(w)*NLON
mean = sum(z[r*NLON+c]*w[r] for r in range(NLAT) for c in range(NLON))/tot
print('area mean m %.0f' % mean)
step_km = 3389.5*math.pi/NLAT
for lag in (4, 6, 8, 14, 28, 68, 136):
    h = lag//2; s = 0.0; c = 0
    for col in range(NLON):
        cs = [0.0]
        for r in range(NLAT): cs.append(cs[-1]+z[r*NLON+col])
        for a in range(0, NLAT-lag+1):
            s += abs((cs[a+lag]-cs[a+h]) - (cs[a+h]-cs[a]))/h; c += 1
    print('Haar lag %.0f km: %.0f m' % (lag*step_km, s/c))
