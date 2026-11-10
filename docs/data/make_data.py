"""Write the small everyday datasets the docs examples read.

Run once after changing a dataset: ``uv run python docs/data/make_data.py``.
The random datasets are seeded, so the CSV files only change on purpose.
Climate and fruit numbers are rounded, approximate public figures.
The chemical elements come from the lets-plot docs (CC BY-SA 4.0, see ``ELEMENTS_URL``).
The datasets of the lets-plot gallery remakes are downloaded from their sources (see
``LETS_PLOT_DATA`` and ``NETFLIX_URL``) and trimmed to the columns the examples use.
"""

import io
import urllib.request
from pathlib import Path

import numpy as np
import polars as pl

DATA = Path(__file__).parent

# Source of the periodic table data, as used by the lets-plot periodic table demo.
# Licensed CC BY-SA 4.0; the cleaned copy in elements.csv is shared under the same license.
ELEMENTS_URL = (
    "https://raw.githubusercontent.com/JetBrains/lets-plot-docs/master/data/chemical_elements.csv"
)

# Data of the lets-plot gallery demos remade in the ggrust gallery.
LETS_PLOT_DATA = "https://raw.githubusercontent.com/JetBrains/lets-plot-docs/master/data/"
# scIB benchmark metrics (Luecken et al. 2022, Nature Methods), MIT licensed.
SCIB_URL = (
    "https://raw.githubusercontent.com/theislab/scib-reproducibility/main/"
    "visualization/data/metrics_RNA_allTasks.csv"
)
# Netflix titles as of 2021 (Kaggle "Netflix Movies and TV Shows", via TidyTuesday).
NETFLIX_URL = (
    "https://raw.githubusercontent.com/rfordatascience/tidytuesday/master/"
    "data/2021/2021-04-20/netflix_titles.csv"
)

# Names, symbols and types that changed after the source was written.
ELEMENT_FIXES = {
    113: ("Nihonium", "Nh", "Metal"),
    114: ("Flerovium", "Fl", "Metal"),
    115: ("Moscovium", "Mc", "Metal"),
    116: ("Livermorium", "Lv", "Metal"),
    117: ("Tennessine", "Ts", "Halogen"),
    118: ("Oganesson", "Og", "Noble Gas"),
}
TYPE_FIXES = {
    "Francium": "Alkali Metal",
    "Radium": "Alkaline Earth Metal",
    "Astatine": "Halogen",
    "Radon": "Noble Gas",
}

MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]

# Approximate monthly mean temperature (°C) and typical daily range.
CLIMATE = {
    "Oslo": ([-4.3, -4.0, -0.2, 4.5, 10.8, 15.2, 16.4, 15.2, 10.8, 6.3, 0.7, -3.1], 7.0),
    "Toronto": ([-5.5, -4.5, 0.0, 7.1, 13.1, 18.6, 21.5, 20.6, 16.2, 9.5, 3.7, -2.2], 9.0),
    "Lima": ([22.1, 22.6, 22.1, 20.4, 18.6, 17.3, 16.5, 16.1, 16.5, 17.4, 18.8, 20.6], 6.0),
    "Pune": ([21.0, 23.4, 27.5, 30.4, 30.6, 27.1, 25.0, 24.5, 24.8, 25.6, 23.0, 21.0], 14.0),
    "Sydney": ([23.0, 23.0, 21.8, 19.2, 16.2, 13.8, 12.9, 14.0, 16.5, 18.6, 20.2, 21.9], 8.0),
}

# Approximate nutrition per 100 g: calories (kcal), sugar (g), fibre (g), vitamin C (mg).
FRUIT = {
    "Apple": (52, 10.4, 2.4, 4.6),
    "Avocado": (160, 0.7, 6.7, 10.0),
    "Banana": (89, 12.2, 2.6, 8.7),
    "Blueberry": (57, 10.0, 2.4, 9.7),
    "Cherry": (63, 12.8, 2.1, 7.0),
    "Coconut": (354, 6.2, 9.0, 3.3),
    "Date": (282, 63.4, 8.0, 0.4),
    "Grape": (69, 15.5, 0.9, 3.2),
    "Guava": (68, 8.9, 5.4, 228.3),
    "Kiwi": (61, 9.0, 3.0, 92.7),
    "Lemon": (29, 2.5, 2.8, 53.0),
    "Mango": (60, 13.7, 1.6, 36.4),
    "Orange": (47, 9.4, 2.4, 53.2),
    "Papaya": (43, 7.8, 1.7, 60.9),
    "Pear": (57, 9.8, 3.1, 4.3),
    "Pineapple": (50, 9.9, 1.4, 47.8),
    "Raspberry": (52, 4.4, 6.5, 26.2),
    "Strawberry": (32, 4.9, 2.0, 58.8),
    "Watermelon": (30, 6.2, 0.4, 8.1),
}


