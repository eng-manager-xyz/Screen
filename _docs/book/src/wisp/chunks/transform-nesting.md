# Nested transforms — M0.7 / M0.8

![transform nesting](../../assets/wisp/transform-nesting.png)

Three nested containers, each spinning at a different rate. Children inherit
their parents' transforms — the inner ring's apparent path is the composition
of all three rotations.

This is the M0.7+M0.8 contract made visible: the renderer's pre-order
traversal multiplies parent-world × local on the way down
(`compose(world_parent, &local)`), so each sprite's final clip-space position
is `parent_outer · parent_mid · parent_inner · local_sprite`.

The recorder uses the same pattern for nested compositions: the recording
quad is a child of a "padding" container, which is a child of a "background"
container. Animating the padding container's transform slides the entire
recording smoothly without touching individual children.

Reverse-direction rotation in the middle ring is intentional — it makes the
composition obvious. If you stopped one ring at a time you could read off
each transform's contribution.

---

[`Container` API](../../api/wisp/scene/struct.Container.html) · [`Transform`](../../api/wisp/scene/struct.Transform.html) · [Stories index](../stories.md)
