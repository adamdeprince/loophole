//! The town description: a plain text file of streets, rivers, areas,
//! buildings and features (`assets/town/README.md` documents the format).
//! Reading it gives a [`TownSpec`]; `plan::build` lays out the houses and the
//! terrain grid from that.

use super::{Dir, Rect};
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreetKind {
    /// A road with sidewalks on both sides.
    Street,
    /// A narrow road without sidewalks.
    Lane,
    /// A footpath (the bike path, park trails, walkways).
    Path,
    /// A footbridge over water.
    Footbridge,
}

#[derive(Clone, Debug)]
pub struct Street {
    pub name: String,
    pub kind: StreetKind,
    /// Width of the roadway (or path) in tiles.
    pub width: f32,
    pub pts: Vec<Vec2>,
    /// Houses are laid out along it.
    pub houses: bool,
    /// Shops, brick and streetlights close together.
    pub downtown: bool,
}

impl Street {
    pub fn sidewalk(&self) -> f32 {
        if self.kind == StreetKind::Street { 1.2 } else { 0.0 }
    }

    /// Half the width taken up by the street, sidewalks included.
    pub fn half_span(&self) -> f32 {
        self.width / 2.0 + self.sidewalk()
    }

    pub fn length(&self) -> f32 {
        self.pts.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// The point `s` tiles along the street and the direction there.
    pub fn at(&self, s: f32) -> (Vec2, Vec2) {
        let mut left = s.max(0.0);
        for w in self.pts.windows(2) {
            let len = w[0].distance(w[1]);
            let dir = (w[1] - w[0]).normalize_or_zero();
            if left <= len {
                return (w[0] + dir * left, dir);
            }
            left -= len;
        }
        let n = self.pts.len();
        let dir = if n >= 2 { (self.pts[n - 1] - self.pts[n - 2]).normalize_or_zero() } else { Vec2::X };
        (self.pts.last().copied().unwrap_or(Vec2::ZERO), dir)
    }
}

#[derive(Clone, Debug)]
pub struct River {
    pub name: String,
    pub width: f32,
    pub pts: Vec<Vec2>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaKind {
    /// Trees close together and undergrowth (Hubbard Park).
    Woods,
    /// Mown grass in front of a public building.
    Lawn,
    /// A college green with big old trees.
    Green,
    Parking,
    /// Paved, for people (a plaza, a churchyard path).
    Plaza,
}

#[derive(Clone, Debug)]
pub struct Area {
    pub kind: AreaKind,
    pub name: String,
    pub pts: Vec<Vec2>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BuildingKind {
    House,
    /// Where Loki and Skaði live; Loki brings eggs home here.
    Home,
    Shop,
    Restaurant,
    /// Dark at night: nobody answers.
    Office,
    StateHouse,
    Museum,
    Court,
    Church,
    Library,
    FireHouse,
    CityHall,
    College,
    PostOffice,
    Hotel,
    Theater,
    Coop,
    Grocery,
    /// The stone lookout tower in Hubbard Park.
    Tower,
}

impl BuildingKind {
    pub fn parse(s: &str) -> Option<BuildingKind> {
        Some(match s {
            "house" => BuildingKind::House,
            "home" => BuildingKind::Home,
            "shop" => BuildingKind::Shop,
            "restaurant" => BuildingKind::Restaurant,
            "office" => BuildingKind::Office,
            "statehouse" => BuildingKind::StateHouse,
            "museum" => BuildingKind::Museum,
            "court" => BuildingKind::Court,
            "church" => BuildingKind::Church,
            "library" => BuildingKind::Library,
            "firehouse" => BuildingKind::FireHouse,
            "cityhall" => BuildingKind::CityHall,
            "college" => BuildingKind::College,
            "postoffice" => BuildingKind::PostOffice,
            "hotel" => BuildingKind::Hotel,
            "theater" => BuildingKind::Theater,
            "coop" => BuildingKind::Coop,
            "grocery" => BuildingKind::Grocery,
            "tower" => BuildingKind::Tower,
            _ => return None,
        })
    }

    /// Somebody may answer the door.
    pub fn answers(self) -> bool {
        !matches!(self, BuildingKind::Office)
    }

