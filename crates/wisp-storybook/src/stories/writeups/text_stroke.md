Stroked text — a `TextTexturePipeline` renders the glyphs into a
texture once; `stroked_text_sprites` then stamps that texture eight
times around a center point in the stroke color, with one final stamp
on top in the fill color. The chaotic pink/yellow/cyan/dark backdrop
makes the difference legible: without the stroke, "READABLE" would
disappear into the yellow band.

No shader changes. The technique is the same one CSS uses for
`text-stroke`: stamping a glyph texture in a ring around a center.
Eight directions at √2/2 increments approximates a circle; raise the
direction count to smooth bigger strokes, raise the radius for
thicker strokes.

For the recorder this becomes: captions, lower-thirds, recording
labels, click annotations — anything that has to stay readable over
whatever the user is recording.