def cities() -> pl.DataFrame:
    """Return monthly mean, low and high temperatures for five cities."""
    rows = []
    for city, (means, daily_range) in CLIMATE.items():
        for month_index, mean in enumerate(means):
            rows.append(
                {
                    "city": city,
                    "month": month_index + 1,
                    "month_name": MONTHS[month_index],
                    "temperature": mean,
                    "low": round(mean - daily_range / 2, 1),
                    "high": round(mean + daily_range / 2, 1),
                }
            )
    return pl.DataFrame(rows)


def commute(generator: np.random.Generator, n: int = 1500) -> pl.DataFrame:
    """Return simulated commute times by city and transport mode."""
    modes = ["Walk", "Bike", "Bus", "Train", "Car"]
    speed_kmh = {"Walk": 5.0, "Bike": 16.0, "Bus": 18.0, "Train": 35.0, "Car": 28.0}
    overhead_minutes = {"Walk": 0.0, "Bike": 3.0, "Bus": 9.0, "Train": 12.0, "Car": 7.0}
    max_distance_km = {"Walk": 3.5, "Bike": 12.0, "Bus": 18.0, "Train": 40.0, "Car": 45.0}
    city_names = ["Oslo", "Toronto", "Lima"]
    city_traffic = {"Oslo": 1.0, "Toronto": 1.25, "Lima": 1.5}
    mode = generator.choice(modes, size=n, p=[0.12, 0.18, 0.25, 0.2, 0.25])
    city = generator.choice(city_names, size=n)
    distance = np.array(
        [generator.uniform(0.4, max_distance_km[m]) ** 0.9 for m in mode],
    ).round(1)
    traffic = np.array(
        [city_traffic[c] if m in ("Bus", "Car") else 1.0 for c, m in zip(city, mode, strict=True)]
    )
    minutes = (
        np.array([overhead_minutes[m] for m in mode])
        + distance / np.array([speed_kmh[m] for m in mode]) * 60 * traffic
    ) * generator.lognormal(0.0, 0.18, size=n)
    return pl.DataFrame(
        {"city": city, "mode": mode, "distance_km": distance, "minutes": minutes.round(1)}
    )


def coffee(generator: np.random.Generator) -> pl.DataFrame:
    """Return simulated cups sold per weekday and drink at a small café."""
    weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    base = {"Espresso": 48, "Cappuccino": 70, "Latte": 85, "Tea": 35, "Hot chocolate": 18}
    weekday_factor = [0.9, 0.95, 1.0, 1.0, 1.1, 1.35, 1.2]
    rows = []
    for weekday, factor in zip(weekdays, weekday_factor, strict=True):
        for drink, cups in base.items():
            rows.append(
                {
                    "weekday": weekday,
                    "drink": drink,
                    "cups": int(generator.poisson(cups * factor)),
                }
            )
    return pl.DataFrame(rows)


def homes(generator: np.random.Generator, n: int = 600) -> pl.DataFrame:
    """Return simulated home sales: floor area, rooms, price by neighbourhood."""
    neighbourhoods = {"Old town": 7.2, "Riverside": 5.6, "Suburbs": 3.9, "Harbour": 6.4}
    names = list(neighbourhoods)
    neighbourhood = generator.choice(names, size=n, p=[0.2, 0.25, 0.4, 0.15])
    area = generator.gamma(6.0, 15.0, size=n).clip(25, 320).round(0)
    rooms = np.clip(np.round(area / 28 + generator.normal(0, 0.6, size=n)), 1, 9).astype(int)
    price_per_m2 = np.array([neighbourhoods[name] for name in neighbourhood])
    price = area * price_per_m2 * generator.lognormal(0.0, 0.15, size=n)
    built = generator.integers(1900, 2025, size=n)
    return pl.DataFrame(
        {
            "neighbourhood": neighbourhood,
            "area_m2": area,
            "rooms": rooms,
            "year_built": built,
            "price_k": price.round(0),
        }
    )


def fruit() -> pl.DataFrame:
    """Return approximate nutrition values per 100 g for common fruit."""
    return pl.DataFrame(
        [
            {"fruit": name, "calories": c, "sugar_g": s, "fibre_g": f, "vitamin_c_mg": v}
            for name, (c, s, f, v) in FRUIT.items()
        ]
    )


