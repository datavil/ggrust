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

write_json(
  list(distribute = distribute, lab = lab),
  "tests/fixtures/animation_reference.json",
  auto_unbox = TRUE, digits = NA, pretty = TRUE
)
