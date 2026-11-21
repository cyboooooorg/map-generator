# Giant and mini-Neptune radius tables

Two tab-separated tables give the radius of a giant planet (10 to
3 594 M⊕) and of a low-mass planet with a hydrogen-helium envelope
(1 to 20 M⊕). They are read as they are published; nothing was fitted or
smoothed. This file says where every number comes from, how it was
converted, and how far it can be trusted. The study behind them is
`docs/superpowers/studies/2026-11-21-planet-model-2a/giants-and-envelopes.md`
(sections 1.2, 1.3 and 2.2).

| File | Content | Rows | Size (bytes) |
| --- | --- | --- | --- |
| `giants-fortney2007.tsv` | Radius and internal luminosity of a giant | 4 550 | 165 741 |
| `mini-neptunes-lopez2014.tsv` | Radius of a planet with an envelope | 792 | 15 592 |

Both start with `#` comment lines, then a header row, then one row per
line, fields separated by tabs.

## Giants: Fortney, Marley and Barnes 2007

Source: Fortney, J. J., Marley, M. S. and Barnes, J. W. 2007, "Planetary
Radii across Five Orders of Magnitude in Mass and Stellar Insolation",
ApJ 659, 1661
([doi:10.1086/512120](https://doi.org/10.1086/512120)). The data are the
25 files `models/tbl_Evpcore_<core>_<dist>.dat` linked from
[Models of Planetary Radii](https://www.ucolick.org/~jfortney/models.htm)
on the author's page, retrieved on 2026-11-21 with `curl` (no login).
The files total 326 016 bytes.

Columns of `giants-fortney2007.tsv`:

| Column | Unit | Meaning |
| --- | --- | --- |
| `core_me` | M⊕ | Heavy-element core: 0, 10, 25, 50 or 100 (50/50 ice and rock) |
| `dist_au` | AU | Distance to a solar-luminosity star: 0.02, 0.045, 0.1, 1 or 9.5 |
| `age_gyr` | Gyr | 0.0010, 0.0032, 0.0100, 0.0316, 0.1000, 0.3162, 1.0000, 3.1623, 10.0000 |
| `mass_me` | M⊕ | Planet mass, as printed in the `Me` column |
| `radius_rj` | R_J | Radius at 1 bar, the `Rj` column |
| `luminosity_erg_s` | erg s⁻¹ | Internal luminosity, the `L` column |

The core comes from the file name (`00b` and `00c` are the coreless
models, `00c` being the 0.02 AU file) and the distance from its suffix
(`point02AU` is 0.02 AU, `9point5AU` is 9.5 AU). The `Mj`, `g` and `Tint`
columns are not kept: the mass in M_J and the gravity follow from the
mass and the radius, and `Tint` is not read. Values are copied as
printed, including the exponent form of `L` (`1.74E+26`).

Rows exist only where the mass exceeds the core mass and, at young ages
and high irradiation, only where the model ran (the author's page says
"some very low-mass young models are excluded"). Rows per block (one
age): 18 to 25 without a core (the 0.02 AU file starts at 18 and gains
rows with age), 17 to 24 with a 10 M⊕ core, 20 to 21 with 25 M⊕, 18 with
50 M⊕ and 15 with 100 M⊕. The 25 files hold 9 blocks each, 4 550 rows.

### Conversion command

```bash
python3 -I convert.py <dir of the 25 .dat files> > rows.tsv
```

`convert.py` is:

```python
import sys,re,glob,os
d=sys.argv[1]
dist={'point02AU':'0.02','point045AU':'0.045','point1AU':'0.1','1AU':'1','9point5AU':'9.5'}
out=[]
counts={}
for core in ['00','10','25','50','100']:
  for dk in ['point02AU','point045AU','point1AU','1AU','9point5AU']:
    fn=glob.glob(f'{d}/tbl_Evpcore_{core}[abc]_{dk}.dat')
    assert len(fn)==1,(core,dk,fn)
    age=None; n={}
    for line in open(fn[0]):
      m=re.match(r't \(Gyr\) =\s+([0-9.]+)',line)
      if m: age=m.group(1); n[age]=0; continue
      f=line.split()
      if age and len(f)==6 and re.match(r'^[0-9.]+$',f[0]):
        out.append('\t'.join([str(int(core)),dist[dk],age,f[0],f[2],f[5]])); n[age]+=1
    assert len(n)==9
    counts[(core,dk)]=list(n.values())
print('\n'.join(out))
for k,v in counts.items(): print(k,v,sum(v),file=sys.stderr)
```

It reads each file in the order core 0, 10, 25, 50, 100 then
distance 0.02, 0.045, 0.1, 1, 9.5, starts a block at each line
`t (Gyr) = <age>`, and writes `core, dist, age, Me, Rj, L` for every
six-field numeric line, as strings, unchanged. The age strings are the
files' own (`0.0010`, `10.0000`). The header lines of
`giants-fortney2007.tsv` were prepended by hand.

### Misprint of the paper's Table 4

The paper's Table 4 (9.5 AU, core 25 M⊕, column 464 M⊕) reads 1.037,
larger than its neighbour at 774 M⊕ (1.035) and than the 10 M⊕ core row
(1.024). The `.dat` has no such error: the row reads 1.013 (3.16 Gyr) and
0.985 (10 Gyr). The table here is the `.dat`; the printed tables served
only as a check.

### Licence of the Fortney tables

None is stated on the author's page. The numbers are also printed in the
paper (4.5 Gyr, 1 Gyr and 300 Myr only), so cite both the paper and the
page. The repository holds the converted table, not the files.

### Reference level and validity of the Fortney tables

- Radius at 1 bar, not at the transit level (5 to 6 scale heights higher,
  Burrows et al. 2006, as quoted in the paper).
- Masses 10 to 3 594 M⊕ without a core, cores 0 to 100 M⊕, distances
  0.02 to 9.5 AU, ages 1 Myr to 10 Gyr. Outside, clamp or refuse; never
  extrapolate.
- For another star, the tables are read at the flux-equivalent distance
  `a / sqrt(L / L☉)`; the paper gives the idea (Eq. 11), the exact use
  is a choice of this repository, stated where it is used.
- No inflation, no deuterium burning, no helium rain; heavy elements are
  50/50 ice and rock only.

### Sha256 of the source files

| File | sha256 |
| --- | --- |
| `tbl_Evpcore_00b_1AU.dat` | `602b34caeb5874ad0548bf67ceb9f0171689eef7c29d0b9dc2f4115a0653f2cd` |
| `tbl_Evpcore_00b_9point5AU.dat` | `ea54d648c0554d9925a0d6c111eda0234d4c0a19c03c471badf5695959ab88c4` |
| `tbl_Evpcore_00b_point045AU.dat` | `c1ba4a189f87feba504b049b0fd229800b074dbdabb3dcab120dd6dae73d8cfc` |
| `tbl_Evpcore_00b_point1AU.dat` | `db74005584c72eda7649d51bb1ff0b2d8f9c890a284a442c5af222bb735ff018` |
| `tbl_Evpcore_00c_point02AU.dat` | `b1952dc8cf3b3c9f242fde7461929e575141d62fd9d5a18bb5a9df0f4d1e39e6` |
| `tbl_Evpcore_100a_1AU.dat` | `a683198b78812faf3ef38ffe9a7820f75d7b416bc803f22f6dc00b879a83e19e` |
| `tbl_Evpcore_100a_9point5AU.dat` | `5271efb78b2135d8841385784ca600c45a5ad4e934a326a811dbb6a442c10c25` |
| `tbl_Evpcore_100a_point02AU.dat` | `46dae2ca02d95864ac31ac9e0f2461b275999ba6d0f1da14310f2d2059cabadc` |
| `tbl_Evpcore_100a_point045AU.dat` | `0e51058ceebf2925cb47bac3fe3281ca02602b181ae55fc8c3d493a994104316` |
| `tbl_Evpcore_100a_point1AU.dat` | `686aad8f629fe66c34b5993c9de0752ab1fdcfcb0d22fc3971785dcb78a920fd` |
| `tbl_Evpcore_10a_1AU.dat` | `5411fe4d8d0115a6a32ac911d4040512dec256235ff729b191a37ff7c6fcd95d` |
| `tbl_Evpcore_10a_9point5AU.dat` | `ebcbe4792d6c3aceb178035a8cb58b95e10e2d1a78905a6a2a60fd9d3a356766` |
| `tbl_Evpcore_10a_point02AU.dat` | `0a0c0fa2e9af5f933c0ff5f0074a6cd513644b63af372005f4094a0085d54ef9` |
| `tbl_Evpcore_10a_point045AU.dat` | `1869e0120382967b5c161a2b401417a939738146efd6b2afbbaef0a622c37f58` |
| `tbl_Evpcore_10a_point1AU.dat` | `23f54d3df6f87fa3ad76a8e3257566eac6ba21584ad8b67bdcb9adb39746cf71` |
| `tbl_Evpcore_25a_1AU.dat` | `7dc8c0df3b6e5d47f85673453eb0b3e4ece38742d8e9bfa25b5c4af25a53e23f` |
| `tbl_Evpcore_25a_9point5AU.dat` | `9b8e4261c322193a9929c000d8169438d1b6ede3cd7665f115fd5b548f6f93e4` |
| `tbl_Evpcore_25a_point02AU.dat` | `bb5c76dae93f0640147fbf350af9b73139a03782dd13df4d15b6ad4cb6654fab` |
| `tbl_Evpcore_25a_point045AU.dat` | `78b8382915102da0f832343fbdbdc37bd31b6178fe8a9b767a8fdcec657ff8c8` |
| `tbl_Evpcore_25a_point1AU.dat` | `529b113c3d3d26fd710f60c6e99ed5acbac4e7365bb0c3f4caa46999a7404196` |
| `tbl_Evpcore_50a_1AU.dat` | `77102732100ce0d4f3ee11d9caa12472fa2d2c72c78b096b042c202867a2e130` |
| `tbl_Evpcore_50a_9point5AU.dat` | `80eb5babed025ca1629fd9a0c4a42a541824a4260d52c4160242df8462864f4d` |
| `tbl_Evpcore_50a_point02AU.dat` | `8ba1bee6f471bb87afb1ebe069b672d30f2a2949c7a62fd39719b86183656b58` |
| `tbl_Evpcore_50a_point045AU.dat` | `1a666e78523bd967003358eba1833191861b3cb9c2decb716d5de1b1604817c4` |
| `tbl_Evpcore_50a_point1AU.dat` | `19b3a0ce6b5ee8f005202c95bffc34031a652d625b105e0d25297d77272dfa13` |

## Mini-Neptunes: Lopez and Fortney 2014

Source: Lopez, E. D. and Fortney, J. J. 2014, "Understanding the Mass-Radius
Relation for Sub-Neptunes: Radius as a Proxy for Composition", ApJ 792, 1
([doi:10.1088/0004-637X/792/1/1](https://doi.org/10.1088/0004-637X/792/1/1),
[arXiv:1311.0329](https://arxiv.org/abs/1311.0329)). The paper has no data
file. Tables 2, 3 and 4 (solar-metallicity atmospheres at 100 Myr, 1 Gyr
and 10 Gyr) were transcribed on 2026-11-21 from the
[ar5iv HTML](https://ar5iv.labs.arxiv.org/html/1311.0329) (sha256
`aa7121cf83ce6bf70f20118907afb5d58be6d206f7a2b7279df4ad6908304481`).
Tables 5 to 7 (50 times solar opacity) are not kept.

Columns of `mini-neptunes-lopez2014.tsv`:

| Column | Unit | Meaning |
| --- | --- | --- |
| `age_gyr` | Gyr | 0.1, 1 or 10 |
| `flux_fe` | F⊕ | Incident flux: 0.1, 10 or 1000 |
| `mass_me` | M⊕ | 1, 1.5, 2.4, 3.6, 5.5, 8.5, 13 or 20 |
| `envelope_pct` | % | H/He envelope mass fraction: 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10 or 20 |
| `radius_re` | R⊕ | Planet radius, as printed |

3 ages times 3 fluxes times 8 masses times 11 fractions give 792 rows.
The conversion parses every `<tr>` of the three tables, checks the 11
column headings and that each row has 13 cells, and writes the cells as
strings, unchanged (`1.30`, `11.7`). No cell was ambiguous.

### Check against the PDF

The journal PDF (IOP) answers 403 to automated requests, so the check
used the [arXiv PDF](https://arxiv.org/pdf/1311.0329) (sha256
`c75c1fc88d23e1a3cb82d7055f024a3d7a6752c38603620333fa19544ed9cd01`),
whose text layer was extracted by decompressing the page streams. Every
one of the 72 rows (flux, mass and the 11 radii, 792 values) of the
three tables was found, character for character, in the PDF text; the
file is therefore equal to the arXiv version of the paper. Five of them,
as the spot check asked for (age, flux, mass, envelope, radius): 100 Myr,
10 F⊕, 5.5 M⊕, 1 %, 2.33; 100 Myr, 1000 F⊕, 20 M⊕, 20 %, 6.80; 1 Gyr,
10 F⊕, 2.4 M⊕, 0.5 %, 1.86; 1 Gyr, 0.1 F⊕, 13 M⊕, 5 %, 3.11; 10 Gyr,
1000 F⊕, 8.5 M⊕, 10 %, 4.10.

### Licence of the Lopez and Fortney tables

The paper is the source; the author states no data licence and the
values are printed in the paper, which is cited.

### Reference level and validity of the Lopez and Fortney tables

- Radius at 20 mbar (the paper's `R_p`, the core plus the convective
  envelope plus the isothermal atmosphere above it), not at 1 bar.
- Masses 1 to 20 M⊕, fluxes 0.1 to 1000 F⊕, envelope fractions 0.01 to
  20 % (the text covers up to 60 %, the tables stop at 20 %), ages 100
  Myr to 10 Gyr. Outside, clamp or refuse; never extrapolate.
- The envelope fractions are of the whole planet, the core being the
  rest.
- The fit of the paper's Eq. 3 is not used: the authors say it is not to
  replace the full models.
