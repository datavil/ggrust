# Reference centroids from the R package ggpp, for tests/test_centroid.py.
#
# Run from the repository root (needs ggplot2, ggpp 0.6.1, jsonlite):
#   Rscript tests/fixtures/generate_ggpp_centroid_reference.R
# Writes tests/fixtures/ggpp_centroid_reference.json: the data, then per case the
# x and y of every output row, ordered by panel and cluster.

suppressPackageStartupMessages({
  library(ggplot2)
  library(ggpp)
  library(jsonlite)
})
pdf(NULL) # ggplot_build may open a device; write no Rplots.pdf

# Three clusters in two samples, like a small UMAP, with a few outlying cells.
set.seed(11)
cluster <- rep(c("B cells", "T cells", "NK"), c(120, 90, 60))
centre_x <- c("B cells" = 0, "T cells" = 5, "NK" = 2)[cluster]
centre_y <- c("B cells" = 0, "T cells" = 1, "NK" = 5)[cluster]
frame <- data.frame(
  x = round(centre_x + rnorm(270) + c(rep(0, 260), rep(6, 10)), 6),
  y = round(centre_y + rnorm(270), 6),
  cluster = cluster,
  sample = rep(c("s1", "s2"), 135)
)

rows <- function(layer) {
  # ggplot2 does not group by label (lets-plot and ggrust do): group explicitly.
  data <- layer_data(
    ggplot(frame, aes(x, y, label = cluster, group = cluster)) + layer + facet_wrap(~sample)
  )
  data <- data[order(data$PANEL, data$label), ]
  list(panel = as.integer(data$PANEL) - 1L, label = data$label, x = data$x, y = data$y)
}

out <- list(
  x = frame$x,
  y = frame$y,
  cluster = frame$cluster,
  sample = frame$sample,
  cases = list(
    list(stat = "centroid", options = list(fun = "mean"),
         rows = rows(stat_centroid(geom = "text", .fun = mean))),
    list(stat = "centroid", options = list(fun = "median"),
         rows = rows(stat_centroid(geom = "text", .fun = median))),
    list(stat = "summary_xy", options = list(fun_x = "median", fun_y = "mean"),
         rows = rows(stat_summary_xy(geom = "text", .fun.x = median, .fun.y = mean)))
  )
)
write_json(out, "tests/fixtures/ggpp_centroid_reference.json", auto_unbox = TRUE, digits = NA)
