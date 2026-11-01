//! Date-time and duration axes, as lets-plot's `DateTimeBreaksHelper`,
//! `DateTimeFixedBreaksGen`, `TimeBreaksGen` and `DateTimeFormat`.
//!
//! Values are milliseconds: since the Unix epoch for date-times (dates at UTC
//! midnight, times of day since midnight), plain milliseconds for durations.
//!
//! - Date-time breaks: under a second per break, linear breaks; past a year,
//!   linear whole years; otherwise the nearest "nice" interval (1 s ... 3 months)
//!   stepped through the calendar. Labels use the interval's pattern (`%Y`,
//!   `%b`, `%b %e`, `%H:%M`, `%M:%S`). `break_width="2 weeks"` fixes the
//!   interval. Dates never break below a day; times of day never above 12 hours.
//! - Duration breaks: the nearest nice step, labelled `1d 2:30`, `0:15` ...
//! - Patterns: lets-plot's subset of strftime (`%Y %y %m %b %B %d %e %j %a %A %w
//!   %H %I %l %M %S %f %p %P`), English names.
//!
//! What it does not do: time zones other than UTC (lets-plot shows local time
//! for zoned columns; ggrust warns and uses UTC), patterns on duration axes.

use super::breaks::linear_breaks;

const SECOND: f64 = 1000.0;
const MINUTE: f64 = 60.0 * SECOND;
const HOUR: f64 = 60.0 * MINUTE;
const DAY: f64 = 24.0 * HOUR;
const WEEK: f64 = 7.0 * DAY;
/// lets-plot `YearInterval.MS` (365 days).
const YEAR: f64 = 365.0 * DAY;

/// What a temporal axis holds (lets-plot `DataType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalKind {
    /// Instants (`datetime` columns, `scale_*_datetime()`).
    DateTime,
    /// Days (`date` columns): breaks at least a day apart.
    Date,
    /// Times of day (`time` columns): breaks at most 12 hours apart.
    TimeOfDay,
    /// Durations (`scale_*_time()`).
    Duration,
}

/// A temporal position axis: its kind and the user's `break_width` and `format`.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalAxis {
    /// What the values are.
    pub kind: TemporalKind,
    /// `break_width=...`.
    pub break_width: Option<TimeInterval>,
    /// `format=...` (date-time axes).
    pub format: Option<DatePattern>,
}

impl TemporalAxis {
    /// About `count` breaks over `[low, high]` (milliseconds) with their labels.
    pub fn breaks(&self, low: f64, high: f64, count: usize) -> (Vec<f64>, Vec<String>) {
        if !(low.is_finite() && high.is_finite()) || high < low {
            return (Vec::new(), Vec::new());
        }
        // A width far below the span would make thousands of breaks (or hang):
        // widen it to a whole multiple that gives at most MAX_FIXED_BREAKS.
        let break_width = self
            .break_width
            .map(|interval| interval.at_most(MAX_FIXED_BREAKS, high - low));
        if self.kind == TemporalKind::Duration {
            let (breaks, width) = match &break_width {
                Some(interval) => {
                    let width = interval.nominal_milliseconds();
                    (multiples(low, high, width), width)
                }
                None => duration_breaks(low, high, count.max(1)),
            };
            let labels = format_durations(&breaks, width, high - low);
            return (breaks, labels);
        }
        let (breaks, pattern) = match &break_width {
            Some(interval) => (interval.range(low, high), interval.pattern()),
            None => datetime_breaks(low, high, count.max(1), self.kind),
        };
        let pattern = self
            .format
            .clone()
            .unwrap_or_else(|| DatePattern::parse(pattern));
        let labels = breaks.iter().map(|&value| pattern.format(value)).collect();
        (breaks, labels)
    }

    /// The label of one value (fixed `breaks=[...]`): `format`, else the pattern
    /// generated breaks over `[low, high]` would use.
    pub fn label(&self, value: f64, low: f64, high: f64, count: usize) -> String {
        if self.kind == TemporalKind::Duration {
            return format_duration(value, false, 0.0);
        }
        let pattern = match &self.format {
            Some(pattern) => pattern.clone(),
            None => {
                let generated = match &self.break_width {
                    Some(interval) => interval.pattern(),
                    None => datetime_breaks(low, high, count.max(1), self.kind).1,
                };
                DatePattern::parse(generated)
            }
        };
        pattern.format(value)
    }
}

