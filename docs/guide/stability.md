# Stability

From version 1.0 ggrust follows [semantic versioning](https://semver.org): code that
works on 1.x keeps working on every later 1.y. This page says what that promise covers
and what it does not.

## What is stable

A change that breaks any of these needs a new major version.

- **Names.** Every name in `ggrust.__all__` (what `from ggrust import *` gives), plus
  `ggrust.display`, the classes in `ggrust.errors`, the datasets in `ggrust.data` and
  the functions in `ggrust.bistro`.
- **Import paths.** `ggrust`, `ggrust.errors`, `ggrust.data`, `ggrust.bistro` (and its
  lets_plot submodules `corr`, `im`, `joint`, `qq`, `residual`), `ggrust.plot.core`
  (`PlotSpec`, `LayerSpec`, `FeatureSpec`, `FeatureSpecArray`, `aes`, `layer`) and
  `ggrust.plot.subplots` (`SupPlotsSpec`).
- **Signatures.** Parameter names, the order of positional parameters, and which
  parameters are keyword-only. New parameters are added at the end or as keyword-only,
  with a default that keeps the old behavior.
- **Objects.** The public methods and properties of `PlotSpec`, `LayerSpec`,
  `SupPlotsSpec` and the other exported classes, their return types, and `+` composition.
- **Errors.** The exception classes, what they subclass (`ValueError`, `TypeError`, ...)
  and their attributes (`path`, `column`, ...).
- **Plot files.** A `.ggrs` file written by any 1.x is read by every later 1.y.

## What may change in a minor release

- **Pixels.** The drawn output is not frozen: layout, default sizes, break positions,
  label formats and colors can change when that fixes a bug or brings ggrust closer to
  lets-plot or ggplot2. Every such change is listed in the changelog under *Changed*
  or *Fixed*. The known differences still open are in
  [deviations.md](https://github.com/datavil/ggrust/blob/main/deviations.md).
- **The spec dictionary.** `as_dict()` matches lets_plot for lets_plot features; keys
  for ggrust-only features can change.
- **Output bytes.** SVG, HTML, PNG and PDF files are not byte-stable, and the embedded
  JavaScript runtime is private.
- **Messages.** The wording of errors and warnings. Catch the class, not the text.
- **Options that warn as unsupported.** They may start to work.
- **Other module paths.** Any module not listed above (for example `ggrust.plot.geom` or
  `ggrust.kernel_density`) and any name starting with an underscore. Import from
  `ggrust` instead.
- **The Rust crates.** `ggrust-core` and `ggrust-python` are internal; only the Python
  package is covered.

## Provisional features

Provisional features are complete and tested, but new. Their names and parameters may
still change in a minor release, with a note in the changelog.

- **Animation:** `animate`, `Animation`, `anim_save`, `last_animation`, `transition_*`,
  `enter_*`, `exit_*`, `shadow_*` and `ease_aes`.

## Deprecation

Before a stable name or parameter is removed, it keeps working for at least one minor
release and emits a `DeprecationWarning` that names the replacement. The removal itself
waits for the next major version.
