# Reference results for the ggpubr helpers of plan step 5.7: `compare_means()`
# tables, `geom_pwc()` brackets and `stat_pvalue_manual()` brackets, for
# crates/ggrust-core/src/stat/{compare_table,pwc,pvalue_manual}.rs and
# tests/test_pubr_helpers.py.
#
# Run from the repository root (needs jsonlite, ggplot2, ggpubr and rstatix in
# .libPaths() or R_LIBS):
#   Rscript tests/fixtures/generate_pubr_helpers_reference.R
# Writes tests/fixtures/pubr_helpers_reference.json:
# - data: the data sets (ToothGrowth, `four`: 4 groups, `three`: 3 legend groups per x).
# - compare_means: per case its options and the result table.
# - pwc: per case its options and one row per bracket (xmin, xmax, bar height `y`,
#   left and right tip lengths, label, colour).
# - pvalue_manual: per case its table, options and one row per bracket.

suppressPackageStartupMessages({
  library(jsonlite)
  library(ggplot2)
  library(ggpubr)
  library(rstatix)
})
pdf(NULL)

tooth <- ToothGrowth
tooth$dose <- factor(tooth$dose)
set.seed(3)
four <- data.frame(
  g = rep(c("w", "x", "y", "z"), each = 8),
  v = round(rnorm(32, rep(c(1, 1.5, 2.6, 1.2), each = 8)), 2)
)
three <- data.frame(
  x = rep(rep(c("a", "b"), each = 18)),
  color = rep(rep(c("u", "v", "w"), each = 6), 2),
  v = round(rnorm(36, rep(c(1, 2, 1.5, 3, 1, 1.2), each = 6)), 2)
)

table_rows <- function(result) {
  result <- as.data.frame(result)
  result[] <- lapply(result, function(column) if (is.factor(column)) as.character(column) else column)
  result
}

compare_case <- function(name, data, formula, ...) {
  options <- list(...)
  result <- do.call(compare_means, c(list(formula = as.formula(formula), data = data), options))
  rows <- table_rows(result)
  # ggrust formats each p-value on its own (deviations.md), R the whole column.
  rows$p.format.each <- vapply(rows$p, format.pval, "", digits = 2)
  list(name = name, formula = formula, options = options, rows = rows)
}

compare_means_cases <- suppressWarnings(list(
  compare_case("dose", tooth, "len ~ dose"),
  compare_case("dose_t", tooth, "len ~ dose", method = "t.test"),
  compare_case("supp_paired", tooth, "len ~ supp", method = "t.test", paired = TRUE),
  compare_case("dose_reference", tooth, "len ~ dose", ref.group = "0.5"),
  compare_case("dose_reference_all", tooth, "len ~ dose", ref.group = ".all.", method = "t.test"),
  compare_case("dose_anova", tooth, "len ~ dose", method = "anova"),
  compare_case("dose_kruskal_by_supp", tooth, "len ~ dose", method = "kruskal.test", group.by = "supp"),
  compare_case("supp_by_dose", tooth, "len ~ supp", group.by = "dose", p.adjust.method = "BH"),
  compare_case("four", four, "v ~ g"),
  compare_case("four_bonferroni", four, "v ~ g", p.adjust.method = "bonferroni")
))

# ggplot_build rows of a bracket layer, one per bracket: the middle segment is the
# bar; the first and last give the tips. geom_pwc stacks the left tips, then the
# bars, then the right tips; stat_pvalue_manual (`interleaved`) gives each bracket
# its three rows in turn.
brackets <- function(plot, layer = 2, interleaved = FALSE) {
  data <- ggplot_build(plot)$data[[layer]]
  n <- nrow(data) / 3
  if (n == 0) return(list())
  part <- function(k) if (interleaved) 3 * (seq_len(n) - 1) + k else (k - 1) * n + seq_len(n)
  left <- data[part(1), ]
  bar <- data[part(2), ]
  right <- data[part(3), ]
  label <- if ("label" %in% names(data)) bar$label else bar$annotation
  data.frame(
    xmin = bar$x, xmax = bar$xend, y = bar$y,
    tip_left = left$yend - left$y, tip_right = right$y - right$yend,
    label = as.character(label), colour = bar$colour
  )
}

