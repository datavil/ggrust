# Never cast strings to `Utf8`; read them in place

**Broke:** casting polars `Utf8View` columns to `Utf8` (i32 offsets) panicked inside
arrow (`byte array offset overflow`) once total string bytes exceeded 2 GiB. Views share
buffers, so a frame that is a few MB in memory can be > 2 GiB when cast. It was also
slower (a full copy) and wrong for booleans (built "true"/"false" strings per row).

**Works:** match on `Utf8` / `LargeUtf8` / `Utf8View` and iterate each in place;
booleans get their own path. Casting is fine only for small arrays (dictionary values).

**Also:** never `concat` dictionary chunks: different dictionaries can overflow the key
type (pandas int8 keys). Convert each chunk, then merge levels (`DiscreteColumn::merge`).
Result (5M rows): strings 144 -> 91 ms, booleans 200 -> 14 ms.
