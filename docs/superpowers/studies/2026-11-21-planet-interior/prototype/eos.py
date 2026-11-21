"""EOS library. All numbers are from study-eos.md (source in comments). SI units.
Every EOS is turned into rho(P) tabulated on a common log-P grid (linear interp in log-log)."""
import numpy as np

LOGP = np.linspace(np.log(1e3), np.log(3e13), 40001)   # Pa
PG = np.exp(LOGP)
DLOG = LOGP[1] - LOGP[0]
GPa = 1e9


def _invert(xgrid, Pgrid, rho0):
    """rho(P) on PG from monotone P(x), rho = rho0*x. P below P(x0) -> rho0 (clamped)."""
    assert np.all(np.diff(Pgrid) > 0), "P(x) not monotone"
    return np.interp(PG, Pgrid, rho0 * xgrid)


def bm3(rho0, K0, K1):                 # 3rd-order Birch-Murnaghan, rho0 [kg/m3], K0 [GPa]
    x = np.exp(np.linspace(0, np.log(80), 200001))
    P = 1.5 * K0 * GPa * (x**(7/3) - x**(5/3)) * (1 + 0.75 * (K1 - 4) * (x**(2/3) - 1))
    ok = np.maximum.accumulate(P) == P
    ok &= np.arange(len(P)) < (np.argmax(np.diff(P) <= 0) if np.any(np.diff(P) <= 0) else len(P))
    return _invert(x[ok], P[ok], rho0)


def bm2(rho0, K0):
    return bm3(rho0, K0, 4.0)


def vinet(rho0, K0, K1):               # x = V/V0 = rho0/rho
    x = np.exp(-np.linspace(0, np.log(80), 200001))
    y = x**(1/3)
    P = 3 * K0 * GPa * x**(-2/3) * (1 - y) * np.exp(1.5 * (K1 - 1) * (1 - y))
    return _invert(1 / x, P, rho0)


def holzapfel_fe_hakim():
    """Hakim 2018 eq (5): valid above P0=234.4 GPa. Returns rho(P) (NaN below P0)."""
    V0, P0, K, c0, c2 = 4.28575e-6, 234.4 * GPa, 1145.7 * GPa, 3.19, -2.40
    M = 55.845e-3   # Fe molar mass kg/mol (standard atomic weight, NOT in the study; cross-check:
    #                 Dewaele rho0 8270 * V0 6.75 cm3/mol = 55.8 g/mol, Unterborn Table 1)
    x = np.exp(-np.linspace(0, np.log(60), 200001))
    y = x**(1/3)
    P = P0 + 3 * K * x**(-5/3) * (1 - y) * (1 + c2 * y * (1 - y)) * np.exp(c0 * (1 - y))
    rhoref = M / V0
    r = _invert(1 / x, P, rhoref)
    r[PG < P0] = np.nan
    return r, rhoref


def switch(P_tr, low, high):
    return np.where(PG < P_tr, low, high)


def mix(wr_pairs):
    """Amagat (equal-pressure, additive volume) mix; list of (mass_fraction, rho_array)."""
    s = sum(w for w, _ in wr_pairs)
    return s / sum(w / r for w, r in wr_pairs)


def poly(rho0, c, n):                  # Seager 2007 Table 3: rho = rho0 + c P^n
    return rho0 + c * PG**n


# ---------------- materials ----------------
def build_all():
    E = {}
    # Seager 2007 Table 3
    E['A_Fe'] = poly(8300.0, 0.00349, 0.528)
    E['A_Si'] = poly(4100.0, 0.00161, 0.541)
    E['A_H2O'] = poly(1460.0, 0.00311, 0.513)
    # Fe: Dewaele 2006 Vinet (Hakim Table F.7; Unterborn Table 1) below 234.4 GPa + Hakim 2018 above
    fe_dew = vinet(8270.0, 163.4, 5.38)
    fe_hak, rref = holzapfel_fe_hakim()
    E['Fe_B'] = np.where(PG < 234.4 * GPa, fe_dew, fe_hak)
    E['Fe_dewaele_only'] = fe_dew
    E['_fe_jump'] = (np.interp(234.4 * GPa, PG, fe_dew), rref)
    # silicates, Unterborn 2016 Table 1 (SLB values at 300 K, BM3)
    brg = bm3(4110.0, 251.0, 4.1)
    fesio3 = bm3(5180.0, 272.0, 4.1)
    mgo = bm3(3590.0, 161.0, 3.8)
    fo = bm3(3230.0, 128.0, 4.2)
    en = bm3(3200.0, 107.0, 7.0)
    # molar masses from rho0*V0 of the same table (g/mol): MgSiO3 4.11*24.45, FeSiO3 5.18*25.49
    mMg, mFe = 4.11 * 24.45, 5.18 * 25.49
    wFe = 0.12 * mFe / (0.12 * mFe + 0.88 * mMg)          # Mg0.88 Fe0.12 SiO3 (Seager Table 1 label)
    brg88 = mix([(1 - wFe, brg), (wFe, fesio3)])
    lower = mix([(0.9, brg88), (0.1, mgo)])               # Seager 2007 Sect 4.1 test mantle 90/10
    upper = mix([(0.5, fo), (0.5, en)])                   # Unterborn Sect 3.5: 50/50 olivine/pyroxene
    Ptr = 23.83 * GPa
    E['Mantle_B'] = switch(Ptr, upper, lower)
    E['_upper_rho0'] = float(np.interp(1e5, PG, upper))
    # water: ice VII Vinet fit to DFT (Seager 2007 Sect 3.3); no sourced low-P phase beyond this
    E['H2O_B'] = vinet(1460.0, 14.3771, 6.57972)
    # C: Zeng 2016 BM2 fits to PREM
    ic = bm2(7850.0, 255.0)
    oc = bm2(7050.0, 201.0)
    E['Fe_C'] = switch(328.851 * GPa, oc, ic)             # ICB pressure from PREM (study sect 4)
    lm = bm2(3980.0, 206.0)
    rho_um_top = 3992.1                                  # PREM above 670 km (study sect 4)
    um_lin = E['_upper_rho0'] + (rho_um_top - E['_upper_rho0']) * np.clip(PG / Ptr, 0, 1)
    E['Mantle_C'] = switch(Ptr, um_lin, lm)
    E['H2O_C'] = E['H2O_B']
    # D1: Valencia 2006 Table 1 Fe+alloy fitted to PREM (BM3)
    ocv = bm3(6800.0, 165.0, 4.2)
    icv = bm3(7990.0, 200.0, 5.2)
    E['Fe_D1'] = switch(328.851 * GPa, ocv, icv)
    # D2: B iron scaled by constant density deficit calibrated on PREM CMB core side
    rho_fe_cmb = float(np.interp(135.751 * GPa, PG, E['Fe_B']))
    delta = 1 - 9903.5 / rho_fe_cmb
    E['_delta'] = delta
    E['Fe_D2'] = E['Fe_B'] * (1 - delta)
    return E


SETS = {  # candidate -> (core, mantle, water)
    'A': ('A_Fe', 'A_Si', 'A_H2O'),
    'B': ('Fe_B', 'Mantle_B', 'H2O_B'),
    'C': ('Fe_C', 'Mantle_C', 'H2O_C'),
    'D1': ('Fe_D1', 'Mantle_B', 'H2O_B'),
    'D2': ('Fe_D2', 'Mantle_B', 'H2O_B'),
}
