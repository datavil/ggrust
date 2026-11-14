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
# - "align": lines and polygons aligned before tweening, from transformr
#   (MIT, Thomas Lin Pedersen) `add_points`, `match_path`, `match_polygon`,
#   `make_polygons` and `as_clockwise` (R/aaa.R, R/tween_polygon.R) copied
#   below, with `find_splits_c`, `insert_points_c` and `rotate_c`
#   (src/align_rings.cpp) translated to R, because transformr is not installed.
# - "enter_exit": one tweened frame between two states with enter and exit
#   effects: tweenr's `.complete_states` (R/tween_state.R, MIT) and gganimate's
#   `transmute_fade` / `transmute_grow` (R/transmuters.R, MIT), copied below
#   with the id fixed to gganimate's group, then every column mixed linearly
#   (colours in Lab with alpha, as above).
# - "wake": shadow_wake's frames and the eased step values (gganimate
#   R/shadow-wake.R `setup_params`, `get_frames`, `prepare_shadow`, MIT),
#   with tweenr's cubic-in ease.

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

# transformr src/align_rings.cpp, translated to R (0-based C++ loops kept).
find_splits <- function(lengths, n) {
  total <- 0
  size <- length(lengths)
  splits <- integer(size)
  i <- 0
  while (i < size - 1) {
    total <- total + lengths[i + 1]
    if (total / (n + 1) > lengths[i + 2]) break
    i <- i + 1
  }
  if (i == size - 1) total <- total + lengths[i + 1]
  n_remain <- n
  j <- 0
  while (n_remain > 0) {
    if (j >= size) {
      splits[1] <- splits[1] + n_remain
      break
    }
    # C++ std::round rounds halves away from zero.
    split <- if (total == 0) n_remain else trunc(n * lengths[j + 1] / total + 0.5)
    if (split == 0) split <- 1
    split <- min(split, n_remain)
    splits[j + 1] <- split
    n_remain <- n_remain - split
    j <- j + 1
  }
  splits
}
insert_points <- function(x, y, splits, n) {
  new_x <- c()
  new_y <- c()
  size <- length(x)
  for (i in seq_len(size)) {
    new_x <- c(new_x, x[i])
    new_y <- c(new_y, y[i])
    if (splits[i] == 0) next
    next_i <- if (i == size) 1 else i + 1
    x_seg <- (x[next_i] - x[i]) / (splits[i] + 1)
    y_seg <- (y[next_i] - y[i]) / (splits[i] + 1)
    for (j in seq_len(splits[i])) {
      new_x <- c(new_x, x[i] + x_seg * j)
      new_y <- c(new_y, y[i] + y_seg * j)
    }
  }
  list(x = new_x, y = new_y)
}
rotate <- function(poly_x, poly_y, ref_x, ref_y) {
  best_ss <- Inf
  best_i <- 0
  n <- length(poly_x)
  for (i in 0:(n - 1)) {
    j <- 0:(n - 1)
    j_off <- (i + j) %% n
    ss <- sum((ref_x[j + 1] - poly_x[j_off + 1])^2 + (ref_y[j + 1] - poly_y[j_off + 1])^2)
    if (ss < best_ss) {
      best_ss <- ss
      best_i <- i
    }
  }
  best_i + 1
}

