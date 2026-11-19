#!/usr/bin/env bash
# Downloads the raw inputs of the opacity table into raw/ (git-ignored):
# HITRAN line lists of H2O and CO2 (main isotopologues, 0-5000 cm-1) through
# HITRAN's public line-by-line API, the HITRAN partition sums (TIPS),
# and MT_CKD 4.3's water continuum coefficients. Anonymous requests: no
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
  for lo in $(seq 0 250 4750); do
    hi=$((lo + 250))
    out="raw/lines/${name}_${lo}_${hi}.par"
    [[ -s "$out" ]] && continue
    # The server's Content-Length overstates the body by one byte per line,
    # so curl reports a short transfer (exit 18) on a complete file.
    curl -g -sS -A "$agent" -o "$out" \
      "https://hitran.org/lbl/api?iso_ids_list=${ids}&numin=${lo}&numax=${hi}&head=False&fixwidth=0&sep=[comma]&request_params=par_line" \
      || [[ $? -eq 18 ]]
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
