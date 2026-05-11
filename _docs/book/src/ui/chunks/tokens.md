# Design tokens

[Linear: AUT-121](https://linear.app/harwood/issue/AUT-121)

Semantic CSS variables that every component reaches for instead of
the raw hex literals. Adding a new product surface means picking from
this list — not introducing a new `--bg-purple-deep` for one
component. The raw zinc palette stays in `style.css` :root as
implementation detail; semantic aliases are the public API.

<iframe src="../../assets/ui/tokens-dark-zinc.html" width="100%" height="320" frameborder="0"></iframe>

[Open as live demo →](../../assets/ui/tokens-dark-zinc.html)

## Token table

| Token | Role |
| --- | --- |
| `--surface-base` | App background — bottom of the stack |
| `--surface-elevated` | Panels + cards |
| `--surface-popover` | Tray menus, dropdowns, command menus |
| `--surface-selected` | Highlighted list row inside a menu |
| `--surface-glass` | Translucent overlay over the recording |
| `--text-primary` | Default text colour |
| `--text-secondary` | Muted labels |
| `--text-tertiary` | Axis labels, footnotes |
| `--line-subtle` | Default 1px borders |
| `--line-strong` | Stronger 1px borders for focus / selected |
| `--action-record` | Record / destructive action |
| `--action-record-hover` | Record hover state |
| `--shadow-popover` | Popover drop shadow |
| `--shadow-elevated` | Card / panel drop shadow |
| `--radius-panel` | Cards, popovers, tray surfaces |
| `--radius-control` | Buttons, inputs |
| `--radius-pill` | Pill / chip badges |
| `--focus-ring` | Focus outline (`box-shadow`) |

```admonish important title="No raw hex in component CSS"
Component classes (`.btn-default`, `.surface-popover`, `.badge-live`,
…) must reference these tokens. The only place a raw hex literal is
allowed is the `:root` definition in `style.css` itself OR a token
demo (the swatches above use inline `style=` because the tokens are
the content). UI-23's grep guardrail will flag stray hex outside
those locations.
```

## Adding a new token

1. Add the variable to `:root` in `style.css` with a comment naming
   the surfaces that need it.
2. Add a row to the table above.
3. Use it from the component CSS — never reach for a raw zinc value.
4. If the new token can be derived from an existing one (alpha
   variant, hover state), prefer `color-mix(...)` in CSS over a new
   hex literal.
