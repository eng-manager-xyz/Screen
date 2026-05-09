# Drop zone — idle

<iframe src="../../assets/ui/drop-zone-idle.html" width="100%" height="280" frameborder="0"></iframe>

The recorder's import surface in its resting state. Dashed outline, neutral
copy, optional keyboard hint chip ("⌘O to browse").

The Tauri shell hosts a single `<DropZone>` and flips it between idle and
active in response to OS-level drag events
(`tauri::Window::on_window_event` → `WindowEvent::DragDrop`). Pure
presentational — the component takes a `state: DropZoneState` prop, the
shell owns the signal that drives it.

[Open as standalone demo →](../../assets/ui/drop-zone-idle.html)

---

[`DropZone` API](../../api/ui_storybook/components/drop_zone/fn.DropZone.html) · [Components index](../components.md)
