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
taken from lets-plot (above). Copyright (c) 2002 Cynthia Brewer, Mark Harrower, and The
Pennsylvania State University. Licensed under the Apache-Style Software License for
ColorBrewer Color Schemes, Version 1.1. https://colorbrewer2.org
