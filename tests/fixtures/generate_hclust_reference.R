# Reference clusterings from base R's hclust, for tests/test_dendrogram.py and
# crates/ggrust-core/tests/cluster_parity.rs.
#
# Run from the repository root (needs jsonlite):
#   Rscript tests/fixtures/generate_hclust_reference.R
# Writes tests/fixtures/hclust_reference.json: per case the matrix (rows are the
# items), the method and distance, and R's merge, height and order (1-based).

suppressPackageStartupMessages(library(jsonlite))

set.seed(7)
matrices <- list(
  # Continuous values: no ties.
  normal = round(matrix(rnorm(30 * 5), 30, 5), 6),
  # Few distinct integers: many tied distances, R's tie-breaking decides.
  ties = matrix(sample(0:3, 25 * 4, replace = TRUE), 25, 4),
  # Repeated rows: zero distances.
  repeats = round(matrix(rnorm(6 * 3), 6, 3), 6)[c(1, 2, 1, 3, 4, 2, 5, 6, 6), ],
  # Real data, scaled columns.
  mtcars = round(scale(as.matrix(mtcars[, c("mpg", "disp", "hp", "drat", "wt", "qsec")])), 6),
  two = matrix(c(1, 2, 3, 5, 7, 9), 2, 3, byrow = TRUE)
)
methods <- c("complete", "average", "single", "ward.D2")
distances <- list(
  euclidean = function(x) dist(x),
  correlation = function(x) as.dist(1 - cor(t(x)))
)
cases <- list()
for (name in names(matrices)) {
  x <- matrices[[name]]
  if (name == "ties") x <- x[apply(x, 1, sd) > 0, ] # correlation needs variance
  for (distance in names(distances)) {
    for (method in methods) {
      tree <- hclust(distances[[distance]](x), method = method)
      cases[[length(cases) + 1]] <- list(
        name = name, method = method, distance = distance,
        matrix = unname(x), merge = unname(tree$merge),
        height = I(tree$height), order = I(tree$order)
      )
    }
  }
}
write_json(cases, "tests/fixtures/hclust_reference.json", digits = NA, matrix = "rowmajor", auto_unbox = TRUE)
