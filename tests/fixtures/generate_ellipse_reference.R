# Reference ellipses from ggplot2's stat_ellipse (ggplot2 3.5.2, MASS 7.3), for
# tests/test_ellipse.py.
#
# Run from the repository root (needs ggplot2, MASS, jsonlite):
#   Rscript tests/fixtures/generate_ellipse_reference.R
# Writes tests/fixtures/ellipse_reference.json: the data, then per case the
# ellipse x/y of every group (group order a, b, ...) and cov.trob's fit.

suppressPackageStartupMessages({
  library(ggplot2)
  library(jsonlite)
})
pdf(NULL)

# Two correlated clouds with a few outliers (so cov.trob iterates several times),
# and a group of three points (too few: no ellipse). Rounded so that the JSON
# decimals give the same doubles in Python.
set.seed(7)
n_a <- 60
a_x <- rnorm(n_a, 0, 1)
a_y <- 0.6 * a_x + rnorm(n_a, 0, 0.5)
a_x[1:4] <- c(5, -4, 4.5, 0.5)
a_y[1:4] <- c(-3, 4, 3.8, 6)
n_b <- 40
b_x <- rnorm(n_b, 6, 0.7)
b_y <- -0.8 * (b_x - 6) + rnorm(n_b, 3, 0.4)
frame <- data.frame(
  x = round(c(a_x, b_x, 1, 2, 3), 6),
  y = round(c(a_y, b_y, 1, 3, 2), 6),
  g = rep(c("a", "b", "c"), c(n_a, n_b, 3))
)

cases <- list(
  list(name = "t", type = "t", level = 0.95, segments = 51),
  list(name = "t_level_50", type = "t", level = 0.5, segments = 51),
  list(name = "norm", type = "norm", level = 0.95, segments = 51),
  list(name = "norm_segments_8", type = "norm", level = 0.9, segments = 8),
  list(name = "euclid", type = "euclid", level = 0.95, segments = 51)
)
results <- list()
for (case in cases) {
  plot <- ggplot(frame, aes(x, y, group = g)) +
    stat_ellipse(type = case$type, level = case$level, segments = case$segments)
  built <- suppressMessages(layer_data(plot))
  built <- built[!is.na(built$x), ]
  groups <- split(built[, c("x", "y")], built$group)
  results[[case$name]] <- list(
    type = case$type, level = case$level, segments = case$segments,
    groups = lapply(groups, function(points) list(x = points$x, y = points$y))
  )
}
fits <- lapply(split(frame[frame$g != "c", c("x", "y")], frame$g[frame$g != "c"]), function(points) {
  fit <- MASS::cov.trob(points)
  list(center = unname(fit$center), cov = as.vector(fit$cov), iterations = fit$iter)
})
write_json(
  list(data = frame, cases = results, trob = fits),
  "tests/fixtures/ellipse_reference.json",
  digits = NA, auto_unbox = TRUE, pretty = FALSE
)
