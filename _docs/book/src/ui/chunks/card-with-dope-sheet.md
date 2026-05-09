# Editor panel — card wrapping dope sheet

<iframe src="../../assets/ui/card-with-dope-sheet.html" width="100%" height="320" frameborder="0"></iframe>

Composition: a `Card` with `CardHeader` ("Timeline · 4 tracks · 8.0s") and a
`CardBody` containing the `DopeSheet`. This is the expected production
placement in the editor — title and metadata up top, the timeline grid
underneath.

It's also the canary that catches "two correct components compose
incorrectly" bugs: padding, scroll behavior, the playhead's vertical extent
all have to coexist with the card's overflow and corner radius. The SSR
snapshot test locks this composition's HTML alongside the isolated views.

[Open as standalone demo →](../../assets/ui/card-with-dope-sheet.html)

---

[`Card`](../../api/ui_storybook/components/card/fn.Card.html) · [`DopeSheet`](../../api/ui_storybook/components/dope_sheet/fn.DopeSheet.html) · [Components index](../components.md)
