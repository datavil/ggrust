# Reference numbers from the R packages ggdist and gghalves, for
# crates/ggrust-core/src/stat/ggdist_density.rs and tests/test_halves_ggdist.py.
#
# Run from the repository root (needs ggplot2 3.5, ggdist 3.3.3, gghalves 0.1.4,
# jsonlite):
#   Rscript tests/fixtures/generate_ggdist_reference.R
# Writes tests/fixtures/ggdist_reference.json:
# - samples: per sample its values, R's bw.SJ(method = "dpi") and bw.nrd0,
#   density_bounded() and density_unbounded() (n = 501) on their grids, and the
#   median_qi / mean_qi / mode_qi points and intervals at .width 0.5, 0.66, 0.95.
# - layers: ggplot_build data of gghalves and ggdist layers on `raincloud`.

suppressPackageStartupMessages({
  library(ggplot2)
  library(ggdist)
  library(gghalves)
  library(jsonlite)
})
pdf(NULL) # ggplot_build may open a device; write no Rplots.pdf

set.seed(7)
# Rounded so that the JSON decimals give the same doubles in Rust and Python.
samples <- list(
  normal = round(rnorm(60, 5, 1), 3),
  bimodal = round(c(rnorm(30, 3, 0.6), rnorm(30, 7, 0.8)), 3),
  skewed = round(rgamma(60, 2, 1 / 1.5) + 2, 3),
  bounded = round(runif(400), 4),
  large = round(rnorm(1200), 4),
  small = c(1.5, 2.25, 2.5, 4),
  integers = c(1, 2, 2, 3, 3, 3, 4, 5, 9)
)

widths <- c(0.5, 0.66, 0.95)
describe <- function(x) {
  bounded <- density_bounded(x, n = 501)
  unbounded <- density_unbounded(x, n = 501)
  intervals <- function(f) {
    r <- f(x, .width = widths)
    list(point = r[[1]], lower = r$.lower %||% r$ymin, upper = r$.upper %||% r$ymax)
  }
  qi <- function(f) {
    r <- f(x, .width = widths)
    list(point = r$y, lower = r$ymin, upper = r$ymax)
  }
  list(
    values = x,
    bw_dpi = bw.SJ(x, method = "dpi"),
    bw_nrd0 = bw.nrd0(x),
    bounded = list(x = bounded$x, y = bounded$y),
    unbounded = list(x = unbounded$x, y = unbounded$y),
    median_qi = qi(median_qi),
    mean_qi = qi(mean_qi),
    mode_qi = qi(mode_qi)
  )
}
`%||%` <- function(a, b) if (is.null(a)) b else a

raincloud <- data.frame(
  g = rep(c("A", "B", "C"), each = 60),
  k = rep(c("u", "v"), 90),
  v = c(samples$normal, samples$bimodal, samples$skewed)
)
columns <- c("x", "xmin", "xmax", "y", "ymin", "ymax", "lower", "middle", "upper",
             "violinwidth", "thickness", ".width", "group")
build <- function(layer, mapping = aes(g, v)) {
  d <- layer_data(ggplot(raincloud, mapping) + layer)
  d <- d[, intersect(columns, names(d)), drop = FALSE]
  as.list(d)
}
layers <- list(
  half_violin = build(geom_half_violin()),
  half_violin_dodge = build(geom_half_violin(), aes(g, v, fill = k)),
  half_boxplot = build(geom_half_boxplot()),
  halfeye_slab = build(stat_slab()),
  pointinterval = build(stat_pointinterval()),
  interval = build(stat_interval())
)

write_json(
  list(samples = lapply(samples, describe), layers = layers),
  "tests/fixtures/ggdist_reference.json",
  digits = NA, auto_unbox = TRUE
)
