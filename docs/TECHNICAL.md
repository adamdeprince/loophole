# Silkie Loophole: technical notes

How the game is put together: the town and how it is laid out, the
simulation and everybody in it, how a town far bigger than the window is
drawn and scrolled, and how each kitty and Silkie protocol is used.

The terminal layer (capability discovery, input decoding, kitty graphics
and OSC 777 encoding, terminal lifecycle), the frame pacer, the text
overlays, the vector scene model, the lettering and the hen and effects
art come from [Silkie Sprint](../../silkie-sprint/docs/TECHNICAL.md) and
[Silkie Scramble](../../silkie-scramble/docs/TECHNICAL.md) and work as
described there. OSC 777 frames are encoded at any length: the framing
gives the fields no limit of their own (a terminal's `escape_limit` may
bound a whole sequence). This document covers what is new.

## Map of the code

| module | what it does |
|---|---|
| `world` | the town file (`town`), laying out the town from it (`plan`: the terrain grid, houses with addresses along the streets, yards, woods, streetlights, a feature index), and finding the way (`nav`: distance fields, A*, line of sight) |
| `sim` | the deterministic night: the raccoon, its costume, candy, doors and who answers them, Loki and Skaði, the hen, Gravy, the black cat, the townsfolk, things on the ground, discoveries, the autopilot |
| `art` | procedural painting of every sprite sheet, as pixels or as vector elements: the raccoon in its sheet, the bag, Loki, Skaði, their mother and Ethan Allen (by hand), townsfolk (procedural), animals, small things, effects |
| `gfx` | the vector scene model, the town's scenery (`scenery`), the town map (`minimap`), lettering |
| `render` | layout and camera, the chunked renderer, text, uploads, snapshots |
| `term` | capability discovery, input, kitty graphics and OSC 777 encoding, terminal lifecycle |
| `app` | the main loop, the camera, input, presentation (`show`), screens, the journal (`save`), pacing, diagnostics |

## The town

`assets/town/montpelier.txt` (format: `assets/town/README.md`) names the
rivers, streets, lanes and paths, areas, the buildings that matter and a
few hand-placed things. `world::plan::build` turns it into a town:

1. A grid of terrain, one tile each (about five metres): areas, then rivers,
   then paths, sidewalks and roads over them (a way over water is a
   bridge), then buildings.
2. Houses along every street marked `houses`: walking each side, a lot at a
   time, a house of random size is tried at a setback from the sidewalk and
   kept where every tile under it (and a margin) is open grass and no other
   building is close. Each gets an address on its street (odd one side, even
   the other, with gaps), a door facing the street, a walkway, a style
   (body, trim and roof colours, porch, storeys, a turret now and then) and
   a seed.
3. Yards: pumpkins on steps, shrubs either side of the front, a tree, a leaf
   pile or a puddle on the lawn, sometimes a raspberry patch or a candy bowl,
   and once in a while a tree somebody covered in toilet paper.
4. The woods are planted (pines, maples, oaks, birches, undergrowth,
   raspberries at the edges), the greens get big maples, the riverbanks
   birches and shrubs, residential streets street trees; streetlights go up
   on sidewalks, closer together downtown.
5. A feature index buckets everything by 8-tile squares for "what is near
   here" questions.

Everything comes from the file's seed, so the same file makes the same
town. Montpelier makes about 140 houses, 2,000 trees, 100 pumpkins, 18 cows
and 60 raspberry patches; the tests check that every door can be reached
from the start and no buildings overlap.

