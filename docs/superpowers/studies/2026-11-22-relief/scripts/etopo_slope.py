"""ETOPO 2022 continental slope: area-weighted gradient of the sea floor in depth bands near land, 60S-60N, every 3rd cell (3 arc minutes).

Data (not in the repository), fetched with curl -g:
  https://www.ngdc.noaa.gov/thredds/dodsC/global/ETOPO2022/60s/60s_surface_elev_netcdf/ETOPO_2022_v1_60s_N90W180_surface.nc.dods?z[1800:3:8999][0:3:21599]

Usage: python3 -I etopo_slope.py <data file> [land distance km, default 150]
Standard library only. Study: ../README.md (relief study, 2026-11-22).
"""
import sys, array, math, struct
raw = open(sys.argv[1], 'rb').read()
i = raw.index(b'Data:\n') + 6
NLAT, NLON = 2400, 7200
n = struct.unpack('>I', raw[i:i+4])[0]; assert n == NLAT*NLON; i += 8
z = array.array('f'); z.frombytes(raw[i:i+4*n]); z.byteswap()
lat = [-60 + 0.05*r for r in range(NLAT)]
dy = 6371000.0*math.radians(0.05)
R_KM = float(sys.argv[2]) if len(sys.argv) > 2 else 150.0
rr = int(round(R_KM*1000/dy))
# land mask dilated along rows (wrapping) then along columns
row_d = bytearray(n)
for r in range(NLAT):
    rc = min(NLON//2, int(math.ceil(rr/max(math.cos(math.radians(lat[r])),1e-3))))
    base = r*NLON
    land = [1 if z[base+c] > 0 else 0 for c in range(NLON)]
    cs = [0]
    for v in land + land + land: cs.append(cs[-1]+v)
    for c in range(NLON):
        a = c + NLON - rc; b = c + NLON + rc + 1
        if cs[b]-cs[a]: row_d[base+c] = 1
near = bytearray(n)
for c in range(NLON):
    cs = [0]
    for r in range(NLAT): cs.append(cs[-1]+row_d[r*NLON+c])
    for r in range(NLAT):
        a = max(0, r-rr); b = min(NLAT, r+rr+1)
        if cs[b]-cs[a]: near[r*NLON+c] = 1
for lo, hi in ((-2500, -200), (-3000, -200), (-2000, -500)):
    vals = []
    for r in range(1, NLAT-1):
        cl = math.cos(math.radians(lat[r])); dx = dy*cl; base = r*NLON
        for c in range(NLON):
            k = base+c; v = z[k]
            if not (lo < v < hi) or not near[k]: continue
            gx = (z[base+(c+1)%NLON] - z[base+(c-1)%NLON])/(2*dx)
            gy = (z[k+NLON] - z[k-NLON])/(2*dy)
            vals.append((math.degrees(math.atan(math.hypot(gx, gy))), cl))
    vals.sort(); tot = sum(w for _, w in vals); acc = 0; q = {}
    for s, w in vals:
        acc += w
        for p in (0.25, 0.5, 0.75):
            if p not in q and acc >= p*tot: q[p] = s
    mean = sum(s*w for s, w in vals)/tot
    print('band %d..%d m, land within %.0f km: cells %d, mean %.2f deg, quartiles %.2f / %.2f / %.2f deg' % (lo, hi, R_KM, len(vals), mean, q[0.25], q[0.5], q[0.75]))
