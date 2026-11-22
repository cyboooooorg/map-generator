"""Measured, not used in W2 (see ../topography.md, "Outer scale"). ETOPO 2022 mean Haar fluctuation from 100 km to half the circumference, whole surface and per province.

The outer scale of the topography is the lag beyond which the mean Haar
fluctuation stops growing. Profiles are whole meridian great circles (a
meridian and its antimeridian, 40 000 km, cyclic), so lags reach half the
circumference. A window counts for a province when every sample in it is on
land (above 0 m) or on the sea floor (deeper than 200 m), as in
etopo_haar_split.py.

Data (not in the repository), fetched with curl -g:
  https://www.ngdc.noaa.gov/thredds/dodsC/global/ETOPO2022/60s/60s_surface_elev_netcdf/ETOPO_2022_v1_60s_N90W180_surface.nc.dods?z[0:6:10799][0:6:21599]

Usage: python3 -I etopo_outer_scale.py <data file>
Standard library only. Study: ../README.md (relief study, 2026-11-22).
"""
import sys, array, math, struct
raw = open(sys.argv[1], 'rb').read()
i = raw.index(b'Data:\n') + 6
NLAT, NLON = 1800, 3600
n = struct.unpack('>I', raw[i:i+4])[0]; i += 8
z = array.array('f'); z.frombytes(raw[i:i+4*n]); z.byteswap()
step_km = 6371.0*math.pi/NLAT
LAGS = (10, 18, 36, 54, 90, 126, 144, 162, 180, 216, 252, 288, 324, 360, 432, 504, 540, 576, 648, 720, 900, 1080, 1440, 1800)
L = 2*NLAT
acc = {lag: {'all':[0.0,0], 'land':[0.0,0], 'floor':[0.0,0]} for lag in LAGS}
for col in range(0, NLON//2, 3):
    # North along `col`, south along the antimeridian: one great circle.
    loop = [z[r*NLON+col] for r in range(NLAT)] + [z[r*NLON+col+NLON//2] for r in reversed(range(NLAT))]
    prof = loop + loop
    cs=[0.0]; cl=[0]; cf=[0]
    for v in prof:
        cs.append(cs[-1]+v); cl.append(cl[-1]+(v>0)); cf.append(cf[-1]+(v<-200))
    for lag in LAGS:
        h = lag//2; a_ = acc[lag]
        for a in range(0, L, 2):
            f = abs((cs[a+lag]-cs[a+h])-(cs[a+h]-cs[a]))/h
            a_['all'][0]+=f; a_['all'][1]+=1
            if cl[a+lag]-cl[a]==lag: a_['land'][0]+=f; a_['land'][1]+=1
            if cf[a+lag]-cf[a]==lag: a_['floor'][0]+=f; a_['floor'][1]+=1
print('lag_km all_m land_m land_n floor_m floor_n')
for lag in LAGS:
    a_ = acc[lag]
    print('%.0f %.0f %s %d %s %d' % (lag*step_km, a_['all'][0]/a_['all'][1],
          '%.0f' % (a_['land'][0]/a_['land'][1]) if a_['land'][1] else '-', a_['land'][1],
          '%.0f' % (a_['floor'][0]/a_['floor'][1]) if a_['floor'][1] else '-', a_['floor'][1]))