/// A calendar interval (lets-plot `TimeInterval`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeInterval {
    /// A fixed length in milliseconds, aligned on `unit` (hours and longer
    /// align on the hour).
    Fixed {
        /// The unit in milliseconds.
        unit: f64,
        /// Units per interval.
        count: u32,
    },
    /// Days from midnight.
    Days(u32),
    /// Weeks from Monday.
    Weeks(u32),
    /// Months from the first of a month.
    Months(u32),
    /// Years from January 1.
    Years(u32),
}

/// Most breaks a `break_width` may give before it is widened.
const MAX_FIXED_BREAKS: f64 = 50.0;

impl TimeInterval {
    /// This interval, its count multiplied so that `span` holds at most `limit` of it.
    fn at_most(self, limit: f64, span: f64) -> TimeInterval {
        let ratio = span / self.nominal_milliseconds() / limit;
        if ratio.is_nan() || ratio <= 1.0 {
            return self;
        }
        let factor = ratio.ceil().min(f64::from(u32::MAX)) as u32;
        let scale = |count: u32| count.saturating_mul(factor);
        match self {
            TimeInterval::Fixed { unit, count } => TimeInterval::Fixed {
                unit,
                count: scale(count),
            },
            TimeInterval::Days(count) => TimeInterval::Days(scale(count)),
            TimeInterval::Weeks(count) => TimeInterval::Weeks(scale(count)),
            TimeInterval::Months(count) => TimeInterval::Months(scale(count)),
            TimeInterval::Years(count) => TimeInterval::Years(scale(count)),
        }
    }

    /// lets-plot `TimeInterval.parse`: `"<count> <unit>"`, e.g. `"2 weeks"`.
    pub fn parse(spec: &str) -> Result<TimeInterval, String> {
        let accepted = "'<count> <unit>' with a unit of ms, sec, min, hour, day, week, month \
                        or year (plural allowed), e.g. '2 weeks'";
        let mut parts = spec.split_whitespace();
        let (Some(count), Some(unit), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(format!(
                "invalid time interval '{spec}'; expected {accepted}"
            ));
        };
        let count: u32 = match count.parse() {
            Ok(count) if count > 0 => count,
            _ => {
                return Err(format!(
                    "invalid count '{count}' in time interval '{spec}'; expected a positive \
                     whole number"
                ));
            }
        };
        let fixed = |unit: f64| TimeInterval::Fixed { unit, count };
        Ok(match unit.to_lowercase().as_str() {
            "ms" | "millis" | "millisecond" | "milliseconds" => fixed(1.0),
            "sec" | "second" | "seconds" => fixed(SECOND),
            "min" | "minute" | "minutes" => fixed(MINUTE),
            "hour" | "hours" => fixed(HOUR),
            "day" | "days" => TimeInterval::Days(count),
            "week" | "weeks" => TimeInterval::Weeks(count),
            "month" | "months" => TimeInterval::Months(count),
            "year" | "years" => TimeInterval::Years(count),
            other => {
                return Err(format!(
                    "unknown time unit '{other}' in '{spec}'; expected {accepted}{}",
                    crate::suggest::did_you_mean(
                        other,
                        ["ms", "sec", "min", "hour", "day", "week", "month", "year"]
                    )
                ));
            }
        })
    }

    /// Length in milliseconds (months as 30 days, years as 365), for duration axes.
    fn nominal_milliseconds(&self) -> f64 {
        match *self {
            TimeInterval::Fixed { unit, count } => unit * f64::from(count),
            TimeInterval::Days(count) => DAY * f64::from(count),
            TimeInterval::Weeks(count) => WEEK * f64::from(count),
            TimeInterval::Months(count) => 30.0 * DAY * f64::from(count),
            TimeInterval::Years(count) => YEAR * f64::from(count),
        }
    }

