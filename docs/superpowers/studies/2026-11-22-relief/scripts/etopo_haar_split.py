"""ETOPO 2022 Haar fluctuations split into windows wholly on land and wholly on the sea floor (deeper than 200 m).

Data (not in the repository), fetched with curl -g:
  https://www.ngdc.noaa.gov/thredds/dodsC/global/ETOPO2022/60s/60s_surface_elev_netcdf/ETOPO_2022_v1_60s_N90W180_surface.nc.dods?z[0:6:10799][0:6:21599]

Usage: python3 -I etopo_haar_split.py <data file>
Standard library only. Study: ../README.md (relief study, 2026-11-22).
"""
import sys, array, math, struct
raw = open(sys.argv[1], 'rb').read()
i = raw.index(b'Data:\n') + 6
NLAT, NLON = 1800, 3600
n = struct.unpack('>I', raw[i:i+4])[0]; i += 8
z = array.array('f'); z.frombytes(raw[i:i+4*n]); z.byteswap()
step_km = 6371.0*math.pi/NLAT
for lag in (8, 10, 18, 36, 90):
    h = lag//2; acc = {'all':[0,0], 'land':[0,0], 'floor':[0,0]}
    for col in range(0, NLON, 3):
        prof = [z[r*NLON+col] for r in range(NLAT)]
        cs=[0.0]; cl=[0]; cf=[0]
        for v in prof:
            cs.append(cs[-1]+v); cl.append(cl[-1]+(v>0)); cf.append(cf[-1]+(v<-200))
        for a in range(0, NLAT-lag+1, 2):
            f = abs((cs[a+lag]-cs[a+h])-(cs[a+h]-cs[a]))/h
            acc['all'][0]+=f; acc['all'][1]+=1
            if cl[a+lag]-cl[a]==lag: acc['land'][0]+=f; acc['land'][1]+=1
            if cf[a+lag]-cf[a]==lag: acc['floor'][0]+=f; acc['floor'][1]+=1
    print('lag %.0f km: ' % (lag*step_km) + ', '.join('%s %.0f m (n=%d)' % (k, s/max(c,1), c) for k,(s,c) in acc.items()))
