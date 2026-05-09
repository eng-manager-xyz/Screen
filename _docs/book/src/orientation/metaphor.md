# Theatre metaphor

A theatre metaphor underpins the codebase navigation. It's the mental model
for where things live and what they're allowed to do.

| Term | Code | Role |
|---|---|---|
| Stage | `wisp::Stage` | The root scene container. |
| Wings | `_docs/` | Off-stage planning, milestone scripts, conventions. |
| Acts | Milestones (M0, M1, …) | Long arcs of narrative. |
| Scenes | Chunks (M0.5, M0.6, …) | Individual numbered units of work. |
| Cast | Public `Stage` children | The named entities visible from a scene. |
| Rehearsal | The recursive-fix loop | We don't ship until `just gate` is green. |
| Storybook | `wisp-storybook` / `ui-storybook` | Where each scene's run is captured for re-watching. |

Why bother: when "scope creep" feels like adding a character, that's a clear
no. When a chunk feels like adding a prop to an existing scene, that's a clear
yes. The metaphor short-circuits a lot of architectural debate.
