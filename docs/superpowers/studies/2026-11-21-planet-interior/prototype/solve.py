"""Hydrostatic structure in mass coordinate t = (m/M)^(1/3) (smooth at the centre, layer
boundaries exact). State (r, P, I). dr/dt = 3 M t^2/(4 pi r^2 rho), dP/dt = -3 G M^2 t^5/(4 pi r^4),
dI/dt = 2 M t^2 r^2  (I = int 2/3 r^2 dm = (8 pi/3) int rho r^4 dr). RK4, N steps per layer."""
import math
import numpy as np
from scipy.optimize import brentq
from eos import LOGP, DLOG

G = 6.6743e-11
PS = 1e5
L0 = float(LOGP[0]); NT = len(LOGP) - 1


def make_rho(tab):
    lt = np.log(tab).tolist()
    def rho(P):
        u = (math.log(P) - L0) / DLOG if P > 1e3 else 0.0
        if u >= NT: u = NT - 1e-9
        i = int(u); f = u - i
        return math.exp(lt[i] + f * (lt[i + 1] - lt[i]))
    return rho


def shoot(Pc, M, layers, N, keep=False):
    """layers = [(rho_fn, mass_fraction)], centre first. Returns (P_end, R, I, info)."""
    t0 = 1e-3
    rho_c = layers[0][0](Pc)
    m0 = M * t0**3
    r = (3 * m0 / (4 * math.pi * rho_c))**(1 / 3)
    P = Pc; I = 0.4 * m0 * r * r
    cum = 0.0; t = t0; info = []; prof = [] if keep else None
    c3 = 3 / (4 * math.pi)
    for k, (rf, w) in enumerate(layers):
        cum += w
        tend = cum**(1 / 3)
        h = (tend - t) / N
        for _ in range(N):
            def f(tt, rr, PP):
                d = rf(PP)
                return (c3 * M * tt * tt / (rr * rr * d),
                        -c3 * G * M * M * tt**5 / rr**4,
                        2 * M * tt * tt * rr * rr, d)
            a = f(t, r, P)
            b = f(t + h / 2, r + h / 2 * a[0], P + h / 2 * a[1])
            c = f(t + h / 2, r + h / 2 * b[0], P + h / 2 * b[1])
            d_ = f(t + h, r + h * c[0], P + h * c[1])
            if keep: prof.append((r, a[3], P, M * t**3))
            r += h / 6 * (a[0] + 2 * b[0] + 2 * c[0] + d_[0])
            P += h / 6 * (a[1] + 2 * b[1] + 2 * c[1] + d_[1])
            I += h / 6 * (a[2] + 2 * b[2] + 2 * c[2] + d_[2])
            t += h
            if P < 1e3: P = 1e3
        info.append((r, P))
    return P, r, I, info, prof


def solve(M, layers, N=400, keep=False, tol=1e-12):
    f = lambda lp: shoot(math.exp(lp), M, layers, N)[0] - PS
    lo, hi = math.log(2e8), math.log(2.9e13)
    lp = brentq(f, lo, hi, xtol=tol, rtol=1e-13)
    Pc = math.exp(lp)
    P, R, I, info, prof = shoot(Pc, M, layers, N, keep)
    return dict(R=R, C=I / (M * R * R), Pc=Pc, info=info, prof=prof)
