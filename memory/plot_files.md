# Plot files (.ggrs)

`python/ggrust/plot_file.py`. Decided with the user on 2026-11-05: a safe format instead of pickle, `.ggrs` only,
callables refuse to save, fonts embedded.

- **Store the object tree, not `as_dict()`.** `as_dict()` cannot be turned back into a
  `PlotSpec` (`+` rewrites: scale groups, gghighlight, ggside merges). The encoder walks
  `vars()` of allowlisted classes (FeatureSpec subclasses, `SupPlotsLayoutSpec`,
  `MappingMeta`, `ClusterTree`) and the decoder sets the attributes with
  `object.__new__` + `object.__setattr__`, without calling `__init__`. Name-mangled attributes
  (`_PlotSpec__layers`) are stored as they are. **A new private attribute on a plot class
  changes the file format:** an old file loads without it. Give it a default with
  `getattr`, or bump `FORMAT_VERSION` and migrate.
- **Allowlist by `module:qualname` from `FeatureSpec.__subclasses__()`**, never
  `importlib` from file text. A class that is not imported yet is not allowed. That is fine,
  because `import ggrust` imports every plot module.
- **Tags start with `$`.** A user dict with a `$` key or a non-string key is stored as
  `$dict` pairs, so a dict is never read as a tag.
- **Frames are stored once by `id()`.** A frame shared by a grid comes back shared. pandas and
  pyarrow are written with pyarrow IPC and read back with `pl.read_ipc().to_arrow()` (the
  polars reader validates the bytes). Then `to_pandas()` gives back the original frame kind.
- **Hashes cover every entry except the manifest.** They catch damage, not edits.
  Signing is in plan.md "Consider later".
- **Fonts:** every string under a `family` key is looked up in the registry
  (`registered_font_files`). Fonts whose license forbids embedding are skipped with a warning.
- **Pickle is left as default Python pickling** (it already works and can carry
  callables). We deliberately chose not to route it through `.ggrs`, because
  `copy`/`deepcopy` also use `__reduce_ex__`.
