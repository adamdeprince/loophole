# The town file

`montpelier.txt` describes the town the game is played in. It is read when
the game starts (`--town FILE` plays another), so a street in the wrong place
or a shop with the wrong joke is fixed by editing the file:

```sh
silkie-loophole town assets/town/montpelier.txt   # check it, list the places
silkie-loophole --town my-town.txt                 # play it
cargo run --example town_grid -- out.png 5         # a plain picture of the grid
cargo run --release --example town_view -- 110 110 40 25 32 out.png
```

A mistake is reported with its line number.

## Coordinates

One tile is about five metres. `x` runs east, `y` runs south, both from the
top left corner. Points are written `x,y` (no space); a line of points
follows a `:`.

## Lines

`#` starts a comment. Words in "double quotes" may contain spaces, and
`key="quoted value"` options work the same way.

```
town "Montpelier" 260 196        first line: name, width, height in tiles
seed 1791                        houses, yards and woods are laid out from this
start 128 132                    where the raccoon starts
```

### Water

```
river "Winooski River" 10 : 0,122 30,123.5 ...
```

A river is a smooth line and a width. Streets and paths that cross it get a
bridge drawn and are walkable; anywhere else the raccoon swims.

### Ways

```
street "State Street" 4 houses downtown : 0,130 135,130
lane "Langdon Street" 2.5 downtown : 135,116 121.5,116
path "the bike path" 1.2 : 0,170.5 40,171 ...
footbridge "the Langdon Street footbridge" 1.8 : 121.5,116 113,116
```

The number is the width of the roadway in tiles. A `street` gets sidewalks
(1.2 tiles each side), a centre line and streetlights; a `lane` has neither.
`houses` lays out houses along both sides, with addresses, wherever there is
open grass; `downtown` brings the streetlights closer together and the
litter out. Street names are painted on the road.

### Areas

```
area woods "Hubbard Park" : 0,0 116,0 ...
```

Kinds: `woods` (trees and undergrowth are planted in it), `lawn`, `green`
(big old trees), `parking`, `plaza`. The name is used in the location line
("in Hubbard Park").

### Buildings

```
building shop "Dog-Eared Books" 123 100 8.6 7 door=E say="..." gives=bookmark
home "Loki and Skaði's house" 138.8 68.6 6 5 door=W say="..." gives=hakarl
```

`x y w h` is the footprint. `door=` is the wall the door is in (`N`, `E`,
`S`, `W`; south by default) and `at=` how far along it (0.1 to 0.9, the middle
by default). `say=` is what whoever answers says when they hand something
over, and `gives=` what besides (or instead of) candy: `chocolate`,
`bookmark`, `pamphlet`, `sticker`, `tape`, `popcorn`, `carob`, `maple`,
`hakarl`, `ethan`.

Kinds: `house`, `shop`, `restaurant`, `office` (dark: nobody answers),
`statehouse`, `museum`, `court`, `church`, `library`, `firehouse`,
`cityhall`, `college`, `postoffice`, `hotel`, `theater`, `coop`, `grocery`,
`tower`. Exactly one `home` is needed: Loki and Skaði live there, their
mother answers its door, and Loki brings the hen's eggs to its porch.

Who answers a house's door (generous, suspicious, nearsighted, the dentist,
the full-size bar family...) and how fast they restock is decided by the
seed.

### Things

```
raspberry 46 87 1.6
cow 30 127.6
flag 6.0 127.15
```

One per line, with a position and sometimes a size: `maple`, `oak`,
`birch`, `pine`, `bush`, `raspberry`, `leafpile`, `puddle`, `streetlight`,
`pumpkin`, `tptree`, `bench`, `hydrant`, `flyerpole`, `candybowl`,
`statue`, `cow`, `flag`, `gmbflag`. Most of the town's trees, shrubs,
pumpkins and leaf piles are planted by the generator; these lines are for
the ones that have to be just so. A thing inside a building or in a river
is left out with a warning in the log.
