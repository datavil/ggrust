# Lists in a mapping are data, not column names (except `group`)

lets-plot's backend runs `ReplaceDataVectorsInAesMappingChange` before building: every
mapping value that is a list (any aesthetic but `group`) becomes a data column of that
spec (plot or layer), named after the aesthetic, else `x1`, `x2`, ... when ANY data
column of the plot (used or not) or an earlier vector has the name. The generated name
is the default axis/legend title: `geom_function(aes(x=[...]))` titles its axis `x1`,
because the builder's own data has an `x` column.

ggrust: `data::inline` + `build::inline_mapping_vectors` (pre-pass of `build_plot`).
Python exports only referenced columns, so frame references carry every column name:
`{"frame": i, "columns": [...]}` (`_render._column_names`). The engine avoids those
names. Exporting candidate columns instead (`x`, `x1`..) broke on unreadable unused
columns (pandas object dtype) and capped the count: send names, not data.

**Why:** before this, ggrust read a string list on any aesthetic as column names and
warned on number lists, so `geom_function(aes(x=[...]))` drew nothing.
**How to apply:** never treat `MappingValue::Columns` as column names outside `group`;
use `inline::is_vector`. Length mismatch: lets-plot draws nothing, ggrust errors.
Mixed-type lists (`[1, "a"]`) error in ggrust; lets-plot draws them (not yet).
