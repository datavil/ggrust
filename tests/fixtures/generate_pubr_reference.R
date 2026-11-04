# Reference numbers for ggpubr / ggpmisc style statistics, for
# crates/ggrust-core/src/stat/hypothesis.rs (p-values, coefficients) and
# tests/test_pubr_stats.py (labels and their places).
#
# Run from the repository root (needs jsonlite; the `layers` part also needs
# ggplot2, ggpubr 1.0 and ggpmisc 1.0, in .libPaths() or R_LIBS):
#   Rscript tests/fixtures/generate_pubr_reference.R
# Writes tests/fixtures/pubr_reference.json:
# - tests: per case its groups, method, paired flag and R's p-value.
# - correlations: per case x, y, method and R's estimate and p-value.
# - polynomials: per case x, y, degree and lm()'s coefficients, R^2, adjusted R^2.
# - adjust: p-values and p.adjust() for every method.
# - format_pval: p and format.pval(p, digits = 2).
# - layers: ggplot_build data of stat_compare_means, stat_cor and stat_poly_eq.

suppressPackageStartupMessages(library(jsonlite))
pdf(NULL)

set.seed(11)
# Rounded so that the JSON decimals give the same doubles in Rust and Python;
# rounding to 1 decimal makes ties.
r2 <- function(x) round(x, 2)
r1 <- function(x) round(x, 1)
samples <- list(
  tiny_a = c(1.5, 2.25, 3.0),
  tiny_b = c(4.0, 5.5),
  small_a = r2(rnorm(8, 10, 2)),
  small_b = r2(rnorm(9, 12, 2)),
  small_c = r2(rnorm(7, 11, 3)),
  tied_a = r1(rnorm(15, 5, 1)),
  tied_b = r1(rnorm(12, 5.6, 1)),
  tied_c = r1(rnorm(20, 4.8, 1)),
  big_a = r2(rnorm(60, 0, 1)),
  big_b = r2(rnorm(70, 0.4, 1.2)),
  far_a = r2(rnorm(40, 0, 1)),
  far_b = r2(rnorm(40, 3, 1))
)

case <- function(names, method, paired = FALSE) {
  groups <- unname(samples[names])
  p <- switch(method,
    t.test = t.test(groups[[1]], groups[[2]], paired = paired)$p.value,
    wilcox.test = suppressWarnings(
      wilcox.test(groups[[1]], groups[[2]], paired = paired)$p.value
    ),
    anova = {
      values <- unlist(groups)
      g <- factor(rep(seq_along(groups), lengths(groups)))
      anova(lm(values ~ g))$`Pr(>F)`[1]
    },
    kruskal.test = kruskal.test(groups)$p.value
  )
  list(name = paste(c(method, names, if (paired) "paired"), collapse = " "),
       groups = groups, method = method, paired = paired, p = p)
}

paired_tied <- list(c(r1(samples$tied_a[1:12])), samples$tied_b)
samples$paired_x <- samples$tied_a[1:12]
samples$paired_zero <- samples$tied_a[1:12]
samples$paired_zero[c(2, 5)] <- samples$tied_b[c(2, 5)]
samples$paired_small_x <- samples$small_a[1:7]
samples$paired_far_x <- samples$far_a[1:20]
samples$paired_far_y <- samples$far_b[1:20]
samples$big_paired <- samples$big_a[1:60]
samples$big_paired_y <- samples$big_b[1:60]

