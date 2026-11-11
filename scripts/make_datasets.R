# Write the example datasets shipped in python/ggrust/data (see THIRD_PARTY_NOTICES.md).
# Run from the repository root: Rscript scripts/make_datasets.R
# Needs R >= 4.5 (datasets::penguins) and ggplot2 (mpg, economics).
library(ggplot2)
out <- "python/ggrust/data"
write <- function(frame, name) {
  write.csv(frame, file.path(out, paste0(name, ".csv")), row.names = FALSE, na = "")
}

# Palmer penguins (CC0), with the palmerpenguins column names plotnine also uses.
penguins <- datasets::penguins
names(penguins) <- c("species", "island", "bill_length_mm", "bill_depth_mm",
                     "flipper_length_mm", "body_mass_g", "sex", "year")
write(penguins, "penguins")

# Fuel economy of 38 car models, 1999 and 2008 (ggplot2, MIT; EPA data).
write(as.data.frame(mpg), "mpg")

# US economic time series, 1967 to 2015 (ggplot2, MIT; FRED data).
economics_frame <- as.data.frame(economics)
economics_frame$date <- format(economics_frame$date, "%Y-%m-%d")
write(economics_frame, "economics")

# Fisher's iris measurements (public domain), with snake_case column names.
iris_frame <- datasets::iris
names(iris_frame) <- c("sepal_length", "sepal_width", "petal_length", "petal_width", "species")
write(iris_frame, "iris")
