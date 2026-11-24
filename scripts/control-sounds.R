# Writes control's two sounds into crates/kotoba-app/assets/sounds: a low F sharp in octaves, then the C sharp above it, as control starts listening, and the two the other way round as it stops. Each note is a sine with a trace of its overtones, struck soft and gone in a tenth of a second, with a few quiet, darker echoes after it, as a small room has. As loud as dictation's chime over its first 200 ms. Firmer, brighter or many-noted candidates sounded like a warning. Base R alone.
#
#   Rscript scripts/control-sounds.R

RATE <- 44100
OUT <- file.path(dirname(sub("^--file=", "", grep("^--file=", commandArgs(FALSE), value = TRUE))), "..", "crates", "kotoba-app", "assets", "sounds")

hz <- function(name) {
  n <- c(C = -9, Db = -8, D = -7, Eb = -6, E = -5, F = -4, Gb = -3, G = -2, Ab = -1, A = 0, Bb = 1, B = 2)
  m <- regmatches(name, regexec("^([A-G]b?)([0-9])$", name))[[1]]
  440 * 2^((n[[m[2]]] + 12 * (as.integer(m[3]) - 4)) / 12)
}

tt <- function(len) (seq_len(round(len * RATE)) - 1) / RATE

# A raised-cosine attack, an exponential decay, and a short fade at the end.
env <- function(t, attack, decay) {
  a <- ifelse(t < attack, 0.5 - 0.5 * cos(pi * t / attack), 1)
  n <- length(t)
  a * exp(-t / decay) * pmin(1, (n - seq_len(n)) / (0.02 * RATE))
}

# One pole, run twice for a gentler slope.
lowpass <- function(x, cutoff) {
  k <- 1 - exp(-2 * pi * cutoff / RATE)
  run <- function(x) as.numeric(stats::filter(k * x, 1 - k, method = "recursive"))
  run(run(x))
}

soft <- function(note, len, attack, decay) {
  t <- tt(len); f <- hz(note)
  (sin(2 * pi * f * t) + 0.08 * sin(4 * pi * f * t) + 0.03 * sin(6 * pi * f * t)) * env(t, attack, decay)
}
notes <- function(names, len, decay) Reduce(`+`, lapply(names, soft, len, 0.004, decay))

# A small room: a few quiet, darker echoes.
room <- function(x, wet = 0.22) {
  taps <- c(0.023, 0.037, 0.051, 0.067, 0.089, 0.113)
  gains <- wet * c(1, 0.8, 0.65, 0.5, 0.38, 0.28)
  n <- length(x) + round(0.35 * RATE)
  y <- c(x, numeric(n - length(x)))
  dark <- lowpass(y, 1800)
  for (k in seq_along(taps)) {
    d <- round(taps[k] * RATE)
    y[(d + 1):n] <- y[(d + 1):n] + gains[k] * dark[1:(n - d)]
  }
  y * pmin(1, (n - seq_len(n)) / (0.08 * RATE))
}

# Sounds one after another `gap` seconds apart, scaled so the first 200 ms are as loud as dictation's chime (an RMS of 0.105), the peak kept under 0.6.
mix <- function(xs, gap = 0) {
  starts <- round((seq_along(xs) - 1) * gap * RATE)
  y <- numeric(max(starts + lengths(xs)))
  for (i in seq_along(xs)) { j <- starts[i] + seq_along(xs[[i]]); y[j] <- y[j] + xs[[i]] }
  w <- y[seq_len(min(length(y), round(0.2 * RATE)))]
  y <- y * 0.105 / sqrt(mean(w^2))
  if (max(abs(y)) > 0.6) y <- y * 0.6 / max(abs(y))
  y
}

write_wav <- function(y, path) {
  s <- as.integer(round(pmax(-1, pmin(1, y)) * 32767))
  con <- file(path, "wb"); on.exit(close(con))
  n <- length(s)
  writeBin(charToRaw("RIFF"), con); writeBin(as.integer(36 + 2 * n), con, size = 4, endian = "little")
  writeBin(charToRaw("WAVEfmt "), con)
  writeBin(16L, con, size = 4, endian = "little")
  writeBin(c(1L, 1L), con, size = 2, endian = "little")
  writeBin(c(as.integer(RATE), as.integer(RATE * 2)), con, size = 4, endian = "little")
  writeBin(c(2L, 16L), con, size = 2, endian = "little")
  writeBin(charToRaw("data"), con); writeBin(as.integer(2 * n), con, size = 4, endian = "little")
  writeBin(s, con, size = 2, endian = "little")
}

tap <- function(first, second) mix(list(room(mix(list(first, second), 0.09))))

write_wav(tap(notes(c("Gb2", "Gb3"), 0.3, 0.09), notes("Db4", 0.35, 0.11)), file.path(OUT, "control-start.wav"))
write_wav(tap(notes("Db4", 0.3, 0.09), notes(c("Gb2", "Gb3"), 0.35, 0.11)), file.path(OUT, "control-stop.wav"))
cat("control-start.wav and control-stop.wav written to", normalizePath(OUT), "\n")
