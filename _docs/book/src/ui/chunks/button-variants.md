# Button — variants

<iframe src="../../assets/ui/button-variants.html" width="100%" height="120" frameborder="0"></iframe>

Five variants in a single row: `Default`, `Outline`, `Ghost`, `Destructive`,
`Secondary`. Same shape, same height, different role.

The variant decides only the surface treatment (`bg-*` / `border-*`) — the
typography, padding, and corner radius come from `btn` + the size class.
That separation is why we can ship a new variant by adding one CSS rule
and one enum case.

Class hooks mirror rust-ui's, so swapping in Tailwind later is a search-
and-replace, not a rewrite.

[Open as standalone demo →](../../assets/ui/button-variants.html)

---

[`Button` API](../../api/ui_storybook/components/button/fn.Button.html) · [Components index](../components.md)