    /// The label pattern of breaks this far apart (lets-plot `tickFormatPattern`).
    fn pattern(&self) -> &'static str {
        match *self {
            TimeInterval::Fixed { unit, count } => {
                let length = unit * f64::from(count);
                if length < MINUTE {
                    "%M:%S"
                } else if length < DAY {
                    "%H:%M"
                } else {
                    "%b %e"
                }
            }
            TimeInterval::Days(_) | TimeInterval::Weeks(_) => "%b %e",
            TimeInterval::Months(_) => "%b",
            TimeInterval::Years(_) => "%Y",
        }
    }

    /// Every interval start in `[start, end]` (lets-plot `range`).
    fn range(&self, start: f64, end: f64) -> Vec<f64> {
        let mut ticks = Vec::new();
        if let TimeInterval::Fixed { unit, count } = *self {
            let step = unit * f64::from(count);
            let atomic = if unit < HOUR { step } else { unit };
            let mut tick = (start / atomic).ceil() * atomic;
            while tick <= end {
                ticks.push(tick);
                tick += step;
            }
            return ticks;
        }
        // Calendar intervals: the interval start at or before `start`, stepped.
        let civil = Civil::of(start);
        let (mut year, mut month, mut day) = match *self {
            TimeInterval::Days(_) => (civil.year, civil.month, civil.day),
            TimeInterval::Weeks(_) => {
                let monday =
                    days_from_civil(civil.year, civil.month, civil.day) - i64::from(civil.weekday);
                civil_from_days(monday)
            }
            TimeInterval::Months(_) => (civil.year, civil.month, 1),
            TimeInterval::Years(_) => (civil.year, 1, 1),
            TimeInterval::Fixed { .. } => unreachable!("handled above"),
        };
        let next = |year: &mut i64, month: &mut u32, day: &mut u32| match *self {
            TimeInterval::Days(count) | TimeInterval::Weeks(count) => {
                let step = if matches!(self, TimeInterval::Days(_)) {
                    1
                } else {
                    7
                };
                let days = days_from_civil(*year, *month, *day) + i64::from(count) * step;
                (*year, *month, *day) = civil_from_days(days);
            }
            TimeInterval::Months(count) => {
                let index = i64::from(*month - 1) + i64::from(count);
                *year += index.div_euclid(12);
                *month = index.rem_euclid(12) as u32 + 1;
            }
            TimeInterval::Years(count) => *year += i64::from(count),
            TimeInterval::Fixed { .. } => {}
        };
        let mut tick = midnight(year, month, day);
        if tick < start {
            next(&mut year, &mut month, &mut day);
            tick = midnight(year, month, day);
        }
        while tick <= end && ticks.len() < 100_000 {
            ticks.push(tick);
            next(&mut year, &mut month, &mut day);
            tick = midnight(year, month, day);
        }
        ticks
    }
}

/// lets-plot `NiceTimeInterval`: the automatic steps, in order.
const NICE_INTERVALS: [TimeInterval; 18] = [
    TimeInterval::Fixed {
        unit: SECOND,
        count: 1,
    },
    TimeInterval::Fixed {
        unit: SECOND,
        count: 5,
    },
    TimeInterval::Fixed {
        unit: SECOND,
        count: 15,
    },
    TimeInterval::Fixed {
        unit: SECOND,
        count: 30,
    },
    TimeInterval::Fixed {
        unit: MINUTE,
        count: 1,
    },
    TimeInterval::Fixed {
        unit: MINUTE,
        count: 5,
    },
    TimeInterval::Fixed {
        unit: MINUTE,
        count: 15,
    },
    TimeInterval::Fixed {
        unit: MINUTE,
        count: 30,
    },
    TimeInterval::Fixed {
        unit: HOUR,
        count: 1,
    },
    TimeInterval::Fixed {
        unit: HOUR,
        count: 3,
    },
    TimeInterval::Fixed {
        unit: HOUR,
        count: 6,
    },
    TimeInterval::Fixed {
        unit: HOUR,
        count: 12,
    },
    TimeInterval::Days(1),
    TimeInterval::Days(2),
    TimeInterval::Weeks(1),
    TimeInterval::Months(1),
    TimeInterval::Months(3),
    TimeInterval::Years(1),
];

