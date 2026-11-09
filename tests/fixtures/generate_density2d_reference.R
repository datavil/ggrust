# Reference contour_var / breaks contours from ggplot2's stat_density_2d, for
# tests/test_contour.py.
#
# Run from the repository root (needs ggplot2, MASS, jsonlite):
#   Rscript tests/fixtures/generate_density2d_reference.R
# Writes tests/fixtures/density2d_reference.json: the data, the bandwidths ggplot2
# uses (MASS::bandwidth.nrd; kde2d's normal sd is h / 4), then per case the
# bounding box of every line level (lines) or the levels of the bands (filled).

suppressPackageStartupMessages({
  library(ggplot2)
  library(jsonlite)
})
pdf(NULL) # ggplot_build may open a device; write no Rplots.pdf

set.seed(7)
frame <- data.frame(
  x = round(c(rnorm(150, 0, 1), rnorm(90, 3, 0.6)), 6),
  y = round(c(rnorm(150, 0, 0.8), rnorm(90, 2, 0.5)), 6)
)
h <- c(MASS::bandwidth.nrd(frame$x), MASS::bandwidth.nrd(frame$y))

line_boxes <- function(contour_var, breaks) {
  plot <- ggplot(frame, aes(x, y)) +
    geom_density_2d(contour_var = contour_var, breaks = breaks, h = h)
  data <- layer_data(plot)
  lapply(split(data, data$level), function(level) list(
    level = level$level[1],
    pieces = length(unique(level$piece)),
    xmin = min(level$x), xmax = max(level$x),
    ymin = min(level$y), ymax = max(level$y)
  ))
}

band_levels <- function(contour_var, breaks) {
  plot <- ggplot(frame, aes(x, y)) +
    geom_density_2d_filled(contour_var = contour_var, breaks = breaks, h = h)
  data <- layer_data(plot)
  sort(unique(data$level_mid))
}

cases <- list(
  list(contour_var = "density", breaks = c(0.01, 0.03, 0.06, 0.1)),
  list(contour_var = "ndensity", breaks = c(0.1, 0.3, 0.5, 0.7, 0.9)),
  list(contour_var = "count", breaks = c(2, 5, 10, 20))
)
out <- list(
  x = frame$x,
  y = frame$y,
  h = h,
  cases = lapply(cases, function(case) list(
    contour_var = case$contour_var,
    breaks = case$breaks,
    lines = unname(line_boxes(case$contour_var, case$breaks)),
    bands = band_levels(case$contour_var, case$breaks)
  ))
)
write_json(out, "tests/fixtures/density2d_reference.json", auto_unbox = TRUE, digits = NA)
