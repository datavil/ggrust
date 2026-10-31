# Benchmarks

`benchmarks/run.py` measures ggrust against lets-plot on **identical work**:

- same polars frame, same scatter plot (`geom_point(size=1)`), same size (600 x 400)
- lets-plot runs with `sampling="none"`: by default it silently draws at most 100k
  points (see `memory/letsplot_default_sampling.md`); ggrust always draws every point
- PNG at scale 2 for both (lets-plot's default)
- each measurement in a fresh process: **cold** = first call (includes start-up work),
  **warm** = median of the following calls
- **memory** = peak resident set above the level reached after building the data

Run: `uv run python benchmarks/run.py` (raw results land in `benchmarks/results/`).

## Results: 2026-10-25, Linux x86-64, 4 cores, Python 3.13, lets-plot 4.11

| output | points | ggrust warm | lets-plot warm | speed-up | ggrust memory | lets-plot memory |
|---|---:|---:|---:|---:|---:|---:|
| SVG | 10,000 | 0.003 s | 0.125 s | 42x | 4.7 MB | 49 MB |
| SVG | 100,000 | 0.026 s | 1.357 s | 52x | 13 MB | 335 MB |
| SVG | 1,000,000 | 0.279 s | 16.877 s | 60x | 84 MB | 3,005 MB |
| PNG | 10,000 | 0.053 s | 1.267 s | 24x | 16 MB | 186 MB |
| PNG | 100,000 | 0.279 s | 9.320 s | 33x | 17 MB | 984 MB |
| PNG | 1,000,000 | 2.448 s | 101.086 s | 41x | 31 MB | 8,822 MB |

Cold times are within a few percent of warm for both libraries (see the raw files).
Known ggrust hot spots are listed in `plan.md` (Phase 6): marker rasterisation
(~1.3 µs per marker), integer-to-float column casts, string level hashing.

## Scenarios: 2026-10-27, macOS arm64 (Apple M4 Pro, 12 cores), Python 3.13, lets-plot 4.11

`benchmarks/scenarios.py`: six common scatter variants (plain, discrete colour, continuous
colour, size + alpha, log scales, title + bottom legend), same data and `sampling="none"`.
Warm = median of 2 repeats after a cold call, fresh process each. Memory is the peak above
the data level, cumulative per process (the PNG row includes the SVG stage). Spec build is
lazy in both libraries (under 0.1 ms) and omitted below. Raw: `benchmarks/results/scenarios-*`.
Side-by-side figure: `python benchmarks/compare_images.py <images dir>`.

| scenario | points | stage | ggrust (s) | lets-plot (s) | speed-up | ggrust MB | lets-plot MB | memory ratio |
|---|---:|---|---:|---:|---:|---:|---:|---:|
| color_continuous | 10,000 | svg | 0.0032 | 0.0947 | 29x | 11 | 51 | 4.8x |
| color_continuous | 10,000 | png | 0.0438 | 0.7134 | 16x | 24 | 275 | 11.6x |
| color_discrete | 10,000 | svg | 0.0030 | 0.0939 | 32x | 11 | 49 | 4.6x |
| color_discrete | 10,000 | png | 0.0435 | 0.6794 | 16x | 24 | 255 | 10.6x |
| log_scales | 10,000 | svg | 0.0023 | 0.0817 | 35x | 11 | 49 | 4.7x |
| log_scales | 10,000 | png | 0.0416 | 0.6410 | 15x | 23 | 244 | 10.4x |
| scatter | 10,000 | svg | 0.0018 | 0.0816 | 45x | 11 | 45 | 4.3x |
| scatter | 10,000 | png | 0.0369 | 0.6457 | 18x | 23 | 247 | 10.5x |
| size_alpha | 10,000 | svg | 0.0038 | 0.0974 | 26x | 11 | 56 | 5.2x |
| size_alpha | 10,000 | png | 0.1198 | 1.3546 | 11x | 24 | 257 | 10.8x |
| titled_themed | 10,000 | svg | 0.0040 | 0.1039 | 26x | 11 | 55 | 5.2x |
| titled_themed | 10,000 | png | 0.0481 | 0.7827 | 16x | 25 | 269 | 11.0x |
| color_continuous | 100,000 | svg | 0.0305 | 1.1153 | 37x | 44 | 442 | 10.0x |
| color_continuous | 100,000 | png | 0.2410 | 4.8536 | 20x | 58 | 1299 | 22.4x |
| color_discrete | 100,000 | svg | 0.0298 | 0.9797 | 33x | 44 | 386 | 8.7x |
| color_discrete | 100,000 | png | 0.2463 | 4.4964 | 18x | 56 | 1252 | 22.2x |
| log_scales | 100,000 | svg | 0.0209 | 0.9090 | 43x | 22 | 410 | 18.7x |
| log_scales | 100,000 | png | 0.2502 | 4.2519 | 17x | 30 | 1210 | 40.7x |
| scatter | 100,000 | svg | 0.0161 | 0.9109 | 57x | 21 | 375 | 18.2x |
| scatter | 100,000 | png | 0.2326 | 4.2343 | 18x | 26 | 1293 | 50.0x |
| size_alpha | 100,000 | svg | 0.0373 | 1.0562 | 28x | 56 | 431 | 7.7x |
| size_alpha | 100,000 | png | 0.9072 | 11.6295 | 13x | 71 | 1314 | 18.6x |
| titled_themed | 100,000 | svg | 0.0373 | 1.1731 | 31x | 53 | 474 | 8.9x |
| titled_themed | 100,000 | png | 0.3049 | 5.2960 | 17x | 67 | 1423 | 21.1x |

## cellestial docstring examples: 2026-10-29, macOS arm64 (Apple M4 Pro), Python 3.14, lets-plot 4.11

`benchmarks/cellestial_examples.py`: nine plots copied from cellestial's docstring `Examples`,
on its pbmc3k dataset (17,041 cells, under every lets-plot sampling limit, so both draw every row).
The same cellestial code runs in two environments: the `ggrust-port` worktree (imports swapped,
see `memory/cellestial_port_check.md`) and the poetry env on real lets_plot, same package versions.
Fresh process per case, warm = median of 5 after a cold call. `build` is the cellestial call
(its own frame building, identical on both sides). svg / png time the render of a built plot
(PNG at scale 2). Render MB = peak memory of both renders above the build peak.

| case | build (s) | svg ggrust (s) | svg lets-plot (s) | svg speed-up | png ggrust (s) | png lets-plot (s) | png speed-up | render MB ggrust | render MB lets-plot |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| dimensional_categorical | 0.0021 | 0.0054 | 0.2053 | 38x | 0.0560 | error | | 30 | 96 (svg only) |
| dimensional_continuous | 0.0202 | 0.0067 | 0.1827 | 27x | 0.0502 | 0.9131 | 18x | 19 | 413 |
| dimensionals_grid | 0.0238 | 0.0244 | 0.9940 | 41x | 0.1819 | 3.8562 | 21x | 75 | 1419 |
| scatter | 0.0020 | 0.0032 | 0.4119 | 129x | 0.1005 | 1.5648 | 16x | 31 | 445 |
| violin | 0.0198 | 0.0035 | 0.0619 | 18x | 0.0273 | 0.6513 | 24x | 18 | 254 |
| dotplot | 0.0237 | 0.0006 | 0.0035 | 6x | 0.0089 | 0.3034 | 34x | 23 | 196 |
| stacked_violin | 0.0386 | 0.0014 | 0.0230 | 17x | 0.0121 | 0.3471 | 29x | 35 | 211 |
| heatmap | 0.0345 | 0.0030 | 0.0586 | 20x | 0.0115 | 0.3500 | 30x | 24 | 222 |
| volcano | 0.0028 | error | 4.9661 | | error | 11.4221 | | error | 204 |

Errors: lets-plot cannot export the on-data legend to PNG ("Unsupported color value: none");
ggrust has no `geom_text_repel` yet (plan step 5.1), which volcano needs. lets-plot SVGs are
2-4x larger for the point-heavy cases (1.2 MB vs 2.9 MB for one UMAP).

## Profile (step 6.1): 2026-10-31, macOS arm64 (Apple M4 Pro, 12 cores), Python 3.14

`benchmarks/profiling.py` splits one warm render into Python stages and the Rust engine call,
records memory (how much the renders raise the peak RSS over the build; `tracemalloc` peak of one
render) and samples a render loop with macOS `sample` into flamegraphs (`inferno`, demangled with
Xcode's `llvm-cxxfilt`; sleeping threads and polars' own thread pool left out; see
`memory/profiling_macos.md`). Cases: the nine cellestial docstring examples (pbmc3k, 2.7k cells)
and three scenarios (`scatter`, `color_continuous` at 1M points, `size_alpha` at 100k). Shares
below are of on-CPU samples. Raw: `benchmarks/results/profile-20261031/` (table, JSON with the top
frames of every case, seven representative flamegraphs).

Python is never the cost: `as_dict` + `to_json` + Arrow export + glue stay between 0.2 and 1.5 ms
in every case, and inside cellestial's 25 to 45 ms `build` ggrust's Python code takes under 1 ms
(the rest is cellestial, scipy and polars). Everything below is the Rust engine.

| case | svg ms | png ms | html ms | html RSS MB | where the time goes |
|---|---:|---:|---:|---:|---|
| dimensional_continuous | 6 | 50 | 19 | 10 | svg: `push_decimal` 37%; png: deflate 53%; html: tooltip text 49% |
| dimensionals_grid | 23 | 182 | 71 | 45 | png: deflate 50%; html: tooltip text 43% |
| scatter | 3 | 101 | 14 | 11 | svg: `push_decimal` 61%; png: deflate 51%, raster 42% |
| violin | 3 | 30 | 7 | 6 | svg: KDE 61% (libm `exp` 58%); png: deflate 60% |
| stacked_violin | 1 | 13 | 1 | 4 | png: raster 43%, PNG row filter 26% |
| heatmap | 3 | 13 | 3 | 4 | no dominant frame |
| volcano | 75 | 211 | 183 | 42 | svg: `push_decimal` 33%; png: deflate 40%; html: tooltip text 63% |
| scatter 1M | 155 | 2,079 | 914 | 435 | svg: `push_decimal` 71%; png: raster 90%; html: tooltip text 74% |
| color_continuous 1M | 285 | 2,106 | 1,649 | 952 | svg: `push_decimal` 43%, RSS 636 MB; html: tooltip text 85% |
| size_alpha 100k | 35 | 891 | 258 | 458 | png: raster 72%, deflate 23%; html: tooltip text 44% |

Hot spots, by expected win:

1. **HTML tooltip text is built eagerly per row** (`tooltip::content::layer_content` ->
   `aesthetic_values` -> `format_general`, one `String` per row and line through `format!`).
   It is 28 to 85% of every HTML render with points and most of HTML's memory (1M points: 0.9 to
   1.6 s and 435 to 952 MB, while the SVG of the same plot takes 0.16 to 0.29 s).
2. **PNG deflate**: the main PNG encoder uses the `png` crate default (`Balanced`, miniz);
   `compress_inner` is 40 to 60% of the PNG time in six of nine cellestial cases. `geom_imshow`
   already uses the fastest deflate (step 5.2); parity is visual, so the same applies here.
3. **Marker rasterisation** (step 6.1c): 72 to 90% of PNGs with 100k+ points.
4. **SVG number writing**: `push_decimal` is 33 to 71% of SVG renders with points (about 10% of
   the others); it calls `write!` twice and allocates a `format!` string per fractional number
   (the malloc/free/memmove frames follow it).
5. **KDE** (`stat::density::estimate_bins`): 61% of a violin SVG render, nearly all libm `exp`.
6. **Memory of large SVGs**: `color_continuous` 1M peaks 636 MB above the build for a 71 MB SVG
   (`scatter` 1M: 295 MB for 42 MB), worth a heap profile.

Rayon overhead is small: ggrust's own idle workers spinning (`swtch_pri`) are at most 15% of CPU
samples (violin HTML), mostly 0 to 6%. The polars pool loaded by cellestial spins far more (up to
73% of all samples in heatmap) and is excluded above.
