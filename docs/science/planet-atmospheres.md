# Planet atmospheres, climate and surface (part 2)

Part 2 of the physical planet model (issue #423) derives what a planet's
interior does not fix: whether it keeps its envelope, its tectonic
regime, atmosphere, climate, liquid and life. It is written section by
section as the tasks land; this file starts with the XUV fluence. The
interiors are in [planet-interiors.md](planet-interiors.md).

## XUV fluence

`planet::xuv::xuv_fluence` gives the XUV energy per unit area a planet
has received over its system's age, relative to Earth's (the Sun at
1 AU at 4.567 Gyr is 1.0). It is the budget that drives photoevaporation.

The star emits `L_XUV / L_bol = 10^-3` while saturated (Wright et al.
2011, doi:10.1088/0004-637X/743/1/48), for a time `t_sat = 0.1 Gyr ·
M^-2` (M in solar masses; 0.1 Gyr for the Sun from Jackson, Davis &
Wheatley 2012, doi:10.1111/j.1365-2966.2012.21384.x, the mass exponent is
a calibration giving 1.1 Gyr at 0.3 M☉ and 2.5 Gyr at 0.2 M☉), then
decays as `t^-1.23` (Ribas et al. 2005, doi:10.1086/427977). The
integral over the age is analytic and is multiplied by the insolation.
The luminosity is taken constant over the age.

Calibration targets: Earth is 1.0 (absolute value
`EARTH_XUV_FLUENCE_J_M2` = 1.5e16 J/m²); 5 to 10 Gyr adds under 30 %;
a 0.2 M☉ star at Earth's insolation delivers more than 10 times Earth's
fluence.
