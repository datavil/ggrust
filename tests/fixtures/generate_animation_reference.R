# Reference values for ggrust's animations (transition_states and tweening),
# for crates/ggrust-core/src/animate/states.rs and tween.rs.
#
# Run from the repository root (needs farver and jsonlite):
#   Rscript tests/fixtures/generate_animation_reference.R
# Writes tests/fixtures/animation_reference.json:
# - "distribute": gganimate's frame split between state holds and transitions,
#   from `distribute_frames` below, copied from gganimate 1.0.x
#   (R/transition-.R, MIT, Thomas Lin Pedersen) because gganimate itself is not
#   installed here.
# - "lab": colours moved part of the way in CIE Lab with alpha, as tweenr's
#   interpolate_colour_state does through farver.
# - "time": transition_time's frame of every time and the time of every frame,
#   from `time_frames` below (gganimate R/transition-time.R `get_times`, copied).
# - "reveal": transition_reveal's frame of every `along` value
#   (`along_frames`, gganimate R/transition_reveal.R `get_row_along`) and the
#   revealed rows per frame from `tween_along_numeric`, a translation to R of
#   tweenr's `numeric_along_interpolator` (src/along.cpp, MIT) followed by
#   gganimate's removal of a raw row repeated by the moving row.

suppressPackageStartupMessages({
  library(farver)
  library(jsonlite)
})

distribute_frames <- function(statics, transitions, frames) {
  total <- sum(c(statics, transitions))
  static_frames <- ceiling(statics * frames / total)
  transition_frames <- ceiling(transitions * frames / total)
  all <- c(static_frames, transition_frames)
  n <- rep(seq_along(all), all)
  ind <- unlist(lapply(all, seq_len))
  n <- table(n[order(ind)[seq_len(frames)]])
  ind <- as.integer(names(n))
  static_numbers <- ind <= length(statics)
  static_frames[ind[static_numbers]] <- n[static_numbers]
  transition_frames[ind[!static_numbers] - length(statics)] <- n[!static_numbers]
  zero_trans <- which(transition_frames == 0 & static_frames[which(transition_frames == 0) + 1] != 0)
  transition_frames[zero_trans] <- 1
  static_frames[zero_trans + 1] <- static_frames[zero_trans + 1] - 1
  list(static_length = static_frames, transition_length = transition_frames, mod = frames / total)
}

cases <- list(
  list(statics = c(1, 1, 1), transitions = c(1, 1, 1), frames = 101),
  list(statics = c(1, 1, 1), transitions = c(1, 1, 0), frames = 100),
  list(statics = c(2, 2), transitions = c(1, 0), frames = 10),
  list(statics = c(1, 1, 1), transitions = c(0, 0, 0), frames = 9),
  list(statics = c(1, 3, 2, 1), transitions = c(2, 1, 1, 0.5), frames = 51),
  list(statics = rep(1, 8), transitions = rep(1, 8), frames = 6),
  list(statics = c(0, 0, 0), transitions = c(1, 1, 1), frames = 31),
  list(statics = c(1, 1, 1, 1), transitions = c(0, 2, 0, 0), frames = 20),
  list(statics = c(1, 2, 1), transitions = c(3, 1, 2), frames = 7)
)
distribute <- lapply(cases, function(case) {
  result <- distribute_frames(case$statics, case$transitions, case$frames)
  list(
    statics = case$statics,
    transitions = case$transitions,
    frames = case$frames,
    holds = as.integer(result$static_length),
    moves = as.integer(result$transition_length)
  )
})

mixes <- list(
  list(from = "#FF0000", to = "#0000FF", share = 0.5),
  list(from = "#1B9E77", to = "#D95F02", share = 0.3),
  list(from = "#FFFFFF", to = "#000000", share = 0.25),
  list(from = "#F8766D", to = "#00BFC4", share = 0.8),
  list(from = "#FF000000", to = "#0000FFFF", share = 0.5)
)
lab <- lapply(mixes, function(mix) {
  ends <- decode_colour(c(mix$from, mix$to), alpha = TRUE, to = "lab")
  mixed <- ends[1, ] + mix$share * (ends[2, ] - ends[1, ])
  colour <- encode_colour(matrix(mixed[1:3], 1), alpha = mixed[4], from = "lab")
  list(from = mix$from, to = mix$to, share = mix$share, colour = colour)
})

