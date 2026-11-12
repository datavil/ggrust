# Third-party notices

## lets-plot

Parts of `python/ggrust/` (the spec-building API: `plot/core.py`, `plot/plot.py`,
`plot/label.py`, `plot/guide.py`, `plot/geom.py`, `plot/util.py`, `plot/series_meta.py`,
`mapping.py`) are ported from lets-plot's Python package, and parts of `crates/ggrust-core`
(break generation, number formatting, color names, theme constants) are ported from its
Kotlin sources. https://github.com/JetBrains/lets-plot

```
MIT License

Copyright (c) 2019 JetBrains

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Liberation Sans and Liberation Mono

`crates/ggrust-core/fonts/` contains Liberation Sans and Liberation Mono 2.1.5, licensed under the SIL Open Font
License 1.1; see `crates/ggrust-core/fonts/OFL.txt`.

## Liberation Serif (tests only)

`tests/fonts/` contains Liberation Serif 2.1.5 Regular and Bold, used only by the tests
of custom fonts (`register_font`), licensed under the SIL Open Font License 1.1; see
`tests/fonts/OFL.txt`. They are not shipped in the package.

## Color maps

`crates/ggrust-core/src/scale/style/colormap_data.rs` holds color map tables taken from
lets-plot (above), which credits them as follows. The color maps `viridis`, `magma`,
`inferno` and `plasma` were created by Stéfan van der Walt and Nathaniel Smith. The color
map `cividis` is a corrected version of `viridis`, developed by Jamie R. Nuñez,
Christopher R. Anderton and Ryan S. Renslow. The color map `turbo` was developed by Anton
Mikhailov. The color map `twilight` was created by Bastian Bechtold
(https://github.com/bastibe/twilight).

## ColorBrewer

`crates/ggrust-core/src/scale/style/brewer_data.rs` holds the ColorBrewer color schemes,
taken from lets-plot (above); `python/ggrust/bistro/funky_heatmap.py` repeats the few
funkyheatmap's default palettes are built from. Copyright (c) 2002 Cynthia Brewer, Mark Harrower, and The
Pennsylvania State University. Licensed under the Apache-Style Software License for
ColorBrewer Color Schemes, Version 1.1. https://colorbrewer2.org

## funkyheatmap

`python/ggrust/bistro/funky_heatmap.py` ports the layout, defaults and palettes of the R
package funkyheatmap 0.5 (https://github.com/funkyheatmap/funkyheatmap), and
`docs/data/scib_icons/` holds its three scIB output icons (`matrix.png`, `embedding.png`,
`graph.png`, downscaled to 96 px).

```
MIT License

Copyright (c) 2022 funkyheatmap authors (Robrecht Cannoodt, Wouter Saelens)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## scIB benchmark data

`docs/data/scib_immune.csv` holds the human immune cell task metrics of the scIB
benchmark (Luecken et al. 2022, Nature Methods), from
https://github.com/theislab/scib-reproducibility (MIT License, Copyright (c) 2020
Theis Lab).

## Example datasets (`ggrust.data`)

The CSV files in `python/ggrust/data/` are written by `scripts/make_datasets.R`.

- `penguins.csv`: Palmer Archipelago (Antarctica) penguin data, as in the R package
  palmerpenguins (Horst, Hill and Gorman 2020; data collected by Dr. Kristen Gorman and
  the Palmer Station Long Term Ecological Research Network), taken from R's
  `datasets::penguins` with palmerpenguins' column names. CC0 1.0 (public domain
  dedication). https://allisonhorst.github.io/palmerpenguins/
- `mpg.csv` and `economics.csv`: the `mpg` and `economics` datasets of the R package
  ggplot2 4.0.1, MIT License, Copyright (c) 2020 ggplot2 authors.
  https://ggplot2.tidyverse.org. Their underlying data are US government works:
  `mpg` from the EPA fuel economy data (https://fueleconomy.gov), `economics` from
  the Federal Reserve Bank of St. Louis FRED series (https://fred.stlouisfed.org).
- `iris.csv`: Fisher's iris measurements (R. A. Fisher 1936, collected by Edgar
  Anderson), public domain, taken from R's `datasets::iris` with snake_case column
  names.

## jpeg-encoder (JPEG output)

JPEG output (`ggsave("plot.jpg")`) uses the Rust crate jpeg-encoder
(https://github.com/vstroebel/jpeg-encoder, MIT OR Apache-2.0), whose forward DCT comes
from the Independent JPEG Group's libjpeg. As the IJG license asks: this software is
based in part on the work of the Independent JPEG Group.
