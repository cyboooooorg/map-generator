"""ETOPO 2022 hypsometry and Haar fluctuations on north-south profiles (every 6th cell, 0.1 degree).

Data (not in the repository), fetched with curl -g:
  https://www.ngdc.noaa.gov/thredds/dodsC/global/ETOPO2022/60s/60s_surface_elev_netcdf/ETOPO_2022_v1_60s_N90W180_surface.nc.dods?z[0:6:10799][0:6:21599]

Usage: python3 -I etopo_stats.py <data file>
Standard library only. Study: ../README.md (relief study, 2026-11-22).
"""
import sys, array, math, struct
raw = open(sys.argv[1], 'rb').read()
i = raw.index(b'Data:\n') + 6
NLAT, NLON = 1800, 3600
n = struct.unpack('>I', raw[i:i+4])[0]; i += 8
z = array.array('f'); z.frombytes(raw[i:i+4*n]); z.byteswap(); i += 4*n
i += 8
lat = array.array('d'); lat.frombytes(raw[i:i+8*NLAT]); lat.byteswap()
w = [math.cos(math.radians(l)) for l in lat]
land, sea = [], []
for r in range(NLAT):
    wr = w[r]; row = z[r*NLON:(r+1)*NLON]
    for v in row:
        (land if v > 0 else sea).append((v, wr))
A = lambda xs: sum(x[1] for x in xs)
tot = A(land) + A(sea)
print('lat range', lat[0], lat[-1], 'n', n)
print('land share %.4f' % (A(land)/tot))
def above(xs, h): return 100*sum(x[1] for x in xs if x[0] > h)/A(xs)
def below(xs, h): return 100*sum(x[1] for x in xs if x[0] < h)/A(xs)
land.sort()
acc, half = 0, A(land)/2
for v, wt in land:
    acc += wt
    if acc >= half: print('land median m %.0f' % v); break
print('land under 500 m %.1f%%' % (100-above(land, 500)))
for h in (1000, 2000, 3000, 4000): print('land above %d m %.2f%%' % (h, above(land, h)))
for h in (-200, -1000, -3000, -4000, -6000): print('sea floor below %d m %.2f%%' % (h, below(sea, h)))
mean = sum(v*w[r] for r in range(NLAT) for v in z[r*NLON:(r+1)*NLON]) / tot
print('area mean m %.0f' % mean)
# Haar fluctuations on N-S profiles (one sample = 0.1 deg = 11.12 km)
step_km = 6371.0*math.pi/NLAT
for lag in (4, 8, 10, 18, 36, 90, 180):
    h = lag//2; s = 0.0; c = 0
    for col in range(0, NLON, 3):
        prof = [z[r*NLON+col] for r in range(NLAT)]
        cs = [0.0]
        for v in prof: cs.append(cs[-1]+v)
        for a in range(0, NLAT-lag+1, 2):
            m1 = (cs[a+h]-cs[a])/h; m2 = (cs[a+lag]-cs[a+h])/h
            s += abs(m2-m1); c += 1
    print('Haar lag %.0f km: %.0f m' % (lag*step_km, s/c))
