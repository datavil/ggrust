# Reference secondary axis breaks from ggplot2's sec_axis (ggplot2 3.5.2), for
# tests/test_sec_axis.py.
#
# Run from the repository root (needs ggplot2, jsonlite):
#   Rscript tests/fixtures/generate_sec_axis_reference.R
# Writes tests/fixtures/sec_axis_reference.json: per case the secondary break
# positions on the primary scale (transformed units, `get_breaks()` of the
# secondary view scale) and their labels.

suppressPackageStartupMessages({
  library(ggplot2)
  library(jsonlite)
})

celsius <- data.frame(
  month = 1:12,
  celsius = c(3.1, 4.2, 8.0, 12.5, 16.9, 20.3, 22.6, 22.1, 18.4, 13.2, 7.6, 4.1)
)
area <- data.frame(
  area = c(103, 385, 506, 378, 3287, 8516, 0.316),
  people = c(0.38, 5.5, 48, 124, 1430, 216, 0.53)
)

secondary <- function(plot, axis) {
  params <- ggplot_build(plot)$layout$panel_params[[1]]
  view <- params[[paste0(axis, ".sec")]]
  breaks <- view$get_breaks()
  kept <- !is.na(breaks)
  list(positions = breaks[kept], labels = view$get_labels()[kept])
}

cases <- list(
  linear_given = secondary(
    ggplot(celsius, aes(month, celsius)) + geom_line() +
      scale_y_continuous(sec.axis = sec_axis(~ . * 1.8 + 32, breaks = c(40, 50, 60, 70))),
    "y"
  ),
  linear_auto = secondary(
    ggplot(celsius, aes(month, celsius)) + geom_line() +
      scale_y_continuous(sec.axis = sec_axis(~ . * 1.8 + 32)),
    "y"
  ),
  log_given = secondary(
    ggplot(area, aes(area, people)) + geom_point() +
      scale_x_log10(sec.axis = sec_axis(~ . * 1000 / 2.59, breaks = c(1e3, 1e4, 1e5, 1e6))),
    "x"
  ),
  reverse_given = secondary(
    ggplot(celsius, aes(month, celsius)) + geom_line() +
      scale_y_reverse(sec.axis = sec_axis(~ . * 2, breaks = c(10, 20, 30, 40))),
    "y"
  ),
  sqrt_given = secondary(
    ggplot(celsius, aes(celsius, month)) + geom_point() +
      scale_x_continuous(sec.axis = sec_axis(~ sqrt(.), breaks = c(2, 3, 4))),
    "x"
  )
)
write_json(cases, "tests/fixtures/sec_axis_reference.json", digits = NA, pretty = TRUE)