# gganimate get_times (transition_time), for one layer of numeric times.
time_frames <- function(v, nframes, range = NULL) {
  if (is.null(range)) range <- range(v, na.rm = TRUE)
  v_u <- unique(v)
  v_v <- round(1 + (nframes - 1) * (v_u - range[1]) / diff(range))
  v_v[duplicated(v_v)] <- NA
  list(
    positions = v_v[match(v, v_u)],
    frame_time = seq(range[1], range[2], length.out = nframes)
  )
}
time_cases <- list(
  list(values = c(1952, 1957, 1962, 1967, 1972, 2007), nframes = 100),
  list(values = c(0, 0.5, 1.5, 4), nframes = 5),
  list(values = c(1, 2, 3, 10), nframes = 4),
  list(values = c(3, 1, 2, 2.1), nframes = 11, range = c(0, 5))
)
time <- lapply(time_cases, function(case) {
  result <- time_frames(case$values, case$nframes, case$range)
  list(
    values = case$values,
    nframes = case$nframes,
    range = if (is.null(case$range)) NULL else case$range,
    positions = result$positions,
    frame_time = result$frame_time
  )
})

# gganimate get_row_along (transition_reveal): the frame of each value.
along_frames <- function(v, nframes, range = NULL) {
  if (is.null(range)) range <- range(v)
  round((nframes - 1) * (v - range[1]) / diff(range)) + 1
}

# tweenr numeric_along_interpolator (C++), as R; rows sorted by group.
tween_along_numeric <- function(data, group, time, frames, history, keep_last) {
  out <- list(data = numeric(), group = integer(), frame = numeric(), raw = logical())
  push <- function(value, g, f, raw) {
    out$data <<- c(out$data, value)
    out$group <<- c(out$group, g)
    out$frame <<- c(out$frame, f)
    out$raw <<- c(out$raw, raw)
  }
  n <- length(data)
  for (frame_time in frames) {
    for (j in seq_len(n)) {
      last <- j == n
      jj <- if (last) j else j + 1
      before <- time[j] <= frame_time
      after <- time[jj] > frame_time
      same <- group[j] == group[jj]
      if ((history && same && before) || ((!same || last) && keep_last && before)) {
        push(data[j], group[j], frame_time, TRUE)
      }
      if (same && before == after) {
        pos <- (frame_time - time[j]) / (time[jj] - time[j])
        push(data[j] + (data[jj] - data[j]) * pos, group[j], frame_time, FALSE)
      }
    }
  }
  # gganimate: drop a raw row the moving row right after it repeats.
  k <- seq_along(out$data)
  repeated <- k[-1][!out$raw[-1] & out$raw[-length(k)] &
    out$group[-1] == out$group[-length(k)] & out$data[-1] == out$data[-length(k)]] - 1
  keep <- setdiff(k, repeated)
  lapply(out, function(column) column[keep])
}
reveal_cases <- list(
  list(
    x = c(10, 40, 70, 30, 50), group = c(1L, 1L, 1L, 0L, 0L),
    along = c(1, 4, 7, 3, 5), nframes = 7
  ),
  list(
    x = c(0, 2, 5, 1, 3, 9), group = c(0L, 0L, 0L, 1L, 1L, 1L),
    along = c(1, 2.2, 6, 0, 3.7, 8), nframes = 9
  )
)
reveal <- lapply(reveal_cases, function(case) {
  position <- along_frames(case$along, case$nframes)
  order <- order(case$group, position)
  x <- case$x[order]
  group <- case$group[order]
  position <- position[order]
  frames <- seq_len(case$nframes)
  run <- function(history, keep_last) {
    result <- tween_along_numeric(x, group, position, frames, history, keep_last)
    result$raw <- NULL
    result
  }
  list(
    x = case$x,
    group = case$group,
    along = case$along,
    nframes = case$nframes,
    positions = along_frames(case$along, case$nframes),
    path = run(TRUE, TRUE),
    point = run(FALSE, TRUE),
    point_no_last = run(FALSE, FALSE)
  )
})

write_json(
  list(distribute = distribute, lab = lab, time = time, reveal = reveal),
  "tests/fixtures/animation_reference.json",
  auto_unbox = TRUE, digits = NA, pretty = TRUE
)
