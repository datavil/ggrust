# Worker for benchmarks/three_way.py: render one scenario with R ggplot2 and report timings.
#
# Usage: Rscript three_way.R SCENARIO CSV FORMAT REPEATS OUTPUT
# Prints one JSON line: cold and warm (median) seconds, output bytes, baseline RSS in KB.
# The driver wraps this process in /usr/bin/time -l for the peak RSS.
# Rendering goes through the same devices ggsave picks: svglite for SVG, ragg for PNG.

suppressPackageStartupMessages({
  library(ggplot2)
  library(data.table)
  library(svglite)
  library(ragg)
})

arguments <- commandArgs(trailingOnly = TRUE)
scenario <- arguments[[1]]
csv <- arguments[[2]]
format <- arguments[[3]]
repeats <- as.integer(arguments[[4]])
output <- arguments[[5]]

frame <- fread(csv)

build <- function() {
  switch(scenario,
    scatter = ggplot(frame, aes(x, y)) + geom_point(size = 1),
    color_discrete = ggplot(frame, aes(x, y, color = group)) + geom_point(size = 1),
    histogram = ggplot(frame, aes(x)) + geom_histogram(bins = 30),
    bar = ggplot(frame, aes(group)) + geom_bar(),
    boxplot = ggplot(frame, aes(group, y)) + geom_boxplot(),
    violin = ggplot(frame, aes(group, y)) + geom_violin(),
    density = ggplot(frame, aes(x, fill = group)) + geom_density(alpha = 0.3),
    smooth = ggplot(frame, aes(x, y)) + geom_point(size = 1) + geom_smooth(method = "lm", formula = y ~ x),
    line = ggplot(frame, aes(x, y, color = group)) + geom_line(),
    bin2d = ggplot(frame, aes(x, y)) + geom_bin2d(bins = 30),
    facet = ggplot(frame, aes(x, y)) + geom_point(size = 1) + facet_wrap(~group, ncol = 3),
    stop(paste("unknown scenario", scenario))
  )
}

# Same pixel size as ggrust / lets-plot: 600 x 400 px, PNG at scale 2 (1200 x 800).
render <- function() {
  plot <- build()
  if (format == "svg") {
    svglite(output, width = 600 / 72, height = 400 / 72)
  } else if (format == "png") {
    agg_png(output, width = 1200, height = 800, res = 144)
  }
  if (format != "build") {
    print(plot)
    invisible(dev.off())
  }
}

rss_kb <- as.numeric(system(paste("ps -o rss= -p", Sys.getpid()), intern = TRUE))
times <- c()
total <- 0
for (index in seq_len(repeats + 1)) {
  start <- proc.time()[["elapsed"]]
  render()
  elapsed <- proc.time()[["elapsed"]] - start
  times <- c(times, elapsed)
  total <- total + elapsed
  if (total > 30) break
}
warm <- if (length(times) > 1) median(times[-1]) else times[[1]]
bytes <- if (format == "build") 0 else file.size(output)
cat(sprintf(
  '{"cold_seconds": %.6f, "warm_seconds": %.6f, "bytes": %d, "baseline_rss_kb": %.0f, "runs": %d}\n',
  times[[1]], warm, as.integer(bytes), rss_kb, length(times)
))