# transformr R/aaa.R and R/tween_polygon.R (one ring per shape, no holes).
add_points <- function(polygon, n, connect = TRUE) {
  if (n < 1) return(polygon)
  if (nrow(polygon) == 1) return(polygon[rep(1, n + 1), , drop = FALSE])
  if (connect) {
    x <- diff(c(polygon$x, polygon$x[1]))
    y <- diff(c(polygon$y, polygon$y[1]))
  } else {
    x <- diff(polygon$x)
    y <- diff(polygon$y)
  }
  l <- sqrt(x * x + y * y)
  longest <- order(l, decreasing = TRUE)
  n_splits <- find_splits(l[longest], n)
  splits <- n_splits[match(seq_along(l), longest)]
  if (!connect) splits <- c(splits, 0)
  new_points <- insert_points(polygon$x, polygon$y, splits, n)
  new_polygon <- polygon[rep(seq_len(nrow(polygon)), splits + 1), ]
  new_polygon$x <- new_points$x
  new_polygon$y <- new_points$y
  new_polygon
}
pad_ring <- function(ring) {
  nr <- nrow(ring)
  if (nr > 0 && nr < 4) ring[c(seq_len(nr), rep(1, 4 - nr)), ] else ring
}
as_clockwise <- function(p) {
  x <- c(p$x, p$x[1])
  y <- c(p$y, p$y[1])
  area <- (x[-1] - x[-length(x)]) * (y[-1] + y[-length(y)])
  if (sum(area) < 0) p <- p[rev(seq_len(nrow(p))), ]
  p
}
match_polygon <- function(main_from, main_to, min_n = 50) {
  main_from <- as_clockwise(pad_ring(main_from))
  main_to <- as_clockwise(pad_ring(main_to))
  n_points <- max(c(min_n, nrow(main_from), nrow(main_to)))
  if (nrow(main_from) < n_points) main_from <- add_points(main_from, n_points - nrow(main_from), connect = TRUE)
  if (nrow(main_to) < n_points) main_to <- add_points(main_to, n_points - nrow(main_to), connect = TRUE)
  offset <- rotate(main_to$x, main_to$y, main_from$x, main_from$y)
  to_end <- seq_len(nrow(main_to)) < offset
  main_to <- rbind(main_to[!to_end, , drop = FALSE], main_to[to_end, , drop = FALSE])
  list(from = main_from, to = main_to)
}
match_path <- function(from, to, min_n = 50) {
  if (nrow(from) == 1) from <- from[c(1, 1), ]
  if (nrow(to) == 1) to <- to[c(1, 1), ]
  n_points <- max(c(min_n, nrow(from), nrow(to)))
  if (nrow(from) < n_points) from <- add_points(from, n_points - nrow(from), connect = FALSE)
  if (nrow(to) < n_points) to <- add_points(to, n_points - nrow(to), connect = FALSE)
  list(from = from, to = to)
}

align_cases <- list(
  list(kind = "polygon", min_n = 50,
       from = data.frame(x = c(0, 1, 0.5), y = c(0, 0, 1)),
       to = data.frame(x = c(2, 3, 3, 2, 2.5), y = c(0, 0, 1, 1, 1.6))),
  list(kind = "polygon", min_n = 7,
       from = data.frame(x = c(0, 4, 4, 0), y = c(0, 0, 3, 3)),
       to = data.frame(x = c(1, 2, 1.5), y = c(1, 1, 2))),
  list(kind = "path", min_n = 50,
       from = data.frame(x = c(0, 1, 2), y = c(0, 2, 1)),
       to = data.frame(x = c(0, 1, 2, 3, 4), y = c(1, 1, 3, 0, 2))),
  list(kind = "path", min_n = 6,
       from = data.frame(x = c(0, 3, 4), y = c(0, 0, 0)),
       to = data.frame(x = c(5), y = c(5))),
  list(kind = "path", min_n = 9,
       from = data.frame(x = c(0, 1, 2, 3), y = c(0, 0, 0, 0)),
       to = data.frame(x = c(0, 1), y = c(1, 1)))
)
align <- lapply(align_cases, function(case) {
  matched <- if (case$kind == "polygon") {
    match_polygon(case$from, case$to, case$min_n)
  } else {
    match_path(case$from, case$to, case$min_n)
  }
  list(
    kind = case$kind, min_n = case$min_n,
    from = list(x = case$from$x, y = case$from$y),
    to = list(x = case$to$x, y = case$to$y),
    aligned_from = list(x = matched$from$x, y = matched$from$y),
    aligned_to = list(x = matched$to$x, y = matched$to$y)
  )
})

