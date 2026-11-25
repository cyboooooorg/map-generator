#!/usr/bin/env bash
# Downloads the stellar spectra binned into crates/science/data/stellar-spectra.tsv
# into raw/ (git-ignored, about 1.9 GB): the PHOENIX-ACES-AGSS-COND-2011
# HiRes spectra (Husser et al. 2013) from 2300 to 12000 K and the ATLAS9
# ck04 spectra (Castelli & Kurucz 2003) from 13000 to 27000 K, at
# [M/H] -1, -0.5, 0 and +0.5, one surface gravity per temperature (see
# README.md). Anonymous requests: no account, no personal data. Each file
# is kept once downloaded; rerun to complete an interrupted download.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p raw/phoenix raw/ck04
agent="stellar-spectra-builder"
phoenix="https://ftp.gwdg.de/pub/misc/phoenix/HiResFITS"
ck04="https://archive.stsci.edu/hlsps/reference-atlases/cdbs/grid/ck04models"

get() { # url out
  [[ -s "$2" ]] && return 0
  curl -sSfL -A "$agent" -o "$2.part" "$1" && mv "$2.part" "$2"
}

get "$phoenix/WAVE_PHOENIX-ACES-AGSS-COND-2011.fits" raw/phoenix/WAVE_PHOENIX-ACES-AGSS-COND-2011.fits

# The main-sequence surface gravity rounded to PHOENIX's 0.5 dex step (see
# README.md): 5.0 below 4000 K, 4.5 below 6400 K, 4.0 above.
logg() {
  if (( $1 < 4000 )); then echo 5.00; elif (( $1 < 6400 )); then echo 4.50; else echo 4.00; fi
}

temperatures="$(seq 2300 100 7000) $(seq 7200 200 12000)"
for z in -1.0 -0.5 -0.0 +0.5; do
  dir="Z${z}"
  for t in $temperatures; do
    name="lte$(printf '%05d' "$t")-$(logg "$t")${z}.PHOENIX-ACES-AGSS-COND-2011-HiRes.fits"
    get "$phoenix/PHOENIX-ACES-AGSS-COND-2011/${dir//+/%2B}/$name" "raw/phoenix/$name" &
    while (( $(jobs -r | wc -l) >= 4 )); do wait -n; done
  done
done
wait

for z in m10 m05 p00 p05; do
  for t in $(seq 13000 1000 27000); do
    get "$ck04/ck${z}/ck${z}_${t}.fits" "raw/ck04/ck${z}_${t}.fits"
  done
done

# The files as downloaded on 2026-11-25 (SHA256SUMS). A mismatch fails the
# script: a changed file is a new release to review before the table is
# regenerated.
(cd raw && shasum -a 256 -c --quiet ../SHA256SUMS)
