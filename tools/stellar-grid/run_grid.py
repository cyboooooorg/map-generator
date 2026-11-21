"""Run every YREC namelist pair of the grid in parallel.

Usage:
    python run_grid.py OUT_DIR [NAMELIST_DIR] [WORKERS]

Outputs (`.track`, `.store`, ...) go to OUT_DIR, about 21 GB for the
312 runs. NAMELIST_DIR defaults to the `namelists/` directory next to
this script, WORKERS to 8. About 35 minutes on 8 cores.
"""

import glob
import os
import sys
import time

import yrec

HERE = os.path.dirname(os.path.abspath(__file__))


def main():
    out = os.path.abspath(sys.argv[1])
    nml = os.path.abspath(sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "namelists"))
    workers = int(sys.argv[3]) if len(sys.argv) > 3 else 8
    os.makedirs(out, exist_ok=True)
    # YREC writes scratch files (fort.*) to the working directory.
    os.chdir(out)
    os.environ["YREC_OUTPUT"] = out
    models = [
        yrec.Model(f, f[:-1] + "2") for f in sorted(glob.glob(os.path.join(nml, "*.nml1")))
    ]
    t = time.time()
    yrec.run_parallel(models, workers=workers)
    print(f"done {len(models)} in {time.time() - t:.0f}s")


if __name__ == "__main__":
    main()
