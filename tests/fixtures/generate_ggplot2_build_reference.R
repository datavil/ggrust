# Reference stat results and scales from ggplot2's ggplot_build(), for
# tests/test_ggplot2_build.py (the ggplot2 validation suite).
#
# Run from the repository root (needs ggplot2, jsonlite, quantreg):
#   Rscript tests/fixtures/generate_ggplot2_build_reference.R
# Writes tests/fixtures/ggplot2_build_reference.json. Every case reads the same
# CSV datasets ggrust ships (python/ggrust/data), so both sides see the same rows.
# Per case: the R version, every layer's numeric columns (rows in ggplot_build()
# order) and the panel's x and y scales: the expanded range, the breaks inside it
# and, for a discrete axis, its levels.
#
# Each case here has a twin of the same name in tests/test_ggplot2_build.py.
# Add a case on both sides, then rerun this script.

suppressPackageStartupMessages({
  library(ggplot2)
  library(jsonlite)
})

data_dir <- file.path("python", "ggrust", "data")
# Empty cells are missing, as polars reads them.
read_data <- function(name) {
  read.csv(file.path(data_dir, name), stringsAsFactors = FALSE, na.strings = c("", "NA"))
}
mpg <- read_data("mpg.csv")
penguins <- read_data("penguins.csv")
iris <- read_data("iris.csv")
penguins <- penguins[stats::complete.cases(penguins), ]

skipped <- c("PANEL", "flipped_aes", "colour", "fill", "linetype", "shape", "alpha",
             "linewidth", "size", "stroke", "weight", "width", "outliers", "relvarwidth",
             "label", "family", "fontface", "lineheight", "angle", "hjust", "vjust")

layer_columns <- function(frame) {
  kept <- names(frame)[!names(frame) %in% skipped]
  columns <- list()
  for (name in kept) {
    column <- frame[[name]]
    if (is.numeric(column)) columns[[name]] <- as.numeric(column)
  }
  if ("outliers" %in% names(frame)) {
    columns$outliers <- lapply(frame$outliers, function(values) as.numeric(values))
  }
  columns
}

axis_scale <- function(view) {
  discrete <- view$is_discrete()
  range <- view$continuous_range
  if (discrete) {
    return(list(discrete = TRUE, range = as.numeric(range), breaks = list(),
                labels = list(), levels = as.character(view$get_limits())))
  }
  breaks <- view$get_breaks()
  kept <- !is.na(breaks)
  list(
    discrete = FALSE,
    range = as.numeric(range),
    breaks = as.numeric(breaks[kept]),
    labels = as.character(view$get_labels()[kept]),
    levels = list()
  )
}

reference <- function(plot) {
  built <- ggplot_build(plot)
  params <- built$layout$panel_params[[1]]
  list(
    layers = lapply(built$data, layer_columns),
    x = axis_scale(params$x),
    y = axis_scale(params$y)
  )
}

cases <- list(
  histogram_bins = ggplot(mpg, aes(hwy)) + geom_histogram(bins = 10),
  histogram_binwidth_boundary = ggplot(mpg, aes(hwy)) +
    geom_histogram(binwidth = 2, boundary = 0),
  histogram_binwidth_center = ggplot(penguins, aes(body_mass_g)) +
    geom_histogram(binwidth = 250, center = 0),
  histogram_stacked = ggplot(mpg, aes(hwy, fill = drv)) +
    geom_histogram(binwidth = 3, boundary = 0),
  freqpoly = ggplot(mpg, aes(cty)) + geom_freqpoly(binwidth = 1),
  bar_count = ggplot(mpg, aes(class)) + geom_bar(),
  bar_stacked = ggplot(mpg, aes(class, fill = drv)) + geom_bar(),
  bar_dodged = ggplot(mpg, aes(class, fill = drv)) + geom_bar(position = "dodge"),
  bar_filled = ggplot(mpg, aes(class, fill = drv)) + geom_bar(position = "fill"),
  col_values = ggplot(data.frame(item = c("a", "b", "c"), value = c(3, -1.5, 7)),
                      aes(item, value)) + geom_col(),
  density_default = ggplot(mpg, aes(hwy)) + geom_density(),
  density_grouped = ggplot(mpg, aes(hwy, colour = drv)) + geom_density(),
  density_adjust = ggplot(penguins, aes(flipper_length_mm)) +
    geom_density(adjust = 0.5, kernel = "epanechnikov"),
  smooth_lm = ggplot(mpg, aes(displ, hwy)) + geom_smooth(method = "lm"),
  smooth_lm_grouped = ggplot(mpg, aes(displ, hwy, colour = drv)) +
    geom_smooth(method = "lm", level = 0.9),
  smooth_loess = ggplot(mpg, aes(displ, hwy)) + geom_smooth(method = "loess"),
  smooth_glm_poly = ggplot(iris, aes(sepal_length, petal_length)) +
    geom_smooth(method = "lm", formula = y ~ poly(x, 2)),
  boxplot = ggplot(mpg, aes(class, hwy)) + geom_boxplot(),
  boxplot_coef = ggplot(penguins, aes(species, body_mass_g)) + geom_boxplot(coef = 1),
  violin = ggplot(iris, aes(species, sepal_width)) + geom_violin(),
  ecdf = ggplot(mpg, aes(hwy)) + stat_ecdf(),
  summary_mean_se = ggplot(mpg, aes(class, hwy)) + stat_summary(fun.data = mean_se),
  summary_median_range = ggplot(mpg, aes(class, hwy)) +
    stat_summary(fun = median, fun.min = min, fun.max = max),
  count_points = ggplot(mpg, aes(cty, hwy)) + geom_count(),
  qq = ggplot(mpg, aes(sample = hwy)) + stat_qq(),
  qq_line = ggplot(mpg, aes(sample = hwy)) + stat_qq_line(),
  quantile = ggplot(mpg, aes(displ, hwy)) + geom_quantile(),
  bin2d = ggplot(mpg, aes(displ, hwy)) + geom_bin_2d(bins = 8),
  point_scales = ggplot(penguins, aes(bill_length_mm, body_mass_g)) + geom_point(),
  point_log10 = ggplot(penguins, aes(bill_length_mm, body_mass_g)) + geom_point() +
    scale_y_log10(),
  point_sqrt = ggplot(mpg, aes(displ, hwy)) + geom_point() + scale_x_sqrt(),
  point_reverse = ggplot(mpg, aes(displ, hwy)) + geom_point() + scale_x_reverse(),
  point_limits = ggplot(mpg, aes(displ, hwy)) + geom_point() +
    scale_x_continuous(limits = c(2, 6)),
  point_expand = ggplot(mpg, aes(displ, hwy)) + geom_point() +
    scale_y_continuous(expand = expansion(mult = 0.1, add = 1)),
  point_negative = ggplot(data.frame(x = c(-0.031, 0.0043, 0.017), y = c(-12500, 3, 98000)),
                          aes(x, y)) + geom_point()
)

output <- list(
  ggplot2 = as.character(packageVersion("ggplot2")),
  r = paste(R.version$major, R.version$minor, sep = "."),
  cases = lapply(cases, reference)
)
path <- file.path("tests", "fixtures", "ggplot2_build_reference.json")
write(toJSON(output, auto_unbox = TRUE, digits = I(10), null = "null", na = "null",
             pretty = FALSE), path)
cat("wrote", path, "with", length(cases), "cases\n")
