"""Write the YREC namelists of the main-sequence grid: 0.3-8.0 Msun x 4 [Fe/H].

Usage:
    python prep_grid.py [OUT_DIR]

OUT_DIR defaults to the `namelists/` directory next to this script. The
shipped YREC grid namelists are read from `$YREC_DATA/grid` (default
`~/.config/yrec/grid`, the data directory of the `yrec` pip package).

The shipped grid has `*_RGB` namelists for 0.3-1.9 Msun and `*_TAMS` ones
for 3.0-8.0 Msun. The missing points (2.0-2.9 Msun, and 1.8-1.9 Msun at
[Fe/H] -1 and -0.5) are derived from the 3.0 Msun TAMS namelist of the
same [Fe/H]: same physics, the mass rescaled, and the nearest lower start
model. Every run stops at a central hydrogen fraction X_c = 1e-4 (TAMS)
or at 20 Gyr, and stores the structure every 20 models.

Start models are referred to as `{YREC_START}/...`: the YREC executable
expands it (the `yrec` package sets it to `$YREC_DATA/startmodels`).
"""

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.environ.get("YREC_DATA", os.path.expanduser("~/.config/yrec"))
GRID = os.path.join(DATA, "grid")
START = os.path.join(DATA, "startmodels")
FEHS = ["-1p0", "-0p5", "+0p0", "+0p5"]
MASSES = [m / 10 for m in range(3, 81)]


def tag(m):
    return f"{int(m)}p{int(round(m * 10)) % 10}"


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "namelists")
    os.makedirs(out, exist_ok=True)
    starts = sorted(
        int(f[1:5])
        for f in os.listdir(os.path.join(START, "dbl_grid"))
        if f.endswith(".first")
    )
    made = []
    for feh in FEHS:
        for m in MASSES:
            name = f"Grid_m{tag(m)}feh{feh}GS98_std25"
            src = None
            for kind in ("RGB", "TAMS"):
                if os.path.exists(f"{GRID}/{name}_{kind}.nml1"):
                    src = f"{GRID}/{name}_{kind}"
                    break
            derived = src is None
            if derived:
                src = f"{GRID}/Grid_m3p0feh{feh}GS98_std25_TAMS"
            n1 = open(src + ".nml1").read()
            n2 = open(src + ".nml2").read()
            n1 = n1.replace(os.path.basename(src), name)
            n1 = re.sub(r"\{YREC_INPUT\}/models/", "{YREC_START}/", n1)
            if derived:
                n1 = re.sub(r"(RSCLM\(1\) = )[0-9.]+", rf"\g<1>{m}", n1)
                s = max(x for x in starts if x <= round(m * 1000))
                n1 = re.sub(r"dbl_grid/m\d{4}", f"dbl_grid/m{s:04d}", n1)
            n1 = re.sub(r"(?m)^ *NMODLS\(2\) *=.*", " NMODLS(2) = 20000", n1)
            n1 = re.sub(
                r"(?m)^ *! *END_XCEN\(2\) = 0\.0001.*", " END_XCEN(2) = 0.0001", n1
            )
            n1 = re.sub(r"(?m)^ *! *ENDAGE\(2\) =.*", " ENDAGE(2) = 2.0D10", n1)
            n1 = re.sub(r"(?m)^ *NPRTMOD *=.*", " NPRTMOD = 20", n1)
            first = re.search(r'FFIRST = +"([^"]+)"', n1).group(1)
            path = first.replace("{YREC_START}", START)
            assert os.path.exists(path), path
            with open(os.path.join(out, f"{name}.nml1"), "w") as f:
                f.write(n1)
            with open(os.path.join(out, f"{name}.nml2"), "w") as f:
                f.write(n2)
            made.append(derived)
    print(len(made), "namelist pairs, derived:", sum(made))


if __name__ == "__main__":
    main()
