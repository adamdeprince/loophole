//! Draws the terrain grid, buildings and features as a plain PNG, for
//! checking the town file: `cargo run --example town_grid -- out.png [px]`.
use silkie_loophole::world::{grid::Terrain, plan, town::{self, FeatureKind}};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or("town.png".into());
    let px: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(6);
    let t = plan::build(&town::parse(town::MONTPELIER).unwrap());
    let mut img = image::RgbImage::new(t.w * px, t.h * px);
    for y in 0..t.h as i32 { for x in 0..t.w as i32 {
        let c = match t.grid.get(x, y) {
            Terrain::Grass => [92,140,70], Terrain::Yard => [110,155,80], Terrain::Lawn => [120,175,90],
            Terrain::Woods => [40,90,45], Terrain::Green => [100,165,85], Terrain::Parking => [90,90,95],
            Terrain::Plaza => [170,160,150], Terrain::Road => [60,60,66], Terrain::Sidewalk => [175,175,170],
            Terrain::Path => [190,170,130], Terrain::Bridge => [140,110,80], Terrain::Water => [50,90,160],
            Terrain::Building => [150,80,70],
        };
        for dy in 0..px { for dx in 0..px { img.put_pixel(x as u32*px+dx, y as u32*px+dy, image::Rgb(c)); } }
    }}
    fn dot(img: &mut image::RgbImage, px: u32, x: f32, y: f32, r: f32, c: [u8;3]) {
        let (cx, cy, rr) = (x*px as f32, y*px as f32, (r*px as f32).max(1.0));
        for yy in (cy-rr) as i32..=(cy+rr) as i32 { for xx in (cx-rr) as i32..=(cx+rr) as i32 {
            if xx>=0 && yy>=0 && (xx as u32) < img.width() && (yy as u32) < img.height() && ((xx as f32-cx).powi(2)+(yy as f32-cy).powi(2)) <= rr*rr {
                img.put_pixel(xx as u32, yy as u32, image::Rgb(c)); } } }
    }
    for b in &t.buildings {
        let c = match b.kind { town::BuildingKind::House => [200,190,150], town::BuildingKind::Home => [255,120,200], _ => [230,120,60] };
        for yy in (b.rect.y*px as f32) as u32..(b.rect.y1()*px as f32) as u32 { for xx in (b.rect.x*px as f32) as u32..(b.rect.x1()*px as f32) as u32 {
            if xx < img.width() && yy < img.height() { img.put_pixel(xx, yy, image::Rgb(c)); } } }
        let s = b.door.step(); dot(&mut img, px, s.x, s.y, 0.3, [255,255,0]);
    }
    for f in &t.features {
        let c = match f.kind { FeatureKind::Tree(_) => [20,60,20], FeatureKind::Bush => [60,120,50], FeatureKind::Raspberry => [200,30,60],
            FeatureKind::Pumpkin => [255,140,0], FeatureKind::Cow => [255,255,255], FeatureKind::Streetlight => [255,240,160], _ => [255,0,255] };
        dot(&mut img, px, f.pos.x, f.pos.y, if matches!(f.kind, FeatureKind::Tree(_)|FeatureKind::Cow) {0.45} else {0.25}, c);
    }
    let s = t.start; dot(&mut img, px, s.x, s.y, 0.8, [255,0,0]);
    img.save(&out).unwrap();
}
