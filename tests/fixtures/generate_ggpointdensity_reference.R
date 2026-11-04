# Reference point densities from the R package ggpointdensity, for
# tests/test_pointdensity.py.
#
# Run from the repository root (needs ggplot2, ggpointdensity 0.2.1, MASS, jsonlite):
#   Rscript tests/fixtures/generate_ggpointdensity_reference.R
# Writes tests/fixtures/ggpointdensity_reference.json: the data, then per case the
# `density` of every row (in data row order).

suppressPackageStartupMessages({
  library(ggplot2)
  library(ggpointdensity)
  library(jsonlite)
})
pdf(NULL) # ggplot_build may open a device; write no Rplots.pdf

# Two blobs, a ring and a uniform background, like a small UMAP. Rounded so that
# the JSON decimals give the same doubles in Python.
set.seed(42)
angle <- runif(150, 0, 2 * pi)
x <- c(rnorm(200, 0, 0.6), rnorm(120, 4, 0.3), 2 + 1.5 * cos(angle) + rnorm(150, 0, 0.1),
       runif(80, -2, 6))
y <- c(rnorm(200, 0, 0.4), rnorm(120, 3, 0.8), 5 + 1.5 * sin(angle) + rnorm(150, 0, 0.1),
       runif(80, -2, 7))
frame <- data.frame(
  x = round(x, 6),
  y = round(y, 6),
  g = rep(c("a", "b", "c", "d"), c(200, 120, 150, 80))
)
stopifnot(!anyDuplicated(frame[, c("x", "y")]))
# A second layer that widens the plot ranges (the neighbour radius follows them).
extra <- data.frame(x = c(-8, 10), y = c(-6, 12))

cases <- list(
  neighbors = list(aes(x, y), geom_pointdensity(method = "neighbors")),
  neighbors_adjust = list(aes(x, y), geom_pointdensity(method = "neighbors", adjust = 0.4)),
  neighbors_groups = list(aes(x, y, group = g), geom_pointdensity(method = "neighbors")),
  neighbors_wide = list(aes(x, y), geom_pointdensity(method = "neighbors"),
                        geom_point(data = extra)),
  auto = list(aes(x, y), geom_pointdensity()),
  kde2d = list(aes(x, y), geom_pointdensity(method = "kde2d")),
  kde2d_adjust = list(aes(x, y), geom_pointdensity(method = "kde2d", adjust = 2)),
  kde2d_groups = list(aes(x, y, group = g), geom_pointdensity(method = "kde2d")),
  kde2d_wide = list(aes(x, y), geom_pointdensity(method = "kde2d"), geom_point(data = extra))
)

key <- function(x, y) paste(format(x, digits = 15), format(y, digits = 15))
densities <- lapply(cases, function(case) {
  plot <- ggplot(frame, case[[1]])
  for (layer in case[-1]) plot <- plot + layer
  built <- ggplot_build(plot)$data[[1]]
  # Rows come back grouped; put them in data row order.
  built$density[match(key(frame$x, frame$y), key(built$x, built$y))]
})

write_json(
  list(data = frame, extra = extra, cases = densities),
  "tests/fixtures/ggpointdensity_reference.json",
  digits = NA
)
