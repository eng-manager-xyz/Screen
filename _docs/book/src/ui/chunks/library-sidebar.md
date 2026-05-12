# Library sidebar

[Linear: AUT-134](https://linear.app/harwood/issue/AUT-134)

Left rail of the library screen. Primary nav rows (New / All /
Starred / Shared / Inbox), `SPACES`, `TAGS`, and a bottom storage
quota meter that turns red past 85%.

<iframe src="../../assets/ui/library-sidebar-default.html" width="280" height="640" frameborder="0"></iframe>

## States

| State | Story |
| --- | --- |
| Default | [`library-sidebar-default`](../../assets/ui/library-sidebar-default.html) |
| Inbox active | [`library-sidebar-inbox-active`](../../assets/ui/library-sidebar-inbox-active.html) |
| 95% storage (warning) | [`library-sidebar-high-storage`](../../assets/ui/library-sidebar-high-storage.html) |
| No spaces section | [`library-sidebar-empty-spaces`](../../assets/ui/library-sidebar-empty-spaces.html) |
| Long labels truncate | [`library-sidebar-long-labels`](../../assets/ui/library-sidebar-long-labels.html) |

## API

```rust
use ui_storybook::components::library::{LibrarySidebar, LibrarySidebarView};
use ui_storybook::fixtures::library::sample_library_sidebar;

view! {
    <LibrarySidebar view=sample_library_sidebar(/* inbox_unread */ 3) />
}
```

```admonish important title="StorageMeter clamp"
`StorageMeterView::percent_used` is a `0.0..=1.0` fraction.
`storage_percent` clamps and rounds it before applying the warn
threshold — the parent doesn't have to pre-validate. The bar turns
red at 85%.
```