tests <- list(
  case(c("tiny_a", "tiny_b"), "t.test"),
  case(c("small_a", "small_b"), "t.test"),
  case(c("far_a", "far_b"), "t.test"),
  case(c("big_a", "big_b"), "t.test"),
  case(c("small_a", "small_c"), "t.test"),
  case(c("paired_small_x", "small_c"), "t.test", TRUE),
  case(c("paired_x", "tied_b"), "t.test", TRUE),
  case(c("tiny_a", "tiny_b"), "wilcox.test"),
  case(c("small_a", "small_b"), "wilcox.test"),
  case(c("small_a", "small_c"), "wilcox.test"),
  case(c("far_a", "far_b"), "wilcox.test"),
  case(c("tied_a", "tied_b"), "wilcox.test"),
  case(c("big_a", "big_b"), "wilcox.test"),
  case(c("big_a", "small_b"), "wilcox.test"),
  case(c("paired_small_x", "small_c"), "wilcox.test", TRUE),
  case(c("paired_far_x", "paired_far_y"), "wilcox.test", TRUE),
  case(c("paired_x", "tied_b"), "wilcox.test", TRUE),
  case(c("paired_zero", "tied_b"), "wilcox.test", TRUE),
  case(c("big_paired", "big_paired_y"), "wilcox.test", TRUE),
  case(c("small_a", "small_b"), "anova"),
  case(c("small_a", "small_b", "small_c"), "anova"),
  case(c("tied_a", "tied_b", "tied_c"), "anova"),
  case(c("far_a", "far_b", "big_a"), "anova"),
  case(c("tiny_a", "tiny_b"), "kruskal.test"),
  case(c("small_a", "small_b", "small_c"), "kruskal.test"),
  case(c("tied_a", "tied_b", "tied_c"), "kruskal.test"),
  case(c("far_a", "far_b", "big_a", "big_b"), "kruskal.test")
)

correlation <- function(name, x, y, method) {
  test <- suppressWarnings(cor.test(x, y, method = method))
  list(name = paste(method, name), x = x, y = y, method = method,
       estimate = unname(test$estimate), p = test$p.value)
}
xs <- list(
  n5 = r2(rnorm(5)), n9 = r2(rnorm(9)), n10 = r2(rnorm(10)), n30 = r2(rnorm(30)),
  n60 = r2(rnorm(60)), tied = r1(rnorm(25)), n1300 = round(rnorm(1300), 4)
)
ys <- lapply(xs, function(x) round(0.5 * x + rnorm(length(x)), if (length(x) == 25) 1 else 2))
ys$n1300 <- round(0.05 * xs$n1300 + rnorm(1300), 4)
strong_x <- r2(rnorm(40))
strong_y <- r2(-2 * strong_x + rnorm(40, 0, 0.3))
correlations <- list()
for (method in c("pearson", "spearman", "kendall")) {
  for (name in names(xs)) {
    if (method == "kendall" && name == "n1300") next
    correlations[[length(correlations) + 1]] <- correlation(name, xs[[name]], ys[[name]], method)
  }
  correlations[[length(correlations) + 1]] <- correlation("strong", strong_x, strong_y, method)
}
# Spearman exact boundary: n = 9 enumerates, n = 10 uses the Edgeworth series.
correlations[[length(correlations) + 1]] <-
  correlation("perm9", c(1, 2, 3, 4, 5, 6, 7, 8, 9), c(2, 1, 4, 3, 6, 5, 9, 7, 8), "spearman")

poly_x <- r2(runif(30, 0, 10))
poly_y <- r2(3 - 1.2 * poly_x + 0.15 * poly_x^2 + rnorm(30))
polynomials <- lapply(1:3, function(degree) {
  fit <- lm(poly_y ~ poly(poly_x, degree, raw = TRUE))
  s <- summary(fit)
  list(degree = degree, x = poly_x, y = poly_y, coefficients = unname(coef(fit)),
       r_squared = s$r.squared, adjusted_r_squared = s$adj.r.squared,
       p = unname(pf(s$fstatistic[1], s$fstatistic[2], s$fstatistic[3], lower.tail = FALSE)))
})

adjust_p <- c(0.012, 0.3, 0.0004, 0.04, 0.041, 0.9, 0.012)
adjust <- list(p = adjust_p, methods = lapply(
  setNames(nm = c("holm", "hochberg", "hommel", "bonferroni", "BH", "BY", "none")),
  function(method) p.adjust(adjust_p, method)
))

format_p <- c(1, 0.5, 0.0645, 0.05, 0.0099, 0.00179124, 0.000999, 1.5e-09, 7.02e-06,
  9.96e-05, 1e-100, 3e-17, 0.995, 0.10, 0.123456)