pwc_case <- function(name, data, mapping, ...) {
  options <- list(...)
  plot <- ggplot(data, mapping) + geom_boxplot() + do.call(geom_pwc, options)
  y <- ggplot_build(plot)$layout$panel_scales_y[[1]]$range$range
  list(name = name, options = options, y_range = y, rows = brackets(plot))
}

pwc_cases <- suppressWarnings(list(
  pwc_case("default", tooth, aes(dose, len)),
  pwc_case("t_test", tooth, aes(dose, len), method = "t_test", label = "p.adj.signif"),
  pwc_case("reference", tooth, aes(dose, len), ref.group = "0.5", p.adjust.method = "bonferroni",
           label = "{p.adj.format}{p.adj.signif}"),
  pwc_case("reference_all", tooth, aes(dose, len), ref.group = "all", label = "p.adj"),
  pwc_case("x_var", tooth, aes(dose, len, color = supp)),
  # ggpubr needs group = supp for legend.var (ggrust takes the color groups).
  pwc_case("legend_var", tooth, aes(dose, len, color = supp, group = supp), group.by = "legend.var"),
  pwc_case("legend_var_hide_ns", tooth, aes(dose, len, color = supp, group = supp),
           group.by = "legend.var", hide.ns = TRUE, label = "p.signif"),
  pwc_case("legend_var_panel", tooth, aes(dose, len, color = supp, group = supp),
           group.by = "legend.var", p.adjust.by = "panel", label = "p.adj.format"),
  pwc_case("positioned", tooth, aes(dose, len), y.position = 40, step.increase = 0.05,
           tip.length = 0.02, bracket.nudge.y = 0.1, label = "p = {p.format}, {method}"),
  pwc_case("four", four, aes(g, v), label = "{p} {p.adj} n = {n1}"),
  pwc_case("three_x_var", three, aes(x, v, color = color), label = "p.adj.format")
))

dose_table <- as.data.frame(compare_means(len ~ dose, tooth))
dose_table$group1 <- as.character(dose_table$group1)
dose_table$group2 <- as.character(dose_table$group2)
dose_table$cat <- c("u", "u", "v")
dose_table$height <- c(35, 37, 39)

manual_case <- function(name, table, ...) {
  options <- list(...)
  plot <- ggplot(tooth, aes(dose, len)) + geom_boxplot() +
    do.call(stat_pvalue_manual, c(list(data = table), options))
  list(name = name, table = table, options = options, rows = brackets(plot, interleaved = TRUE))
}

pvalue_manual_cases <- suppressWarnings(list(
  manual_case("stepped", dose_table, label = "p.signif", y.position = 35, step.increase = 0.1),
  manual_case("columns", dose_table, label = "p = {p.adj}", y.position = "height",
              tip.length = 0.01, bracket.nudge.y = 1),
  manual_case("step_groups", dose_table, label = "p.format", y.position = 35,
              step.increase = 0.1, step.group.by = "cat"),
  manual_case("hide_ns", transform(dose_table, p.adj = c(0.01, 0.2, 0.03)), y.position = 35,
              step.increase = 0.1, hide.ns = TRUE)
))

write_json(
  list(
    data = list(
      tooth = list(len = tooth$len, supp = as.character(tooth$supp), dose = as.character(tooth$dose)),
      four = four,
      three = three
    ),
    compare_means = compare_means_cases,
    pwc = pwc_cases,
    pvalue_manual = pvalue_manual_cases
  ),
  "tests/fixtures/pubr_helpers_reference.json",
  digits = NA, auto_unbox = TRUE, pretty = TRUE, dataframe = "rows", na = "null"
)