`Town::describe` turns a point into the location line ("on Main Street,
outside Bare Paws Books", "in the yard of 14 Elm Street", "swimming in the
Winooski River").

## The night

`sim::Game::step` advances the night by 1/60 s. Everything random comes from
one ChaCha8 generator; the same seed and the same inputs make the same
night (a test checks the state hash). The evening clock runs from 5:30 to
9:30 pm at four seconds a minute; then Halloween starts again (bowls full,
leaves re-raked, pumpkins relit), and the raccoon keeps its haul.

### The raccoon

Input is held directions (and run), plus one-shot actions: act, eat,
patch, and a point to walk to (a click or a tap: an A* path, followed
waypoint by waypoint).

**Happiness** (0 to 1) is the energy. Every tile walked costs 0.0038
(running 0.0075, swimming 0.011); candy given at a door adds 0.04, a
friendly word 0.02, a belly rub 0.12, and eating adds the candy's *joy*
(raisins 0.02, chocolate 0.18, cheddar 0.32, a full-size bar 0.45, hákarl
everything). Speed is `pace × (0.4 + 0.6 × happiness)`, times a sugar rush,
times 0.85 when stuffed, times `1 / (1 + bag/600)` for a heavy bag, times the
terrain (woods 0.78, water 0.45). Slipping in poop or barf costs 0.15.

**The belly** fills with what is eaten (up to 12; a raccoon will not eat
past that) and empties by one every 22 seconds. Its level (0 to 4) is in
the art: the sheet bulges and the hem rides up.

**The costume** is twelve places on the sheet (four columns by three rows,
the last the hem: a sheet is a fixed piece of cloth). Each builds up strain;
past 1 a whole place snags, a snag becomes a hole. A patch has a strength
from its material (tape 5, gum 3.5, TP 2.4, paper 1.5, a leaf 0.8...) that
wear uses up, and then it is a hole again. Bushes wear the hem and sides,
thorns anywhere, falls the front, pulls the hem; walking frays the hem a
little per tile. Mud (puddles), wet (swimming, drying over 80 s) and smell
(poop, barf, fading over 150 s) are separate. How well the sheet *passes*:

```
0.92  (a new sheet with a ringed tail out of the back)
- 0.035 per snag - 0.085 per hole + each patch's look (-0.03 gum ... +0.03 evergreen)
- 0.22 dirt - 0.2 wet - 0.15 smell - 0.04 per belly level above 1
```

The costume's coarse state (each place's state, dirt, wet, leaves stuck on,
belly) is its *look*; the art is repainted only when the look changes.

### Doors

