# Asset provenance

Everything the game shows is listed here.

## Sprites

Original artwork, painted at run time by the code in `src/art/` (shapes,
gradients and strokes, from fixed seeds) at exactly the size it is shown,
as pixels or as vector records for Silkie.
`cargo run --release --example preview_sheet -- NAME PX out.png` paints any
sheet.

* `raccoon`, `bag`: the raccoon in its bedsheet, and the pillowcase.
* `loki`, `skadi`: Loki and Skaði, the children from Silkie Sprint by the
  same author, redrawn on foot and in Halloween costumes (a dinosaur
  hoodie; cat ears, whiskers and a tail over her own dress), keeping their
  hair, faces, clothes and colours.
* `mom`: their mother, in a maple syrup jug costume.
* `ethan`: Ethan Allen (1738 to 1789), leader of the Green Mountain Boys, as
  a cartoon in period dress.
* `folk0` to `folk40`: townsfolk, procedural: children in ten costumes and
  grown-ups of every build.
* `gravy`, `pets`: Gravy the corded dog, the black cat, walked dogs.
* `hen`, `fx`: from Silkie Sprint and Silkie Scramble, by the same author.
* `stuff`: candy, patching scraps, eggs and the rest.

None of them reproduces a character, layout, animation or look from any
other game.

## Scenery

`src/gfx/scenery.rs` and `src/gfx/minimap.rs` draw the town from the town
file: original. The town is Montpelier, Vermont, drawn from memory and
simplified. Public places keep their own names; shops, restaurants and
hotels have made-up ones, and their jokes are not about the real
businesses.

## Typeface

`assets/fonts/LilitaOne-Regular.ttf`: Lilita One by Juan Montoreano,
licensed under the SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).
Used for the title, signs, street names and the map. Unmodified.

## Licences

The game's code, art and town are under the Apache License 2.0 (`LICENSE`).
The typeface is under the SIL Open Font License 1.1 and is included
unmodified. The dependencies (see `Cargo.lock`) are under permissive
licences (MIT, Apache-2.0, BSD, Zlib, ISC, Unlicense), except `option-ext`
(MPL-2.0, through `directories`), which is used unmodified as a library.

## The town

`assets/town/montpelier.txt`: written for this game.
