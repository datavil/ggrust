# Errors

Every error ggrust raises is a `GgrustError` and the builtin that matches the problem,
so `except ValueError` (as with lets_plot) keeps working. Errors from the engine carry
where they happened: `SpecError.path` is the spec path of the bad value
(`"layers[0].size"`, `"figures[1].theme.legend_position"`), `ColumnNotFoundError`
lists the available columns and suggests a close name.

```python
from ggrust.errors import ColumnNotFoundError

try:
    plot.to_svg()
except ColumnNotFoundError as error:
    print(error.column, error.path, error.available)
```

::: ggrust.errors.GgrustError

::: ggrust.errors.SpecError

::: ggrust.errors.UnsupportedValueError

::: ggrust.errors.DataError

::: ggrust.errors.ColumnNotFoundError

::: ggrust.errors.UnsupportedColumnTypeError

::: ggrust.errors.UnsupportedDataError

::: ggrust.errors.RenderError

::: ggrust.errors.ExportError
