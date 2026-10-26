# Default style scales: lets-plot rules (Phase 0.10.2c)

Verified point by point against lets-plot 4.11 (`tests/test_style_parity.py`).

- **color/fill, discrete**: ColorBrewer `Set1` (sets start at 3 colors; smaller sets are
  prefixes of the 9-color set). Over 9 categories: extra colors from
  `PaletteUtil.generateColors` (base hue cycled, HCL lightness shifted by +/-15, +/-30,
  ... wrapped into 10..90). Categories are joined across layers in first appearance.
- **color/fill, continuous**: LAB gradient `#132B43` -> `#56B1F7` over the data range;
  values outside the domain and missing -> `#808080`. Port the color-space math
  exactly (lets-plot's LUV formulas differ from the textbook CIE_K variant).
- **size**: linear onto diameters 3..21 px (aes units = px / 2.2); NA 1 px.
  **alpha**: linear onto 0.1..1; NA 0 (invisible!). Linear maps extrapolate outside
  the domain; a zero-width domain maps to the middle (`Mappers.linear`), and a single
  value is widened to +/- 0.5 first (`ensureApplicableDomain`).
- Near-constant ranges: lets-plot's `isBeyondPrecision` takes `log10(base)` of the raw
  end, NaN for negatives, so only *positive* near-constant ranges collapse to the
  middle. The style path copies that literally; axis breaks keep the magnitude test.
- Mixed kinds on one aesthetic: any discrete layer makes the scale discrete; numbers
  become categories in first-appearance order, keyed by f64 bits (2 != "2", -0.0 != 0.0).
- Discrete outputs cycle (`index % size`); discrete -> continuous spreads indices
  0..n-1 over the range (one category: the middle).
- lets-plot draws legend keys as extra `<circle>`s after the panel: compare only the
  first n circles. lets-plot writes alpha as a double, ggrust as 8-bit: < 1/255 apart.

Benchmark note: the worker machine changed on 2026-10-25 (about 2x slower raster on
untouched code). Re-run both libraries on the same machine before comparing timings.