def elements() -> pl.DataFrame:
    """Return the 118 chemical elements with their position in the periodic table.

    Applies the same fixes as the lets-plot demo (new names for 113 to 118, a few
    types). The tile labels format the atomic masses (``label_format``).
    """
    with urllib.request.urlopen(ELEMENTS_URL) as response:
        text = response.read().decode("mac_roman")
    source = pl.read_csv(io.StringIO(text))
    rows = []
    for row in source.iter_rows(named=True):
        number = row["Atomic Number"]
        name, symbol = row["Element"].strip(), row["Symbol"].strip()
        kind = TYPE_FIXES.get(name, row["Type"])
        if number in ELEMENT_FIXES:
            name, symbol, kind = ELEMENT_FIXES[number]
        if kind == "Transactinide":
            kind = "Transition Metal"
        rows.append(
            {
                "atomic_number": number,
                "element": name,
                "symbol": symbol,
                "atomic_mass": float(row["Atomic Weight"]),
                "period": row["Period"],
                "group": row["Group"],
                "type": kind,
                "electron_configuration": row["Electron Configuration"],
            }
        )
    return pl.DataFrame(rows)


def download(url: str) -> bytes:
    """Return the bytes at ``url``."""
    with urllib.request.urlopen(url) as response:
        return response.read()


def netflix() -> pl.DataFrame:
    """Return Netflix movies released since 2000: genres (comma-separated) and minutes."""
    titles = pl.read_csv(io.BytesIO(download(NETFLIX_URL)))
    return titles.filter(
        pl.col("type") == "Movie",
        pl.col("release_year") >= 2000,
        pl.col("duration").is_not_null(),
    ).select(
        pl.col("listed_in").alias("genres"),
        pl.col("duration").str.extract(r"(\d+)").cast(pl.Int64).alias("minutes"),
    )


def penguins() -> pl.DataFrame:
    """Return the Palmer penguins (CC0, Horst, Hill and Gorman)."""
    return pl.read_csv(io.BytesIO(download(LETS_PLOT_DATA + "penguins.csv")), null_values="NA")


def pulsar() -> pl.DataFrame:
    """Return the radio pulses of pulsar CP 1919, one row per pulse, as on the album cover."""
    return pl.read_csv(io.BytesIO(download(LETS_PLOT_DATA + "pulsar.csv")), has_header=False)


def unemployment() -> pl.DataFrame:
    """Return monthly US unemployment in thousands (ggplot2's economics, from FRED)."""
    economics = pl.read_csv(
        io.BytesIO(download(LETS_PLOT_DATA + "economics.csv")), infer_schema_length=None
    )
    return economics.select("date", "unemploy")


def presidents() -> pl.DataFrame:
    """Return US presidential terms and parties (ggplot2's presidential)."""
    presidential = pl.read_csv(io.BytesIO(download(LETS_PLOT_DATA + "presidential.csv")))
    return presidential.select("name", "start", "end", "party")


def bitcoin() -> pl.DataFrame:
    """Return the daily Bitcoin close price in US dollars on Bitstamp, 2024."""
    prices = pl.read_csv(
        io.BytesIO(download(LETS_PLOT_DATA + "Bitstamp_BTCUSD_d.csv")), infer_schema_length=None
    )
    return (
        prices.select(pl.col("date").str.slice(0, 10), pl.col("close"))
        .filter(pl.col("date").str.starts_with("2024"))
        .sort("date")
    )


def scib_immune() -> pl.DataFrame:
    """Return the scIB integration metrics of the human immune cell task (69 runs)."""
    metrics = pl.read_csv(io.BytesIO(download(SCIB_URL)), infer_schema_length=None)
    metrics = metrics.rename({metrics.columns[0]: "path"})
    parts = pl.col("path").str.strip_prefix("/").str.split("/")
    method_output = parts.list.get(4).str.split("_")
    return (
        metrics.filter(pl.col("path").str.starts_with("/immune_cell_hum/"))
        .filter(pl.col("NMI_cluster/label").is_not_null())
        .with_columns(
            scaling=parts.list.get(2),
            features=parts.list.get(3),
            method=method_output.list.get(0),
            output=method_output.list.get(1),
        )
        .filter(~((pl.col("method") == "trvae") & (pl.col("output") == "full")))
        .select(
            "method",
            "output",
            "features",
            "scaling",
            pl.exclude("path", "method", "output", "features", "scaling"),
        )
    )


if __name__ == "__main__":
    generator = np.random.default_rng(2026)
    cities().write_csv(DATA / "cities.csv")
    commute(generator).write_csv(DATA / "commute.csv")
    coffee(generator).write_csv(DATA / "coffee.csv")
    homes(generator).write_csv(DATA / "homes.csv")
    fruit().write_csv(DATA / "fruit.csv")
    elements().write_csv(DATA / "elements.csv")
    netflix().write_csv(DATA / "netflix.csv")
    penguins().write_csv(DATA / "penguins.csv")
    pulsar().write_csv(DATA / "pulsar.csv", include_header=False)
    unemployment().write_csv(DATA / "unemployment.csv")
    presidents().write_csv(DATA / "presidents.csv")
    bitcoin().write_csv(DATA / "bitcoin.csv")
    scib_immune().write_csv(DATA / "scib_immune.csv")
