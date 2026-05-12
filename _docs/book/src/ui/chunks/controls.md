# Controls

[Linear: AUT-124](https://linear.app/harwood/issue/AUT-124)

Seven new control primitives that expand the `Button` vocabulary into
everything the recorder + editor + cursor studio need. All stateless;
all driven by props.

## Icon buttons

<iframe src="../../assets/ui/controls-buttons-icon-buttons.html" width="400" height="80" frameborder="0"></iframe>

`IconButtonVariant { Ghost, Filled, Danger }`. Supports `pressed` for
toggle-style icon buttons and `disabled`. Accessible label is
required since the button has no visible text.

## Toggle switch

<iframe src="../../assets/ui/controls-toggle-states.html" width="400" height="120" frameborder="0"></iframe>

```admonish important title="Toggles don't flip themselves"
`checked` is a prop. The parent owns the boolean and re-renders with
the new value when the callback fires. UI-23's grep guardrail
catches any `RwSignal::new` / `set_checked.set(…)` inside the
component module.
```

## Segmented control

<iframe src="../../assets/ui/controls-segmented-record-mode.html" width="400" height="160" frameborder="0"></iframe>

`Vec<Segment>` + `active: String` (the segment id). Each segment can
have an optional leading icon glyph + `disabled` flag.

## Slider

<iframe src="../../assets/ui/controls-slider-values.html" width="420" height="180" frameborder="0"></iframe>

`Slider { value, min, max, disabled, label, readout }`. Pure visual —
renders track + fill + thumb at the computed percent. `slider_percent`
helper is `pub` and unit-tested for clamping behavior.

## Color swatches

<iframe src="../../assets/ui/controls-color-swatches.html" width="280" height="60" frameborder="0"></iframe>

Circular tiles with an outer ring when `selected`. Used in the cursor
studio color picker.

## Meters

<iframe src="../../assets/ui/controls-meters.html" width="320" height="220" frameborder="0"></iframe>

Audio-level bars driven by a normalized `[0, 1]` level. `bar_count`
defaults to 12; `danger=true` switches the lit color from emerald to
the action-record red (used for clipping). `lit_segments` is `pub` +
unit-tested.

## Select pill

Bundled in the meters demo above. Compact pill that opens a popover
when clicked. The component renders the pill chrome only; the popover
content is the parent's job (typically a `PopoverSurface` from UI-03).