/// Lengths of [`NICE_INTERVALS`] (months as 30 days).
const NICE_STEPS: [f64; 18] = [
    SECOND,
    5.0 * SECOND,
    15.0 * SECOND,
    30.0 * SECOND,
    MINUTE,
    5.0 * MINUTE,
    15.0 * MINUTE,
    30.0 * MINUTE,
    HOUR,
    3.0 * HOUR,
    6.0 * HOUR,
    12.0 * HOUR,
    DAY,
    2.0 * DAY,
    WEEK,
    30.0 * DAY,
    90.0 * DAY,
    YEAR,
];

/// Index of `ONE_DAY` and `TWELVE_HOURS` in [`NICE_INTERVALS`].
const ONE_DAY: usize = 12;
const TWELVE_HOURS: usize = 11;

/// lets-plot `NiceTimeInterval.forMillisIntern`: the nearest nice step (ties up).
fn nice_interval(step: f64) -> usize {
    if step <= NICE_STEPS[0] {
        return 0;
    }
    for index in 1..NICE_STEPS.len() {
        if NICE_STEPS[index] >= step {
            let down = step - NICE_STEPS[index - 1];
            let up = NICE_STEPS[index] - step;
            return if down < up { index - 1 } else { index };
        }
    }
    NICE_STEPS.len() - 1
}

/// lets-plot `DateTimeBreaksHelper`: breaks and their label pattern.
fn datetime_breaks(
    low: f64,
    high: f64,
    count: usize,
    kind: TemporalKind,
) -> (Vec<f64>, &'static str) {
    let (minimum, maximum) = match kind {
        TemporalKind::Date => (Some(ONE_DAY), None),
        TemporalKind::TimeOfDay => (None, Some(TWELVE_HOURS)),
        _ => (None, None),
    };
    let step = (high - low) / count as f64;
    if step < SECOND {
        // lets-plot: the millisecond interval's pattern, or the minimum's.
        let pattern = minimum.map_or("%M:%S", |index| NICE_INTERVALS[index].pattern());
        return (linear_breaks(low, high, count), pattern);
    }
    if let Some(index) = minimum {
        let ticks = NICE_INTERVALS[index].range(low, high);
        if ticks.len() <= count {
            return (ticks, NICE_INTERVALS[index].pattern());
        }
    }
    if step > YEAR {
        let start = Civil::of(low);
        let mut first_year = start.year;
        if low > midnight(first_year, 1, 1) {
            first_year += 1;
        }
        let last_year = Civil::of(high).year;
        let years = linear_breaks(first_year as f64, last_year as f64, count);
        let ticks = years
            .iter()
            .map(|&year| midnight(year.round() as i64, 1, 1))
            .collect();
        let pattern = maximum.map_or("%Y", |index| NICE_INTERVALS[index].pattern());
        return (ticks, pattern);
    }
    let mut index = nice_interval(step);
    if let Some(minimum) = minimum {
        index = index.max(minimum);
    }
    if let Some(maximum) = maximum {
        index = index.min(maximum);
    }
    let interval = NICE_INTERVALS[index];
    (interval.range(low, high), interval.pattern())
}

/// lets-plot `TimeBreaksGen`: linear breaks under a second per break, else the
/// nice duration nearest to the step. Returns the breaks and the step.
fn duration_breaks(low: f64, high: f64, count: usize) -> (Vec<f64>, f64) {
    let step = (high - low) / count as f64;
    if step < SECOND {
        return (linear_breaks(low, high, count), step);
    }
    const NICE: [f64; 18] = [
        SECOND,
        5.0 * SECOND,
        15.0 * SECOND,
        30.0 * SECOND,
        MINUTE,
        5.0 * MINUTE,
        15.0 * MINUTE,
        30.0 * MINUTE,
        HOUR,
        3.0 * HOUR,
        6.0 * HOUR,
        12.0 * HOUR,
        DAY,
        2.0 * DAY,
        WEEK,
        4.0 * WEEK,
        12.0 * WEEK,
        48.0 * WEEK,
    ];
    // Kotlin `minByOrNull { abs(it - step.toLong()) }`: the first of equals.
    let target = step.trunc();
    let nice = NICE
        .iter()
        .copied()
        .fold((SECOND, f64::INFINITY), |(best, distance), candidate| {
            let candidate_distance = (candidate - target).abs();
            if candidate_distance < distance {
                (candidate, candidate_distance)
            } else {
                (best, distance)
            }
        })
        .0;
    (multiples(low, high, nice), step)
}

