# Reference point positions from the R package ggbeeswarm, for tests/test_beeswarm.py.
#
# Run from the repository root (needs ggplot2, ggbeeswarm 0.7.2, jsonlite):
#   Rscript tests/fixtures/generate_ggbeeswarm_reference.R
# Writes tests/fixtures/ggbeeswarm_reference.json: the data, then per case the
# drawn x and y of every row (in data row order). Only deterministic methods.

suppressPackageStartupMessages({
  library(ggplot2)
  library(ggbeeswarm)
  library(jsonlite)
})
pdf(NULL) # ggplot_build may open a device; write no Rplots.pdf

# Rounded so that the JSON decimals give the same doubles in Python.
values <- round(c(
  qnorm(ppoints(30)),
  qgamma(ppoints(45), 2),
  round(qnorm(ppoints(20), 3, 0.5), 1)
), 6)
frame <- data.frame(
  id = seq_along(values),
  g = rep(c("a", "b", "c"), c(30, 45, 20)),
  k = rep(c("u", "v"), length.out = length(values)),
  n = rep(c(1, 2, 4), c(30, 45, 20)),
  v = values
)

cases <- list(
  swarm = list(aes(g, v), geom_beeswarm()),
  swarm_cex = list(aes(g, v), geom_beeswarm(cex = 2.5)),
  swarm_descending = list(aes(g, v), geom_beeswarm(priority = "descending")),
  swarm_none = list(aes(g, v), geom_beeswarm(priority = "none")),
  swarm_density = list(aes(g, v), geom_beeswarm(priority = "density")),
  swarm_side_right = list(aes(g, v), geom_beeswarm(side = 1)),
  swarm_side_left = list(aes(g, v), geom_beeswarm(side = -1)),
  compactswarm = list(aes(g, v), geom_beeswarm(method = "compactswarm")),
  compactswarm_side = list(aes(g, v), geom_beeswarm(method = "compactswarm", side = 1)),
  center = list(aes(g, v), geom_beeswarm(method = "center")),
  hex = list(aes(g, v), geom_beeswarm(method = "hex")),
  square = list(aes(g, v), geom_beeswarm(method = "square")),
  square_side = list(aes(g, v), geom_beeswarm(method = "square", side = -1)),
  hex_side = list(aes(g, v), geom_beeswarm(method = "hex", side = 1)),
  swarm_dodge = list(aes(g, v, color = k), geom_beeswarm(dodge.width = 0.8)),
  swarm_flipped = list(aes(v, g), geom_beeswarm()),
  swarm_continuous_x = list(aes(n, v), geom_beeswarm()),
  quasirandom = list(aes(g, v), geom_quasirandom()),
  quasirandom_bandwidth = list(aes(g, v), geom_quasirandom(bandwidth = 1.5)),
  quasirandom_width = list(aes(g, v), geom_quasirandom(width = 0.2)),
  quasirandom_nbins = list(aes(g, v), geom_quasirandom(nbins = 100)),
  quasirandom_varwidth = list(aes(g, v), geom_quasirandom(varwidth = TRUE)),
  smiley = list(aes(g, v), geom_quasirandom(method = "smiley")),
  frowney = list(aes(g, v), geom_quasirandom(method = "frowney")),
  quasirandom_dodge = list(aes(g, v, color = k), geom_quasirandom(dodge.width = 0.8)),
  quasirandom_flipped = list(aes(v, g), geom_quasirandom()),
  quasirandom_continuous_x = list(aes(n, v), geom_quasirandom())
)

drawn <- lapply(cases, function(case) {
  mapping <- case[[1]]
  mapping$text <- quote(id)
  built <- suppressWarnings(suppressMessages(
    ggplot_build(ggplot(frame, mapping) + case[[2]])
  ))
  layer <- built$data[[1]]
  layer <- layer[order(layer$text), ]
  list(x = as.numeric(layer$x), y = as.numeric(layer$y))
})

# varwidth in ggbeeswarm 0.7.2 is a no-op (setup_params stores `max_length`,
# compute_panel reads `max.length`), so its documented behaviour comes from vipor.
by_group <- split(seq_len(nrow(frame)), frame$g)
documented <- numeric(nrow(frame))
for (rows in by_group) {
  documented[rows] <- vipor::offsetSingleGroup(
    frame$v[rows], maxLength = max(table(frame$g)), adjust = 0.5
  ) * 0.4
}
drawn$quasirandom_varwidth_vipor <- list(
  x = as.numeric(match(frame$g, sort(unique(frame$g)))) + documented,
  y = frame$v
)

write_json(
  list(
    versions = list(
      ggbeeswarm = as.character(packageVersion("ggbeeswarm")),
      vipor = as.character(packageVersion("vipor")),
      beeswarm = as.character(packageVersion("beeswarm"))
    ),
    data = as.list(frame[c("g", "k", "n", "v")]),
    cases = drawn
  ),
  "tests/fixtures/ggbeeswarm_reference.json",
  digits = NA, auto_unbox = TRUE, pretty = FALSE
)