Every building that answers has a household: a persona (generous, normal,
suspicious, nearsighted, grumpy, the dentist, the full-size bar family,
maple people, cheese people; keepers for shops and public buildings; the
children's mother; Ethan Allen), a stock of candy and a restocking rate.
Stock and rate depend on the persona and differ by up to 30% between
households, so every porch light comes back on at its own pace. A porch
light is on while there is at least one piece to give.

A knock goes Knocking (0.6 s), Waiting (0.5 to 1.3 s), Open (greeting; the
raccoon says something like "Trick or chitter!"), Answer (2 s), Closing.
The verdict, at Answer:

```
p = passing + 0.12 (Loki or Skaði within 4.5 tiles)
      - wariness (word that there is a raccoon about, fading)
      - 0.3 (a black cat crossed your path)
      - min(0.25, (bag - 40) / 400) (that's quite a haul)
      - 0.25 (smell)
      - 0.45 (same door, costume within 3 places of last time)
pass if p ≥ threshold, or within 0.12 of it half the time
```

Thresholds: nearsighted 0.05, keepers 0.36, generous 0.38, normal 0.5,
suspicious 0.68. Refused far enough below, normal and suspicious people may
scream, which makes every door within 16 tiles warier. A sad raccoon is
sometimes given one out of pity. Children along get a piece each, from the
same bowl. Rival trick-or-treaters take from bowls too.

### Loki and Skaði

Each decides every half second or so when idle. Skaði, in order: her
brother crying (she drops whatever she holds and runs); candy the raccoon
dropped (she brings it back); a torn sheet and a scrap nearby (she patches
it); the hen in the road (she carries her to the nearest sidewalk or
lawn); now and then a cuddle with the hen; otherwise she walks beside the
raccoon. Loki: an egg within 16 tiles (he takes it home, at a run, by
path); otherwise at random a dash past or through the raccoon, a leaf pile,
the hen, something on the ground (eaten, given to the raccoon, or put down
somewhere else), the raccoon's tail, a wander, or following.

Contacts: Loki faster than 3.6 tiles/s spills 10% of the bag (at least one
piece); touching Loki while he carries an egg breaks it (he cries for nine
seconds, Skaði comes and hugs him); Skaði faster than 3.6 knocks the raccoon
down and spills 25% (at least two). Both children holding the hen at once
is a tug of war (2.4 s; Skaði wins three times in five).

Their mother gives them hákarl when they come home (they jump up and down),
and Ethan Allen gives them a paper tricorn (Loki) and a key to Fort
Ticonderoga (Skaði), which they wear from then on.

### Gullinkombi

Hunger rises over 150 s; fullness from eating falls over 140 s. Content
(hunger under 0.3) she ignores food; peckish, she goes for candy within 3.5
tiles and the bag within 2.2; hungry, candy within 12, the bag within 10,
raspberries. Full (over 0.6), she lays every 22 to 34 s and poops every 8 to
13 s (otherwise every 70 to 110 and 30 to 45). Idle, she stays near the
children, pecks, sits, or walks to just ahead of a moving raccoon and sits
down. A raccoon walking into her is held back; one running into her trips.

### Gravy and the cat

Gravy ambles at 0.9 tiles/s around the corner of State and Main. When the
raccoon comes walking his way he goes to a spot 3.5 tiles ahead of it and
lies down, and lying down he is a wall until his belly is rubbed. He goes
for candy within six tiles (2.5 lying down), eats it, and three to seven
seconds later it comes back up: a barf on the ground for 200 s, slipped in
like poop.

The cat crosses in front of a walking raccoon (bad luck: -0.3 at the next
door), hisses at a costume passing under 0.6, and goes up the nearest tree
when Loki comes at a run.

### Townsfolk

About two in three are children trick-or-treating: families led by a parent
and bands of older kids led by the eldest. A leader picks the nearest lit
door not yet visited, the group goes, each child takes from the bowl, and
on to the next. Children grab candy on the ground within 4.5 tiles. A
child who sees a costume passing under 0.5 (or under 0.62, three times in
ten) shouts and runs, the rest of the group with them, and frightened
children drop candy. Every child near the children's house flees the
hákarl. Grown-ups react by temper (friendly, suspicious and telling the
neighbours, oblivious, grumpy, scared) or by who they are: teenagers knock
candy out of big bags, dogs bark, chase and grab the sheet, joggers knock
the raccoon over, tourists take flash pictures, professors and the
legislator have opinions, farmers hand over maple sugar.

All walkers path with A* over the grid (refreshed when the target moves or
they stall) and steer, about 120 of them, in a few tens of microseconds a
tick altogether.

## Drawing a town bigger than the window

### Scenery

`gfx::scenery` describes the whole town once as *pieces*: vector elements in
world units (16 per tile) with their bounds, on two layers: the ground
(everything underfoot, buildings seen from above with their front walls,
signs, street names) and the canopy (tree crowns, lamp heads, flags, toilet
paper), which is drawn over people. Montpelier is about 22,000 pieces.
Repeated shapes (leaves) are symbols placed by uses; glows are one shared
unit radial gradient used by circles with transforms.

The town is cut into square chunks (about 640 pixels, `chunk_tiles`). A
chunk's scene is the pieces whose bounds touch it, in their global order,
under a root transform that maps the chunk to its pixels.

Things that change during the night are *decals*: porch lights, pumpkins,
berries, leaf piles, toilet paper, candy bowls, flyers, the egg basket,
flags. Each owns a fixed run of sixteen element ids and is always drawn
with all of them (an absent part is an empty path), so a change replaces
elements in place and never changes the drawing order.

### Chunks around the camera

The camera eases towards the raccoon (a little ahead of where it is
walking) and stays inside the town. Each frame the renderer works out the
chunks in view, sends any it does not have (and one more from the ring
around the view each frame, so walking on finds them there), places them
(clipped to the field by source rectangle, like sprites), and deletes chunks
too far away: more than two chunks outside the view for vector chunks, one
for pixel chunks. Pixel images are also kept within a budget: kitty's
default image storage quota is 320 MB, past which it evicts images and
placements of them fail. Every pixel image (sheets, chunks, the map) is
counted; town chunks get the 256 MB budget minus the sheets, farthest
chunks go first, and prefetching stops at the budget. Sheets for townsfolk
nobody has worn for fifteen seconds are deleted. On a new size the old
sheets and chunks are deleted before the new ones are sent, so the
terminal never holds both. And on kitty tiles are drawn at most 96 pixels:
pixels grow with the square of the tile, and on a 3000 by 1750 pixel window
at 119 pixels a tile the visible town and the sheets alone came near the
quota. Measured on that window, the most held at once is about 265 MB.

| | kitty | Silkie |
|---|---|---|
| a chunk | rasterized here (a few milliseconds) and sent as pixels | its scene sent as `f=200` records, compressed |
| a decal changes | the chunks it touches are rasterized and sent again | a patch (`a=e`) replacing its elements in every chunk that holds it |
| candles and flags | still | five times a second, a patch of just the parts that changed (a flame, a glow, a flag's cloth; flags at half the rate) of the decals in view: about 15 KB a second downtown |
| scrolling | placements move | placements move |

Silkie redraws only the rectangle a path patch touches, so a porch light
going out costs a few hundred bytes and a few hundred pixels. The tests
check it is a patch on Silkie and a resend on kitty.

A vector image's texture is drawn at the size it is shown, so each chunk is
placed without `c` or `r`: its intrinsic size is its size in pixels, and
clipping it by source rectangle at the edge of the field never changes the
size of its texture (which would draw it all again).

### Sprites

Every sheet is painted by the art code at the tile size times a factor
(people 1.35, the hen 0.85: characters are drawn bigger than the town's
scale, so they read). The painter has two back ends: pixels (tiny-skia),
for kitty; and recorded vector elements, one group per frame, sent to Silkie
as an `f=200` image the size of the sheet, so the terminal draws it. The
same code makes both, so both terminals show the same art. The townsfolk
are a sheet per look, sent the first time someone wearing it is in view.

The raccoon's sheet is repainted when its look changes, into a second image
id; its placement moves over in the same frame and the old image is
deleted once nothing shows it.

Stacking: ground chunks, items on the ground, bodies sorted by the height of
their feet, the canopy, then the raccoon, its bag, the children and the hen
(sorted the same way, above the trees, so they are never hidden), then
Ethan Allen on his tower and effects.

### Text

The status lines, menus, the journal and speech bubbles are terminal text.
A bubble sits above its speaker's head, in the speaker's colour, and moves
up where it would overlap another. On Silkie text backgrounds are
translucent (`SGR 70`).

## The protocols

As in Scramble and Sprint, plus:

* **Vector images** for the scenery chunks, every sprite sheet, the logo and
  the town map. HDR brightness (`*percent`) in the paints of porch-light
  bulbs, candle flames, streetlamp heads, lit windows and the gold dome,
  when the terminal accepted an HDR image probe.
* **Gesture** (`gesture`, mode 1): a pinch in opens the town map, out closes
  it; a smart zoom toggles it.
* **Tablet pointer**: a pen or touchscreen finger touching down walks the
  raccoon there.
* **Haptics**: `click` for knocks, tears and patches; `alignment` for candy,
  favourites and hugs; `generic` for falls, refusals and screams; `complete`
  for a discovery.

## Testing

`cargo test` runs a hundred tests, none needing a terminal: the town
file (mistakes reported with their line), the layout (houses, doors all
reachable, no overlaps, paths across town), the simulation (determinism,
happiness and speed, doors, refusals, restocking at different rates,
bumps, the egg and Skaði coming, Gravy blocking until rubbed and barfing,
frightened children dropping candy, who is out, a four-minute night with
everything pressed), the costume, the art (every sheet in both back ends,
within kitty's image size limit), the scenery (decal ids stable in every
state, chunk sizes), and the renderer by decoding the bytes it writes (the
first frame sends chunks, a still frame sends nothing, scrolling is only
placements, a porch light is a patch on Silkie and a resend on kitty, far
chunks are deleted, pixels held stay within kitty's quota without chunks
being sent over and over, flickering costs little), and the controls (no
letter that opens a screen also walks). `cargo run --release --example soak
-- 40 SEED` runs forty minutes of random input headless.

The game was also run in kitty and Silkie on macOS with `--autoplay`, with
the terminal's replies logged and the window captured.