# tweenr .complete_states with gganimate's id (group): rows matched by
# <group>_<occurrence>; entering rows (enter(to rows)) appended to `from`,
# exiting rows (exit(from rows)) appended to `to`, `to` reordered to `from`.
count_occourance <- function(x) {
  if (length(x) == 0) return(integer(0))
  unsplit(lapply(split(x, x), seq_along), x)
}
complete_states <- function(from, to, enter, exit) {
  from_id <- paste(from$group, count_occourance(from$group), sep = "_")
  to_id <- paste(to$group, count_occourance(to$group), sep = "_")
  entering <- !to_id %in% from_id
  exiting <- !from_id %in% to_id
  if (is.null(enter) || sum(entering) == 0) {
    to <- to[!entering, , drop = FALSE]
    to_id <- to_id[!entering]
    enters <- to[0, , drop = FALSE]
    enter_id <- to_id[0]
  } else {
    enters <- enter(to[entering, , drop = FALSE])
    enter_id <- to_id[entering]
  }
  if (is.null(exit) || sum(exiting) == 0) {
    from <- from[!exiting, , drop = FALSE]
    from_id <- from_id[!exiting]
    exits <- from[0, , drop = FALSE]
    exit_id <- from_id[0]
  } else {
    exits <- exit(from[exiting, , drop = FALSE])
    exit_id <- from_id[exiting]
  }
  from <- rbind(from, enters)
  from_id <- c(from_id, enter_id)
  to <- rbind(to, exits)
  to_id <- c(to_id, exit_id)
  list(from = from, to = to[match(from_id, to_id), , drop = FALSE])
}
# gganimate transmute_fade default, transmute_grow default / bar / path.
fade <- function(alpha) function(x) {
  no_alpha <- is.na(x$alpha)
  x$alpha[!no_alpha] <- alpha
  x$colour[no_alpha] <- scales::alpha(x$colour[no_alpha], alpha)
  x
}
grow_point <- function(size) function(x) {
  x$size <- size * x$size
  x$stroke <- size * x$stroke
  x
}
grow_bar <- function(size) function(x) {
  x$y <- x$y * size
  x$ymax <- x$ymax * size
  x
}
shrink_multielement <- function(size) function(x) {
  mean_x <- vapply(split(x$x, x$group), mean, numeric(1))
  mean_y <- vapply(split(x$y, x$group), mean, numeric(1))
  group <- match(x$group, names(mean_x))
  if (size == 0) {
    x$x <- mean_x[group]
    x$y <- mean_y[group]
  } else {
    x$x <- mean_x[group] + size * (x$x - mean_x[group])
    x$y <- mean_y[group] + size * (x$y - mean_y[group])
  }
  x
}
chain <- function(...) {
  fs <- list(...)
  if (length(fs) == 0) return(NULL)
  function(x) Reduce(function(d, f) f(d), fs, x)
}
mix_frame <- function(from, to, p) {
  out <- from
  for (column in names(from)) {
    if (column == "group") next
    if (column == "colour") {
      ends_from <- decode_colour(from$colour, alpha = TRUE, to = "lab")
      ends_to <- decode_colour(to$colour, alpha = TRUE, to = "lab")
      mixed <- ends_from + p * (ends_to - ends_from)
      out$colour <- encode_colour(mixed[, 1:3, drop = FALSE], alpha = mixed[, 4], from = "lab")
    } else {
      out[[column]] <- from[[column]] + p * (to[[column]] - from[[column]])
    }
  }
  out
}
enter_exit_cases <- list(
  list(
    geom = "point", progress = 0.25,
    enter = list(list(name = "fade", alpha = 0), list(name = "grow", size = 0)),
    exit = list(list(name = "fade", alpha = 0.2)),
    from = data.frame(
      group = c(1, 2, 3), x = c(1, 2, 3), y = c(1, 2, 3), size = c(3, 3, 4),
      stroke = c(0.75, 0.75, 0.75), alpha = c(0.8, NA, NA),
      colour = c("#FF0000", "#0000FF", "#00AA00")
    ),
    to = data.frame(
      group = c(3, 2, 4), x = c(6, 5, 7), y = c(3, 1, 0), size = c(4, 5, 2),
      stroke = c(0.75, 0.75, 1.5), alpha = c(NA, NA, NA),
      colour = c("#00AA00", "#000000", "#808080")
    ),
    enter_fn = chain(fade(0), grow_point(0)), exit_fn = chain(fade(0.2))
  ),
  list(
    geom = "bar", progress = 0.6,
    enter = list(list(name = "grow", size = 0)),
    exit = list(list(name = "shrink", size = 0.5)),
    from = data.frame(group = c(0, 1), x = c(1, 2), y = c(4, 6), ymin = c(0, 0), ymax = c(4, 6)),
    to = data.frame(group = c(1, 2), x = c(2, 3), y = c(8, 5), ymin = c(0, 0), ymax = c(8, 5)),
    enter_fn = chain(grow_bar(0)), exit_fn = chain(grow_bar(0.5))
  ),
  list(
    geom = "line", progress = 0.5,
    enter = list(list(name = "grow", size = 0)),
    exit = list(),
    from = data.frame(group = c(0, 0), x = c(0, 1), y = c(0, 1)),
    to = data.frame(group = c(0, 0, 1, 1, 1), x = c(0, 2, 0, 1, 5), y = c(1, 1, 3, 4, 8)),
    enter_fn = chain(shrink_multielement(0)), exit_fn = NULL
  )
)
enter_exit <- lapply(enter_exit_cases, function(case) {
  full <- complete_states(case$from, case$to, case$enter_fn, case$exit_fn)
  list(
    geom = case$geom, progress = case$progress, enter = case$enter, exit = case$exit,
    from = case$from, to = case$to,
    frame = mix_frame(full$from, full$to, case$progress)
  )
})

