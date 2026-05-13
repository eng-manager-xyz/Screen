# Theme + palette

The visual configuration applied at render time. `Theme::light()`
ships v1; dark + custom themes follow.

## Defaults

```rust,ignore
Theme {
    bg: #ffffff,
    row_alt_bg: Some(#fafafa),
    grid_week: { color: #e5e5e5, width: 1.0 },
    grid_month: { color: #cccccc, width: 2.0 },
    header_bg: #f5f5f5,
    text_primary: #222222,
    text_muted: #888888,
    bar_corner_radius: 6.0,
    bar_height: 28.0,
    row_height: 44.0,
    gutter_width: 180.0,
    header_height: 60.0,
    palette: OwnerPalette::Wong,
}
```

## Wong palette

The default owner-colour palette is **Wong's
colourblind-friendly 8-colour set**:

| # | Colour | Hex |
|---|---|---|
| 0 | Blue | `#0072b2` |
| 1 | Vermillion | `#d55e00` |
| 2 | Bluish green | `#009e73` |
| 3 | Reddish purple | `#cc79a7` |
| 4 | Yellow | `#f0e442` |
| 5 | Sky blue | `#56b4e9` |
| 6 | Orange | `#e69f00` |
| 7 | Black | `#000000` |

Auto-assignment hashes the owner's name (FNV-1a 64-bit, then
modulo). The hash is stable across native + wasm32, so the
same fixture renders the same colours in every build.

## Contrast-aware bar text

`Color::luminance` implements the WCAG 2.x relative-luminance
formula (sRGB → linear → weighted). `contrast_text_color(bg)`
picks black if `bg.luminance() > 0.179` else white. The bar's
owner name uses this against the bar's fill.

## OwnerPalette variants

```rust,ignore
pub enum OwnerPalette {
    Wong,                          // default
    Custom(Vec<Color>),            // hash against your own list
    AutoWithOverrides(Vec<Color>), // explicit PersonMap wins; rest hash
}
```

```admonish tip title="Override individual owners, keep the default for the rest"
The most common case — most owners get auto-assigned, a few
get explicit brand colours — uses
`AutoWithOverrides(WONG-decoded)` for the fallback palette and
fills `PersonMap` with the explicit entries.
```
