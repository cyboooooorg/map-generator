#!/usr/bin/env bash
# Downloads what crates/science/data/rfmip-profiles.tsv and the RFMIP tests'
# quoted reference values are built from into
# raw/ (git-ignored, about 11 MB): the RFMIP-IRF atmospheric profiles
# (input4MIPs UColorado-RFMIP-1-2, Pincus et al. 2016) and the clear-sky
# shortwave fluxes of the rad-irf experiment computed by LBLRTM-12-8 (the
# line-by-line reference) and RTE-RRTMGP-181204, from the ESGF node of
# ORNL. The netCDF files are checked against SHA256SUMS (the checksums ESGF
# publishes); the tool reads the same variables as OPeNDAP text views of
# those files, served by the same node, whose checksums are recorded too.
# Anonymous requests: no account, no personal data.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p raw
agent="rfmip-shortwave-builder"
server="https://esgf-node.ornl.gov/thredds"
input="user_pub_work/input4MIPs/CMIP6/RFMIP/UColorado/UColorado-RFMIP-1-2/atmos/fx/multiple/none/v20190401/multiple_input4MIPs_radiation_RFMIP_UColorado-RFMIP-1-2_none.nc"
output() { # model version variable
  echo "css03_data/CMIP6/RFMIP/$4/$1/rad-irf/r1i1p1f1/Efx/$3/gn/$2/$3_Efx_$1_rad-irf_r1i1p1f1_gn.nc"
}

get() { # url out
  [[ -s "$2" ]] && return 0
  curl -gsSfL -A "$agent" -o "$2.part" "$1" && mv "$2.part" "$2"
}

files=("$input")
for v in rsd rsu; do
  files+=("$(output LBLRTM-12-8 v20190514 "$v" AER)")
  files+=("$(output RTE-RRTMGP-181204 v20191007 "$v" RTE-RRTMGP-Consortium)")
done

for f in "${files[@]}"; do
  name="$(basename "$f")"
  get "$server/fileServer/$f" "raw/$name"
  case "$name" in
    multiple_input4MIPs*)
      vars="profile_weight,solar_zenith_angle,total_solar_irradiance,surface_albedo,pres_level,pres_layer,temp_level,water_vapor,carbon_dioxide_GM"
      ;;
    rsd_*) vars="rsd" ;;
    rsu_*) vars="rsu" ;;
  esac
  get "$server/dodsC/$f.ascii?$vars" "raw/${name%.nc}.txt"
done

shasum -a 256 -c --quiet SHA256SUMS