/// Multiples of `step` in `[low, high]` (lets-plot `LinearBreaksHelper.generateBreaks`);
/// exact for whole-millisecond steps, unlike the rounding `breaks_with_step`.
fn multiples(low: f64, high: f64, step: f64) -> Vec<f64> {
    let tolerance = step / 10_000.0;
    let first = ((low - tolerance) / step).ceil();
    let last = ((high + tolerance) / step).floor();
    if !(step > 0.0 && first.is_finite() && last.is_finite()) || last - first > 100_000.0 {
        return Vec::new();
    }
    (0..=(last - first) as i64)
        .map(|index| ((first + index as f64) * step).min(high))
        .collect()
}

/// lets-plot `DurationFormatter.formatBreaks`.
fn format_durations(breaks: &[f64], width: f64, span: f64) -> Vec<String> {
    if width < SECOND {
        return breaks
            .iter()
            .map(|&value| format_duration(value, false, 0.0))
            .collect();
    }
    let hide_seconds = breaks
        .iter()
        .all(|&value| value >= DAY || ((value as i64).abs() / 1000) % 60 == 0);
    breaks
        .iter()
        .map(|&value| format_duration(value, hide_seconds, span))
        .collect()
}

/// lets-plot `DurationFormatter.formatDuration`: `2d 3:04:05.6`, `0:30`, `-1:00`.
fn format_duration(value: f64, hide_seconds: bool, span: f64) -> String {
    let total = value as i64;
    if total == 0 {
        return "0".to_owned();
    }
    let absolute = total.unsigned_abs();
    let days = absolute / 86_400_000;
    let hours = absolute / 3_600_000 % 24;
    let minutes = absolute / 60_000 % 60;
    let seconds = absolute / 1000 % 60;
    let millis = absolute % 1000;
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    let mut time = String::new();
    if hide_seconds {
        if hours > 0 || minutes > 0 {
            time = format!("{hours}:{minutes:02}");
        }
    } else {
        if hours > 0 || minutes > 0 || seconds > 0 {
            time = format!("{hours}:{minutes:02}:{seconds:02}");
        }
        if millis > 0 {
            if span > SECOND && time.is_empty() {
                time = format!("{hours}:{minutes:02}:{seconds:02}");
            }
            if !time.is_empty() {
                time.push('.');
            }
            if millis % 100 == 0 {
                time.push_str(&(millis / 100).to_string());
            } else {
                time.push_str(&millis.to_string());
            }
        }
    }
    if !time.trim().is_empty() {
        parts.push(time);
    }
    let sign = if total < 0 { "-" } else { "" };
    format!("{sign}{}", parts.join(" "))
}

/// A date-time label pattern (lets-plot `DateTimeFormat`): text with `%`
/// directives; inside `{...}` placeholders when the pattern has braces.
#[derive(Debug, Clone, PartialEq)]
pub struct DatePattern {
    parts: Vec<PatternPart>,
}

#[derive(Debug, Clone, PartialEq)]
enum PatternPart {
    Text(String),
    Directive(char),
}

const DIRECTIVES: &str = "aAbBdejmwyYHIlMpPSf";

