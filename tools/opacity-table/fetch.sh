#!/usr/bin/env bash
# Downloads the raw inputs of the opacity table into raw/ (git-ignored):
# HITRAN line lists of H2O and CO2 (main isotopologues, 0-14500 cm-1: the
# longwave table's 0-5000 and the shortwave table's 2000-14400) and of CH4,
# O3 and N2O (0-5250 cm-1, the longwave table's only) through HITRAN's public
# line-by-line API, the HITRAN partition sums (TIPS),
# MT_CKD 4.3's water continuum coefficients, HITRAN's CO2-CO2, N2-N2,
# O2-O2, O2-N2, N2-CH4 and CH4-CH4 collision-induced absorption, and
# Serdyuchenko et al.'s ozone cross-sections (IUP Bremen). The files the
# continuum tables are built from and the ozone file are checked against
# the SHA-256 recorded below. Anonymous requests: no account, no personal data.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p raw/lines raw/q
agent="opacity-table-builder"

# HITRAN global isotopologue ids: H2O 161, 181, 171, 162 (1-4);
# CO2 626, 636, 628, 627 (7-10); O3 666, 668, 686, 667, 676 (16-20);
# CH4 211, 311, 212, 312 (32-35); N2O 446, 456, 546, 448, 447 (21-25).
# CH4, O3 and N2O enter the longwave table only, whose lines reach 5000
# cm-1 plus the 25 cm-1 cutoff.
for gas in h2o:1,2,3,4:14250 co2:7,8,9,10:14250 o3:16,17,18,19,20:5000 ch4:32,33,34,35:5000 \
  n2o:21,22,23,24,25:5000; do
  name="${gas%%:*}"
  rest="${gas#*:}"
  ids="${rest%:*}"
  last="${rest##*:}"
  for lo in $(seq 0 250 "$last"); do
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

for id in 1 2 3 4 7 8 9 10 16 17 18 19 20 21 22 23 24 25 32 33 34 35; do
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

# HITRAN's nitrogen, oxygen and methane pairs (the main set, and CH4-CH4
# from the alternate set, the only CH4-CH4 file HITRAN serves), binned
# into crates/science/data/pairs-cia.tsv by `opacity-table --continua`.
for file in main/N2-N2_2021 main/O2-O2_2024 main/O2-N2_2024 main/N2-CH4_2024 \
  alternate/CH4-CH4_2011; do
  name="${file#*/}.cia"
  [[ -s "raw/$name" ]] || curl -sSL -A "$agent" -o "raw/$name" \
    "https://hitran.org/data/CIA/${file}.cia"
done

# Ozone's cross-sections of Serdyuchenko et al. (2014), IUP Bremen (the
# "ASCII data, version 25.07.2012" of its O3 spectra page), reduced into
# crates/science/data/ozone.tsv by `opacity-table --ozone`.
[[ -s raw/serdyuchenkogorshelev5digits.dat ]] || curl -sSL -A "$agent" \
  -o raw/serdyuchenkogorshelev5digits.dat \
  "https://www.iup.uni-bremen.de/gruppen/molspec/downloads/serdyuchenkogorshelev5digits.dat"

# The inputs of the continuum tables, as downloaded on 2026-11-18
# (MT_CKD), 2026-11-22 (CO2-CO2) and 2026-11-25 (the other pairs), and
# the ozone file (2026-11-25). A
# mismatch fails the script: HITRAN's server serves its current edition,
# so a changed CIA file means a new edition to review before the tables
# are regenerated.
(cd raw && shasum -a 256 -c - <<'SUMS'
69944eb8b045c268e2daeb2cddf536b99f9e9efe7e91c425067b46a5b287ddb3  absco-ref_wv-mt-ckd.nc
643f4173063d51759a0ea9e5715d0fcbd4066902452a2a5e56f62d37de69bcf7  contnm.f90
9db0856cb008cb147728e3a945f818c60a02370442d492c38d7f8884c9cfadca  CO2-CO2_2024.cia
f0525f75c667802e8dbbfd7054a0a7e07863d8a4ffdb449fe80666bc658c849d  N2-N2_2021.cia
0146acfe5c1414b8deaa56be9a63159154479d5804d1615855888a2e1efd4961  O2-O2_2024.cia
90851fbfae226d7ecda93efc0de16105f72966a532adc7f1bc234cb13833799f  O2-N2_2024.cia
7056ee0bc4a38a4a4d2b215b40312ba5be6029dd6098bf1b1c76d5522eb12523  N2-CH4_2024.cia
12de769200685d2bb7faff7550b6276ee20c9e3d3af4f318d2dc46935d39403a  CH4-CH4_2011.cia
4dfbf021b746512c192df5f0d43c54cee6ea3b4365bb490bcf6ed347f0ce7092  serdyuchenkogorshelev5digits.dat
SUMS
)
