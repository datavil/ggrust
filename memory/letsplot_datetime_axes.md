# Date-time axes: port lets-plot's breaks, compare labels under TZ=UTC

**Works:** `scale/datetime.rs` ports `DateTimeBreaksHelper` (nice intervals 1 s ... 1 y,
nearest by length; < 1 s linear; > 1 y linear whole years), the calendar intervals
(weeks start Monday, months on the 1st), `TimeBreaksGen` + `DurationFormatter` and the
`DateTimeFormat` subset. Values are epoch milliseconds (Arrow temporal columns are
converted in `data/column.rs`). Axis labels match lets-plot in every tested case.

**Traps:**
- lets-plot reads naive Python datetimes in the local time zone. Compare with
  `TZ=UTC` (tests set it with `monkeypatch.setenv` + `time.tzset()`).
- Do not use `breaks_with_step` for time steps: it rounds to the step's magnitude, which
  is fine for 1/2/5 steps but turns 2 days into 1.7e8 ms. Use exact multiples.
- `break_width` makes the breaks fixed: lay out the labels like fixed breaks
  (`has_fixed_breaks`), or they overlap where lets-plot turns them vertical.
- A tiny `break_width` over a long span ("1 sec" over months) made millions of breaks
  and hung. Widths are widened to a whole multiple giving at most 50 breaks
  (`TimeInterval::at_most`); lets-plot thins them at layout instead.
- Make date breaks in data units and transform them after (`data_domain`), or
  `scale_x_reverse()` / log axes format negated or log values as dates.
- The HTML zoom runtime generates number breaks only: temporal axes ship their drawn
  breaks as fixed, or zoomed axes show epoch numbers.
- Automatic datetime axes come from series annotations (`type` datetime/date/time),
  not from the Arrow type; dates never break below a day, times of day never above 12 h.

**Deviations / gaps (2026-11-01):** labels are UTC only (zoned columns warn); fixed
`breaks=[...]` get the axis pattern (lets-plot writes ISO strings); tooltips show raw
numbers.
