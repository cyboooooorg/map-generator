#!/usr/bin/env bash
# Downloads the raw inputs of the opacity table into raw/ (git-ignored):
# HITRAN line lists of H2O and CO2 (main isotopologues, 0-14500 cm-1: the
# longwave table's 0-5000 and the shortwave table's 2000-14400) through
# HITRAN's public line-by-line API, the HITRAN partition sums (TIPS),
# MT_CKD 4.3's water continuum coefficients, and HITRAN's CO2-CO2
# collision-induced absorption. The three files the continuum tables
# are built from are checked against the SHA-256 recorded below. Anonymous requests: no
# account, no personal data.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p raw/lines raw/q
agent="opacity-table-builder"

# HITRAN global isotopologue ids: H2O 161, 181, 171, 162 (1-4);
# CO2 626, 636, 628, 627 (7-10).
for gas in h2o:1,2,3,4 co2:7,8,9,10; do
  name="${gas%%:*}"
  ids="${gas#*:}"
  for lo in $(seq 0 250 14250); do
    hi=$((lo + 250))
    out="raw/lines/${name}_${lo}_${hi}.par"
    [[ -s "$out" || -e "$out.empty" ]] && continue
    # The server's Content-Length overstates the body by one byte per line,
    # so curl reports a short transfer (exit 18) on a complete file. A
    # piece without lines (CO2 has none at 13000-13500 and above 14250
    # cm-1) is answered 404 with an HTML page: it is kept as an empty file.
    code=$(curl -g -sS -A "$agent" -o "$out" -w '%{http_code}' \
      "https://hitran.org/lbl/api?iso_ids_list=${ids}&numin=${lo}&numax=${hi}&head=False&fixwidth=0&sep=[comma]&request_params=par_line") \
      || [[ $? -eq 18 ]]
    if [[ "$code" == 404 ]]; then
      : > "$out"
      touch "$out.empty"
    fi
  done
done

for id in 1 2 3 4 7 8 9 10; do
  [[ -s "raw/q/q${id}.txt" ]] || curl -sS -A "$agent" -o "raw/q/q${id}.txt" "https://hitran.org/data/Q/q${id}.txt"
done

[[ -s raw/absco-ref_wv-mt-ckd.nc ]] || curl -sSL -A "$agent" -o raw/absco-ref_wv-mt-ckd.nc \
  "https://raw.githubusercontent.com/AER-RC/MT_CKD_H2O/4.3/data/absco-ref_wv-mt-ckd.nc"

# MT_CKD 4.3 (AER-RC/MT_CKD, tag 4.3) takes LBLRTM at this commit
# ("Changes for MT_CKD_4.1", 2022-12-28); its contnm.f90 holds the CO2
# continuum (BLOCK DATA BFCO2, XFACCO2, FRNCO2).
[[ -s raw/contnm.f90 ]] || curl -sSL -A "$agent" -o raw/contnm.f90 \
  "https://raw.githubusercontent.com/AER-RC/LBLRTM/f66bb596e1686e5ff162a6fadc014e91d3ce01d4/src/contnm.f90"

# HITRAN2024's CO2-CO2 collision-induced absorption, binned into
# crates/science/data/co2-cia.tsv by `opacity-table --continua`.
[[ -s raw/CO2-CO2_2024.cia ]] || curl -sSL -A "$agent" -o raw/CO2-CO2_2024.cia \
  "https://hitran.org/data/CIA/main/CO2-CO2_2024.cia"

# The inputs of the continuum tables, as downloaded on 2026-11-18 (MT_CKD)
# and 2026-11-22 (CIA). A mismatch fails the script: HITRAN's server
# serves its current edition, so a changed CIA file means a new edition
# to review before the tables are regenerated.
(cd raw && shasum -a 256 -c - <<'SUMS'
69944eb8b045c268e2daeb2cddf536b99f9e9efe7e91c425067b46a5b287ddb3  absco-ref_wv-mt-ckd.nc
643f4173063d51759a0ea9e5715d0fcbd4066902452a2a5e56f62d37de69bcf7  contnm.f90
9db0856cb008cb147728e3a945f818c60a02370442d492c38d7f8884c9cfadca  CO2-CO2_2024.cia
SUMS
)
