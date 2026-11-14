# One gganimate measurement, called by benchmarks/animation.py in a fresh process.
# Usage: Rscript animation.R <scenario> <csv> <out.gif> <repeats>
# Prints one JSON line: baseline RSS after loading, and the time of each render.
suppressPackageStartupMessages({
  library(ggplot2)
  library(gganimate)
  library(gifski)
})

arguments <- commandArgs(trailingOnly = TRUE)
scenario <- arguments[[1]]
frame <- read.csv(arguments[[2]])
out <- arguments[[3]]
repeats <- as.integer(arguments[[4]])
frame$group <- factor(frame$group)

plot <- switch(scenario,
  states = ggplot(frame, aes(x, y, colour = group, group = id)) +
    geom_point(size = 1.5) +
    transition_states(state, transition_length = 1, state_length = 1),
  states_fade = ggplot(frame, aes(x, y, colour = group, group = id)) +
    geom_point(size = 1.5) +
    transition_states(state, transition_length = 1, state_length = 1) +
    enter_fade() + exit_fade(),
  time = ggplot(frame, aes(x, y, colour = group, group = id)) +
    geom_point(size = 1.5) +
    transition_time(time),
  reveal = ggplot(frame, aes(time, y, colour = group)) +
    geom_line() +
    transition_reveal(time),
  stop("unknown scenario ", scenario)
)

rss_kb <- function() as.numeric(system(paste("ps -o rss= -p", Sys.getpid()), intern = TRUE))
baseline <- rss_kb()
times <- numeric(0)
for (i in seq_len(repeats)) {
  start <- Sys.time()
  animation <- animate(plot,
    nframes = 100, fps = 10, width = 480, height = 480, units = "px", res = 72,
    renderer = gifski_renderer(out), progress = FALSE
  )
  times <- c(times, as.numeric(difftime(Sys.time(), start, units = "secs")))
}
cat(sprintf(
  '{"baseline_kb": %.0f, "times": [%s]}\n', baseline,
  paste(sprintf("%.4f", times), collapse = ", ")
))