    /// Downtown brick, as opposed to a clapboard house.
    pub fn is_commercial(self) -> bool {
        matches!(
            self,
            BuildingKind::Shop
                | BuildingKind::Restaurant
                | BuildingKind::Hotel
                | BuildingKind::Theater
                | BuildingKind::Coop
                | BuildingKind::Grocery
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Door {
    /// The middle of the doorway, on the wall.
    pub at: Vec2,
    pub facing: Dir,
}

impl Door {
    /// Where someone stands to knock: just outside the door.
    pub fn step(&self) -> Vec2 {
        self.at + self.facing.vec() * 0.8
    }
}

#[derive(Clone, Debug)]
pub struct BuildingSpec {
    pub kind: BuildingKind,
    pub name: String,
    pub rect: Rect,
    pub door: Dir,
    /// Along the wall, as a fraction (0.5 is the middle).
    pub door_at: f32,
    pub say: Option<String>,
    pub gives: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TreeKind {
    Maple,
    Oak,
    Birch,
    Pine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FeatureKind {
    Tree(TreeKind),
    /// A shrub or a stretch of hedge: walk through it and the sheet snags.
    Bush,
    /// Berries (they count as candy) behind thorns (they tear sheets).
    Raspberry,
    /// Raked leaves: run through and they go everywhere, and stick.
    LeafPile,
    /// Mud.
    Puddle,
    Streetlight,
    /// A jack-o'-lantern on a step.
    Pumpkin,
    /// A tree somebody covered in toilet paper.
    TpTree,
    Bench,
    Hydrant,
    /// A pole with flyers stapled to it: paper for patching.
    FlyerPole,
    /// A bowl of candy left out with a "please take one" sign.
    CandyBowl,
    /// The egg basket on the kids' porch.
    EggBasket,
    /// Ceres, or anyone else on a plinth.
    Statue,
    /// A life-size fibreglass cow, painted by a local artist.
    Cow,
    /// A flag on a pole: the Stars and Stripes.
    Flag,
    /// The Green Mountain Boys' flag: green, a blue canton, thirteen stars.
    GmbFlag,
}

impl FeatureKind {
    pub fn parse(s: &str) -> Option<FeatureKind> {
        Some(match s {
            "maple" => FeatureKind::Tree(TreeKind::Maple),
            "oak" => FeatureKind::Tree(TreeKind::Oak),
            "birch" => FeatureKind::Tree(TreeKind::Birch),
            "pine" => FeatureKind::Tree(TreeKind::Pine),
            "bush" => FeatureKind::Bush,
            "raspberry" => FeatureKind::Raspberry,
            "leafpile" => FeatureKind::LeafPile,
            "puddle" => FeatureKind::Puddle,
            "streetlight" => FeatureKind::Streetlight,
            "pumpkin" => FeatureKind::Pumpkin,
            "tptree" => FeatureKind::TpTree,
            "bench" => FeatureKind::Bench,
            "hydrant" => FeatureKind::Hydrant,
            "flyerpole" => FeatureKind::FlyerPole,
            "candybowl" => FeatureKind::CandyBowl,
            "statue" => FeatureKind::Statue,
            "cow" => FeatureKind::Cow,
            "flag" => FeatureKind::Flag,
            "gmbflag" => FeatureKind::GmbFlag,
            _ => return None,
        })
    }

    /// The radius a creature is kept out of (trunks, poles), or zero.
    pub fn solid_radius(self) -> f32 {
        match self {
            FeatureKind::Tree(_) | FeatureKind::TpTree => 0.32,
            FeatureKind::Streetlight | FeatureKind::FlyerPole | FeatureKind::Hydrant | FeatureKind::Flag | FeatureKind::GmbFlag => 0.18,
            FeatureKind::Statue => 0.6,
            FeatureKind::Cow => 0.55,
            FeatureKind::Bench => 0.35,
            _ => 0.0,
        }
    }

    /// The radius within which walking through it does something.
    pub fn reach(self) -> f32 {
        match self {
            FeatureKind::Bush => 0.75,
            FeatureKind::Raspberry => 0.85,
            FeatureKind::LeafPile => 0.75,
            FeatureKind::Puddle => 0.8,
            FeatureKind::Pumpkin => 0.4,
            FeatureKind::CandyBowl => 0.6,
            FeatureKind::FlyerPole => 0.7,
            FeatureKind::TpTree => 0.95,
            _ => self.solid_radius(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FeatureSpec {
    pub kind: FeatureKind,
    pub pos: Vec2,
    /// For puddles and bushes: how big.
    pub size: f32,
}

/// Everything the file says, before the houses are laid out.
#[derive(Clone, Debug, Default)]
pub struct TownSpec {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub start: Vec2,
    pub seed: u64,
    pub streets: Vec<Street>,
    pub rivers: Vec<River>,
    pub areas: Vec<Area>,
    pub buildings: Vec<BuildingSpec>,
    pub features: Vec<FeatureSpec>,
}

/// A mistake in the town file, with its line.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Splits a line into words, keeping "quoted strings" (and key="quoted
/// values") whole. A `#` outside quotes starts a comment.
fn words(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            '#' if !quoted => break,
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if quoted {
        return Err("a quoted string is not closed".into());
    }
    if any {
        out.push(cur);
    }
    Ok(out)
}

fn num(s: &str) -> Result<f32, String> {
    s.parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{s:?} is not a number"))
}

fn point(s: &str) -> Result<Vec2, String> {
    let (x, y) = s.split_once(',').ok_or_else(|| format!("{s:?} is not a point (x,y)"))?;
    Ok(Vec2::new(num(x)?, num(y)?))
}

/// The points after a `:` word.
fn points(w: &[String]) -> Result<Vec<Vec2>, String> {
    let i = w.iter().position(|s| s == ":").ok_or("expected ':' followed by points")?;
    let pts = w[i + 1..].iter().map(|s| point(s)).collect::<Result<Vec<_>, _>>()?;
    if pts.len() < 2 {
        return Err("needs at least two points".into());
    }
    Ok(pts)
}

/// `key=value` options among the words before the `:`.
fn option<'a>(w: &'a [String], key: &str) -> Option<&'a str> {
    let stop = w.iter().position(|s| s == ":").unwrap_or(w.len());
    w[..stop].iter().find_map(|s| s.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
}

fn flag(w: &[String], key: &str) -> bool {
    let stop = w.iter().position(|s| s == ":").unwrap_or(w.len());
    w[..stop].iter().any(|s| s == key)
}

pub fn parse(text: &str) -> Result<TownSpec, ParseError> {
    let mut spec = TownSpec {
        seed: 1,
        ..Default::default()
    };
    let mut saw_town = false;
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let err = |message: String| ParseError { line, message };
        let w = words(raw).map_err(err)?;
        if w.is_empty() {
            continue;
        }
        let r: Result<(), String> = (|| {
            match w[0].as_str() {
                "town" => {
                    if w.len() < 4 {
                        return Err("town \"name\" width height".into());
                    }
                    spec.name = w[1].clone();
                    spec.width = num(&w[2])? as u32;
                    spec.height = num(&w[3])? as u32;
                    if spec.width < 8 || spec.height < 8 {
                        return Err("the town must be at least 8 by 8 tiles".into());
                    }
                    saw_town = true;
                }
                "start" => {
                    if w.len() < 3 {
                        return Err("start x y".into());
                    }
                    spec.start = Vec2::new(num(&w[1])?, num(&w[2])?);
                }
                "seed" => {
                    spec.seed = w.get(1).and_then(|s| s.parse().ok()).ok_or("seed N")?;
                }
                "street" | "lane" | "path" | "footbridge" => {
                    let kind = match w[0].as_str() {
                        "street" => StreetKind::Street,
                        "lane" => StreetKind::Lane,
                        "path" => StreetKind::Path,
                        _ => StreetKind::Footbridge,
                    };
                    if w.len() < 3 {
                        return Err(format!("{} \"name\" width [houses] [downtown] : points", w[0]));
                    }
                    let width = num(&w[2])?;
                    if width <= 0.0 {
                        return Err("width must be positive".into());
                    }
                    spec.streets.push(Street {
                        name: w[1].clone(),
                        kind,
                        width,
                        pts: points(&w)?,
                        houses: flag(&w, "houses"),
                        downtown: flag(&w, "downtown"),
                    });
                }
                "river" => {
                    if w.len() < 3 {
                        return Err("river \"name\" width : points".into());
                    }
                    spec.rivers.push(River {
                        name: w[1].clone(),
                        width: num(&w[2])?,
                        pts: points(&w)?,
                    });
                }
                "area" => {
                    if w.len() < 3 {
                        return Err("area kind \"name\" : points".into());
                    }
                    let kind = match w[1].as_str() {
                        "woods" => AreaKind::Woods,
                        "lawn" => AreaKind::Lawn,
                        "green" => AreaKind::Green,
                        "parking" => AreaKind::Parking,
                        "plaza" => AreaKind::Plaza,
                        k => return Err(format!("unknown area kind {k:?}")),
                    };
                    let pts = points(&w)?;
                    if pts.len() < 3 {
                        return Err("an area needs at least three points".into());
                    }
                    spec.areas.push(Area {
                        kind,
                        name: w[2].clone(),
                        pts,
                    });
                }
                "building" | "home" => {
                    let (kind, rest) = if w[0] == "home" {
                        (BuildingKind::Home, &w[1..])
                    } else {
                        let k = w.get(1).ok_or("building kind \"name\" x y w h door=S")?;
                        (BuildingKind::parse(k).ok_or_else(|| format!("unknown building kind {k:?}"))?, &w[2..])
                    };
                    if rest.len() < 5 {
                        return Err("expected \"name\" x y w h".into());
                    }
                    let rect = Rect::new(num(&rest[1])?, num(&rest[2])?, num(&rest[3])?, num(&rest[4])?);
                    if rect.w < 1.0 || rect.h < 1.0 {
                        return Err("a building must be at least a tile each way".into());
                    }
                    let door = match option(rest, "door") {
                        Some(d) => Dir::parse(d).ok_or_else(|| format!("door={d}: use N, E, S or W"))?,
                        None => Dir::S,
                    };
                    let door_at = match option(rest, "at") {
                        Some(v) => num(v)?.clamp(0.1, 0.9),
                        None => 0.5,
                    };
                    spec.buildings.push(BuildingSpec {
                        kind,
                        name: rest[0].clone(),
                        rect,
                        door,
                        door_at,
                        say: option(rest, "say").map(str::to_string),
                        gives: option(rest, "gives").map(str::to_string),
                    });
                }
                k => {
                    let kind = FeatureKind::parse(k).ok_or_else(|| format!("unknown directive {k:?}"))?;
                    if w.len() < 3 {
                        return Err(format!("{k} x y [size]"));
                    }
                    let size = match w.get(3) {
                        Some(s) => num(s)?,
                        None => 1.0,
                    };
                    spec.features.push(FeatureSpec {
                        kind,
                        pos: Vec2::new(num(&w[1])?, num(&w[2])?),
                        size,
                    });
                }
            }
            Ok(())
        })();
        r.map_err(err)?;
    }
    if !saw_town {
        return Err(ParseError {
            line: 1,
            message: "the file must start with: town \"name\" width height".into(),
        });
    }
    if !spec.buildings.iter().any(|b| b.kind == BuildingKind::Home) {
        return Err(ParseError {
            line: text.lines().count().max(1),
            message: "the town needs a home for the children".into(),
        });
    }
    Ok(spec)
}

/// The town the game ships with.
pub const MONTPELIER: &str = include_str!("../../assets/town/montpelier.txt");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_keep_quotes_and_drop_comments() {
        let w = words(r#"building shop "Bare Paws Books" 1 2 3 4 say="Hello there" # comment"#).unwrap();
        assert_eq!(w, vec!["building", "shop", "Bare Paws Books", "1", "2", "3", "4", "say=Hello there"]);
    }

    #[test]
    fn mistakes_name_their_line() {
        let e = parse("town \"T\" 20 20\nhome \"h\" 1 1 3 3\nstreet \"A\" 3 : 0,0\n").unwrap_err();
        assert_eq!(e.line, 3);
        let e = parse("town \"T\" 20 20\nwidget 1 2\n").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("widget"));
    }

    #[test]
    fn montpelier_parses() {
        let s = parse(MONTPELIER).unwrap();
        assert!(s.streets.iter().any(|st| st.name == "State Street"));
        assert!(s.streets.iter().any(|st| st.name == "Main Street"));
        assert!(s.buildings.iter().any(|b| b.kind == BuildingKind::StateHouse));
        assert!(s.buildings.iter().any(|b| b.kind == BuildingKind::Library));
        assert!(s.buildings.iter().any(|b| b.kind == BuildingKind::FireHouse));
        assert!(s.buildings.iter().any(|b| b.kind == BuildingKind::College));
    }
}
