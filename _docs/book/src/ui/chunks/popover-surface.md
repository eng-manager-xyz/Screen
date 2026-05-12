# Popover surface

[Linear: AUT-123](https://linear.app/harwood/issue/AUT-123)

Chrome shared by every tray menu / dropdown / on-screen-options
popover. Owns the corner radius, drop shadow, header / body / footer
slots, and a `placement` class the parent overlay layer uses to
position the surface. The component itself doesn't compute
coordinates — that's the parent's job.

<iframe src="../../assets/ui/popover-basic.html" width="320" height="300" frameborder="0"></iframe>

[Open as live demo →](../../assets/ui/popover-basic.html)

## With a footer

<iframe src="../../assets/ui/popover-with-footer.html" width="320" height="340" frameborder="0"></iframe>

```admonish important title="No positioning math"
`PopoverPlacement` only emits a `popover-<placement>` CSS class. The
parent (overlay layer in `app-ui`) decides anchor coordinates and
flip behavior. Keeping placement math out of the surface means
storybook stories render deterministically without a viewport.
```

## API

```rust
use ui_storybook::components::{
    PopoverSurface, PopoverPlacement, MenuList, MenuRow,
};

view! {
    <PopoverSurface
        placement=PopoverPlacement::BottomLeft
        width_px=300_u16
        title="Choose camera"
        footer=ToChildren::to_children(|| view! { /* primary action */ })
    >
        <MenuList label="Cameras">
            /* … rows */
        </MenuList>
    </PopoverSurface>
}
```

## Placement classes

| `PopoverPlacement` | CSS class |
| --- | --- |
| `TopLeft` | `popover-tl` |
| `TopRight` | `popover-tr` |
| `BottomLeft` (default) | `popover-bl` |
| `BottomRight` | `popover-br` |
| `Centered` | `popover-center` |