format_pval <- list(p = format_p, text = vapply(format_p, format.pval, "", digits = 2))

# Regressions (ex1.7 bug hunt): mid-range p-values with many degrees of freedom
# (t and F tails near the beta mean), and centred normal statistics (R's sign(0)).
for (n in c(100, 200, 500)) {
  x <- r2(runif(n))
  y <- r2(x + rnorm(n, 0, if (n == 500) 1 else 0.3))
  correlations[[length(correlations) + 1]] <- correlation(paste0("mid", n), x, y, "pearson")
}
samples$wide_a <- r2(rnorm(300, 0, 1))
samples$wide_b <- r2(rnorm(300, 0.6, 1))
samples$same_a <- c(1, 2, 2, 3)
samples$same_b <- c(1, 2, 2, 3)
tests <- c(tests, list(
  case(c("wide_a", "wide_b"), "t.test"),
  case(c("wide_a", "wide_b", "big_a"), "anova"),
  case(c("same_a", "same_b"), "wilcox.test")
))

layers <- NULL
if (requireNamespace("ggpubr", quietly = TRUE) && requireNamespace("ggpmisc", quietly = TRUE)) {
  suppressPackageStartupMessages({
    library(ggplot2)
    library(ggpubr)
    library(ggpmisc)
  })
  tooth <- ToothGrowth
  tooth$dose <- factor(tooth$dose)
  cars <- mtcars
  cars$cyl <- factor(cars$cyl)
  keep <- c("x", "xend", "y", "yend", "label", "annotation", "hjust", "vjust", "group")
  built <- function(plot, layer = 2) {
    data <- ggplot_build(plot)$data[[layer]]
    data <- data[, intersect(names(data), keep)]
    data[] <- lapply(data, function(column) if (is.factor(column)) as.character(column) else column)
    data
  }
  boxes <- ggplot(tooth, aes(dose, len)) + geom_boxplot()
  comparisons <- list(c("0.5", "1"), c("1", "2"), c("0.5", "2"))
  layers <- suppressMessages(suppressWarnings(list(
    tooth = list(len = tooth$len, supp = as.character(tooth$supp), dose = as.character(tooth$dose)),
    cars = list(wt = cars$wt, mpg = cars$mpg, cyl = as.character(cars$cyl)),
    global = built(boxes + stat_compare_means()),
    anova = built(boxes + stat_compare_means(method = "anova")),
    comparisons = built(boxes + stat_compare_means(comparisons = comparisons)),
    signif = built(boxes + stat_compare_means(comparisons = comparisons, label = "p.signif")),
    t_test = built(boxes + stat_compare_means(comparisons = comparisons, method = "t.test")),
    reference = built(boxes + stat_compare_means(ref.group = "0.5", label = "p.signif")),
    reference_all = built(boxes + stat_compare_means(ref.group = ".all.", label = "p.signif")),
    grouped = built(ggplot(tooth, aes(dose, len, color = supp)) + geom_boxplot() +
      stat_compare_means(label = "p.signif")),
    paired = built(ggplot(tooth, aes(supp, len)) + geom_boxplot() +
      stat_compare_means(method = "t.test", paired = TRUE)),
    cor = built(ggplot(cars, aes(wt, mpg)) + geom_point() + stat_cor(output.type = "text")),
    cor_groups = built(ggplot(cars, aes(wt, mpg, color = cyl)) + geom_point() +
      stat_cor(output.type = "text")),
    poly_eq = built(ggplot(cars, aes(wt, mpg)) + geom_point() +
      stat_poly_eq(use_label("eq", "R2"), formula = y ~ poly(x, 2, raw = TRUE)))
  )))
}

reference <- list(
  generated_with = R.version.string,
  tests = tests, correlations = correlations, polynomials = polynomials,
  adjust = adjust, format_pval = format_pval, layers = layers
)
write_json(reference, "tests/fixtures/pubr_reference.json", auto_unbox = TRUE,
  digits = NA, pretty = FALSE, null = "null", na = "null")