# gganimate shadow_wake: frames and steps; tweenr's cubic-in is t^3.
wake_frames <- function(i, nframes, wake_length, wrap) {
  wake_length <- round(nframes * wake_length)
  frames <- rev(i - seq_len(wake_length))
  if (wrap) {
    frames <- frames %% nframes
    frames[frames == 0] <- nframes
  } else {
    frames <- frames[frames > 0 & frames <= nframes]
  }
  at <- seq(0, 1, length = wake_length + 1)[seq_len(wake_length)]
  list(frames = frames, at = at[seq_along(frames)])
}
wake <- lapply(list(
  list(i = 1, nframes = 37, wake_length = 0.1, wrap = TRUE),
  list(i = 3, nframes = 37, wake_length = 0.1, wrap = FALSE),
  list(i = 20, nframes = 50, wake_length = 0.25, wrap = TRUE),
  list(i = 5, nframes = 20, wake_length = 0.125, wrap = TRUE)
), function(case) {
  planned <- wake_frames(case$i, case$nframes, case$wake_length, case$wrap)
  eased <- planned$at^3
  colour <- "#3366CC80"
  # mod_alpha: the colour's alpha tweened to 0; size tweened to 0.
  alpha <- 0 + eased * (col2rgb(colour, TRUE)[4, ] / 255 - 0)
  c(case, list(
    frames = planned$frames, at = planned$at,
    size = 0 + eased * (4 - 0),
    colour = scales::alpha(rep(colour, length(eased)), alpha)
  ))
})

write_json(
  list(
    distribute = distribute, lab = lab, time = time, reveal = reveal,
    align = align, enter_exit = enter_exit, wake = wake
  ),
  "tests/fixtures/animation_reference.json",
  auto_unbox = TRUE, digits = NA, pretty = TRUE
)