impl DatePattern {
    /// Parse a pattern; `{%d.%m}` placeholders are replaced, other text kept.
    pub fn parse(pattern: &str) -> DatePattern {
        // lets-plot `StringFormat`: with braces only the placeholders are formats.
        let inner: String = if pattern.contains('{') {
            pattern.replace(['{', '}'], "")
        } else {
            pattern.to_owned()
        };
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut characters = inner.chars().peekable();
        while let Some(character) = characters.next() {
            match (character, characters.peek()) {
                ('%', Some(&next)) if DIRECTIVES.contains(next) => {
                    characters.next();
                    if !text.is_empty() {
                        parts.push(PatternPart::Text(std::mem::take(&mut text)));
                    }
                    parts.push(PatternPart::Directive(next));
                }
                _ => text.push(character),
            }
        }
        if !text.is_empty() {
            parts.push(PatternPart::Text(text));
        }
        DatePattern { parts }
    }

    /// Whether the pattern has a date-time directive (lets-plot `isDateTimeFormat`).
    pub fn is_datetime(pattern: &str) -> bool {
        let mut characters = pattern.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '%'
                && characters
                    .peek()
                    .is_some_and(|next| DIRECTIVES.contains(*next))
            {
                return true;
            }
        }
        false
    }

    /// Format milliseconds since the epoch (UTC).
    pub fn format(&self, value: f64) -> String {
        if !value.is_finite() {
            return String::new();
        }
        let civil = Civil::of(value);
        let mut out = String::new();
        for part in &self.parts {
            match part {
                PatternPart::Text(text) => out.push_str(text),
                PatternPart::Directive(directive) => civil.write(*directive, &mut out),
            }
        }
        out
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
/// From Monday.
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// A UTC calendar date and time.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Civil {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: u32,
    /// 0 = Monday.
    weekday: u32,
    /// 1 = January 1.
    day_of_year: u32,
}

impl Civil {
    fn of(milliseconds: f64) -> Civil {
        let total = milliseconds.floor() as i64;
        let days = total.div_euclid(86_400_000);
        let within = total.rem_euclid(86_400_000);
        let (year, month, day) = civil_from_days(days);
        Civil {
            year,
            month,
            day,
            hour: (within / 3_600_000) as u32,
            minute: (within / 60_000 % 60) as u32,
            second: (within / 1000 % 60) as u32,
            millisecond: (within % 1000) as u32,
            // 1970-01-01 was a Thursday.
            weekday: (days + 3).rem_euclid(7) as u32,
            day_of_year: (days - days_from_civil(year, 1, 1) + 1) as u32,
        }
    }

    /// lets-plot `DateTimeFormat.getValueForPattern`.
    fn write(&self, directive: char, out: &mut String) {
        use std::fmt::Write;
        let hour12 = match self.hour {
            0 => 12,
            hour if hour <= 12 => hour,
            hour => hour - 12,
        };
        let month_name = MONTHS[(self.month - 1) as usize];
        let weekday_name = WEEKDAYS[self.weekday as usize];
        let _ = match directive {
            'Y' => write!(out, "{}", self.year),
            'y' => write!(
                out,
                "{}",
                self.year.to_string().get(2..).unwrap_or_default()
            ),
            'm' => write!(out, "{:02}", self.month),
            'b' => write!(out, "{}", &month_name[..3]),
            'B' => write!(out, "{month_name}"),
            'd' => write!(out, "{:02}", self.day),
            'e' => write!(out, "{}", self.day),
            'j' => write!(out, "{:03}", self.day_of_year),
            'a' => write!(out, "{}", &weekday_name[..3]),
            'A' => write!(out, "{weekday_name}"),
            // Sunday = 0.
            'w' => write!(out, "{}", (self.weekday + 1) % 7),
            'H' => write!(out, "{:02}", self.hour),
            'I' => write!(out, "{hour12:02}"),
            'l' => write!(out, "{hour12}"),
            'M' => write!(out, "{:02}", self.minute),
            'S' => write!(out, "{:02}", self.second),
            'f' => write!(out, "{:03}", self.millisecond),
            'P' => write!(out, "{}", if self.hour < 12 { "am" } else { "pm" }),
            'p' => write!(out, "{}", if self.hour < 12 { "AM" } else { "PM" }),
            _ => Ok(()),
        };
    }
}

