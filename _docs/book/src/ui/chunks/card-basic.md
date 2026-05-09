# Card — header + body

<iframe src="../../assets/ui/card-basic.html" width="100%" height="240" frameborder="0"></iframe>

`Card` is a surface container. `CardHeader { title, subtitle }` sits above
a `CardBody` separated by a 1px divider. Used everywhere the editor groups
related controls — recording metadata, timeline, export presets.

The composition pattern (`<Card> <CardHeader/> <CardBody>…</CardBody>
</Card>`) is rust-ui-flavored: small composable building blocks rather than
a single fat component with a dozen props.

[Open as standalone demo →](../../assets/ui/card-basic.html)

---

[`Card` API](../../api/ui_storybook/components/card/fn.Card.html) · [Components index](../components.md)
