# Where this book sits

`wisp-chart` is one of three mdBooks composed into the same
GitHub Pages artifact.

```mermaid
flowchart LR
    Screen["/Screen/<br/>screen recorder"]:::shell
    Wisp["/Screen/wisp/<br/>wisp renderer"]:::wisp
    Chart["/Screen/wisp-chart/<br/>chart compositions"]:::chart
    Api["/Screen/api/<br/>rustdoc"]:::api

    Chart -->|depends on| Wisp
    Screen -->|uses| Wisp
    Screen -->|uses| Chart

    classDef shell fill:#1e293b,stroke:#475569,color:#e2e8f0
    classDef wisp fill:#7c2d12,stroke:#ea580c,color:#fed7aa
    classDef chart fill:#312e81,stroke:#6366f1,color:#e0e7ff
    classDef api fill:#374151,stroke:#9ca3af,color:#f3f4f6
```

## Cross-book tags

The `mdbook-preprocessor-cross` tags resolve per-book. Inside
this book:

| Tag | Renders as |
|---|---|
| `{{wisp-link wisp/overview}}` | `/Screen/wisp/wisp/overview.html` |
| `{{wisp-chart-link charts/gantt/api}}` | `./charts/gantt/api.html` (relative — same book) |
| `{{shared cross-link-convention.md}}` | inlined from `_docs/shared/` |

Inside the screen + wisp books, the same `{{wisp-chart-link}}`
tag emits an absolute URL to `/Screen/wisp-chart/...`. Authors
write the tag once; the preprocessor adapts per book.