/// Milliseconds at UTC midnight of a date.
fn midnight(year: i64, month: u32, day: u32) -> f64 {
    days_from_civil(year, month, day) as f64 * DAY
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date of a day number (inverse of [`days_from_civil`]).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i64, month: u32, day: u32) -> f64 {
        midnight(year, month, day)
    }

    #[test]
    fn calendar_round_trips() {
        for days in [-800_000, -1, 0, 1, 11_016, 19_000, 2_000_000] {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(days_from_civil(year, month, day), days);
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    }

    #[test]
    fn formats_like_lets_plot() {
        // Tue 6 Aug 2019 04:46:35.123 UTC.
        let value = at(2019, 8, 6) + 4.0 * HOUR + 46.0 * MINUTE + 35.0 * SECOND + 123.0;
        let format = |pattern: &str| DatePattern::parse(pattern).format(value);
        assert_eq!(format("%d.%m.%y"), "06.08.19");
        assert_eq!(format("%B %Y"), "August 2019");
        assert_eq!(format("%a, %e %b %Y %H:%M:%S"), "Tue, 6 Aug 2019 04:46:35");
        assert_eq!(format("%H:%M:%S.%f"), "04:46:35.123");
        assert_eq!(format("%j %w %I %l %p"), "218 2 04 4 AM");
        assert_eq!(format("Week of {%b %e}"), "Week of Aug 6");
        assert!(DatePattern::is_datetime("%Y") && !DatePattern::is_datetime("{.2f}"));
    }

    #[test]
    fn calendar_intervals_start_on_their_boundary() {
        let months = TimeInterval::Months(1).range(at(2021, 1, 15), at(2021, 4, 1));
        assert_eq!(months, vec![at(2021, 2, 1), at(2021, 3, 1), at(2021, 4, 1)]);
        // 2021-01-06 is a Wednesday: weeks start on Monday the 11th.
        let weeks = TimeInterval::Weeks(1).range(at(2021, 1, 6), at(2021, 1, 20));
        assert_eq!(weeks, vec![at(2021, 1, 11), at(2021, 1, 18)]);
        let hours = TimeInterval::Fixed {
            unit: HOUR,
            count: 6,
        }
        .range(HOUR, DAY);
        assert_eq!(hours, vec![HOUR, 7.0 * HOUR, 13.0 * HOUR, 19.0 * HOUR]);
        assert_eq!(TimeInterval::parse("2 weeks"), Ok(TimeInterval::Weeks(2)));
        assert!(
            TimeInterval::parse("2 fortnights")
                .unwrap_err()
                .contains("unknown time unit")
        );
    }

    #[test]
    fn automatic_breaks_pick_a_nice_interval() {
        let (breaks, pattern) =
            datetime_breaks(at(2021, 1, 1), at(2021, 12, 31), 5, TemporalKind::DateTime);
        assert_eq!(pattern, "%b");
        assert_eq!(breaks.first(), Some(&at(2021, 1, 1)));
        let (years, pattern) =
            datetime_breaks(at(2000, 6, 1), at(2020, 6, 1), 5, TemporalKind::DateTime);
        assert_eq!(pattern, "%Y");
        assert_eq!(years.first(), Some(&at(2005, 1, 1)));
        // Dates never break below a day.
        let (_, pattern) = datetime_breaks(at(2021, 1, 1), at(2021, 1, 2), 5, TemporalKind::Date);
        assert_eq!(pattern, "%b %e");
    }

    #[test]
    fn durations_format_like_lets_plot() {
        assert_eq!(format_duration(0.0, false, 0.0), "0");
        assert_eq!(format_duration(90.0 * MINUTE, true, 0.0), "1:30");
        assert_eq!(
            format_duration(DAY + 2.0 * HOUR + 5.5 * SECOND, false, 0.0),
            "1d 2:00:05.5"
        );
        assert_eq!(format_duration(-HOUR, false, 0.0), "-1:00:00");
        let (breaks, _) = duration_breaks(0.0, 6.0 * HOUR, 5);
        assert_eq!(breaks[1], HOUR);
    }
}
