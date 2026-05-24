use macroquad::prelude::*;
use ::rand::Rng;
use std::f32::consts::{PI, TAU, FRAC_1_SQRT_2};

const PLAYER_SIZE: f32 = 18.0;
const MAX_PARTICLES: usize = 600;
const MAX_POWERUPS: usize = 20;
const MAX_BOSS_BULLETS: usize = 300;
const HIGHSCORE_FILE: &str = "highscore.txt";

// === ENUMS ===
#[derive(Clone, Copy, PartialEq)]
enum WeaponType { Normal, Spread, Homing, Laser }

#[derive(Clone, Copy, PartialEq)]
enum AsteroidKind { Normal, Fire, Ice }

// Enemy traits - each enemy is a unique combination
#[derive(Clone, Copy, PartialEq)]
enum EnemySize { Tiny, Small, Medium, Large, Huge }
#[derive(Clone, Copy, PartialEq)]
enum EnemyMove { Chase, Strafe, Zigzag, Orbit, Rush }
#[derive(Clone, Copy, PartialEq)]
enum EnemyWeapon { None, Single, Spread, Burst, Sniper }
#[derive(Clone, Copy, PartialEq)]
enum EnemySpecial { None, Bomb, Shield, Regen }

#[derive(Clone, Copy, PartialEq)]
enum BossKind { Default, Barrage, Tank, Triplet }

#[derive(Clone, Copy, PartialEq)]
enum GameState { Menu, DiffSelect, Playing, Paused, Shopping, GameOver }

#[derive(Clone, Copy, PartialEq)]
enum DiffMode { Easy, Normal, Hard }

// === ENEMY TEMPLATES (100 UNIQUE) ===
// Each template is a combination of traits encoded in a u16:
// bits 0-1: size (0-4), bits 2-3: move (0-4), bits 4-5: weapon (0-4),
// bits 6-7: special (0-3), bits 8-9: color_offset (0-3)

fn template_count() -> usize { 100 }

fn template_meta(id: usize) -> (EnemySize, EnemyMove, EnemyWeapon, EnemySpecial, f32) {
    let _r = id as f32 * 1.6180339887;
    let sz = match (id / 1) % 5 { 0 => EnemySize::Tiny, 1 => EnemySize::Small, 2 => EnemySize::Medium, 3 => EnemySize::Large, _ => EnemySize::Huge };
    let mv = match (id / 5) % 5 { 0 => EnemyMove::Chase, 1 => EnemyMove::Strafe, 2 => EnemyMove::Zigzag, 3 => EnemyMove::Orbit, _ => EnemyMove::Rush };
    let wp = match (id / 25) % 5 { 0 => EnemyWeapon::None, 1 => EnemyWeapon::Single, 2 => EnemyWeapon::Spread, 3 => EnemyWeapon::Burst, _ => EnemyWeapon::Sniper };
    let sp = match (id / 125) % 4 { 0 => EnemySpecial::None, 1 => EnemySpecial::Bomb, 2 => EnemySpecial::Shield, _ => EnemySpecial::Regen };
    let hue = (id as f32 * 37.7) % 360.0;
    (sz, mv, wp, sp, hue)
}

fn template_stats(id: usize) -> (f32, f32, f32, i32) {
    // Returns (size_mult, speed_mult, hp_mult, gold_value)
    let base = |x: f32| 0.5 + x * 0.5;
    let sz_i = (id / 1) % 5;
    let sp_i = (id / 5) % 5;
    let hp_i = (id / 25) % 5;
    let gv = match (id % 3) { 0 => 2, 1 => 3, _ => 4 } + (hp_i as i32)/2;
    let sm = match sz_i { 0 => 0.6, 1 => 0.8, 2 => 1.0, 3 => 1.4, _ => 1.8 };
    let spm = match sp_i { 0 => 0.5, 1 => 0.75, 2 => 1.0, 3 => 1.5, _ => 2.2 };
    let hpm = match hp_i { 0 => 0.4, 1 => 0.7, 2 => 1.0, 3 => 1.8, _ => 3.5 };
    (sm, spm, hpm, gv)
}

fn template_name(id: usize) -> &'static str {
    let names = [
        "Scout", "Runner", "Striker", "Blitzer", "Charger", "Dart", "Flea", "Spark", "Zippy", "Viper",
        "Maurader", "Raider", "Prowler", "Stalker", "Hunter", "Fang", "Claw", "Fury", "Rage", "Wrath",
        "Guardian", "Sentinel", "Bulwark", "Fortress", "Bastion", "Aegis", "Shield", "Wall", "Rock", "Stone",
        "Flame", "Blaze", "Inferno", "Ember", "Cinder", "Scorch", "Burn", "Heat", "Spark", "Flash",
        "Frost", "Glacier", "Shiver", "Chill", "Ice", "Snow", "Hail", "Blizzard", "Crystal", "Gem",
        "Shadow", "Shade", "Dark", "Night", "Gloom", "Dusk", "Eclipse", "Void", "Abyss", "Phantom",
        "Thunder", "Storm", "Bolt", "Zap", "Surge", "Volt", "Static", "Spark", "Flash", "Boom",
        "Venom", "Toxin", "Poison", "Viper", "Serpent", "Fang", "Sting", "Bite", "Acid", "Corrode",
        "Beam", "Ray", "Laser", "Prism", "Lens", "Focus", "Light", "Glow", "Shine", "Radiant",
        "Omega", "Alpha", "Delta", "Sigma", "Nova", "Star", "Comet", "Meteor", "Nebula", "Cosmos",
    ];
    let idx = (id * 7 + 3) % names.len();
    names[idx]
}

// === STRUCTS ===
struct Star { x: f32, y: f32, speed: f32, size: f32, brightness: f32, layer: f32 }

struct Player {
    x: f32, y: f32, angle: f32, speed: f32,
    hp: i32, max_hp: i32, shield: f32, max_shield: f32,
    energy: f32, max_energy: f32,
    weapon: WeaponType, weapon_level: i32, drones: i32,
    invincible: f32, dash_cooldown: f32,
    combo: i32, combotimer: f32, kills: i64, score: i64,
    gold: i64, damage_bonus: i32, fire_rate_bonus: f32,
    hp_regen: f32, hp_regen_accum: f32, dash_reduction: f32, bullet_size: f32, combo_bonus: f32,
    shop_hp_lv: i32, shop_shield_lv: i32, shop_speed_lv: i32,
    shop_weapon_lv: i32, shop_drone_lv: i32,
    shop_damage_lv: i32, shop_firerate_lv: i32, shop_energy_lv: i32,
    shop_speed2_lv: i32, shop_firerate2_lv: i32, shop_regen_lv: i32,
    shop_dash_lv: i32, shop_bullet_lv: i32, shop_combo_lv: i32,
    shop_gold_lv: i32, shop_energy_regen_lv: i32,
}

#[derive(Clone)]
struct PBullet { x: f32, y: f32, vx: f32, vy: f32, kind: WeaponType, damage: i32, life: f32, size: f32, pierce: i32 }

#[derive(Clone)]
struct EBullet { x: f32, y: f32, vx: f32, vy: f32, damage: i32, life: f32, size: f32 }

#[derive(Clone)]
struct Asteroid {
    x: f32, y: f32, vx: f32, vy: f32, size: f32, rot: f32, rot_speed: f32,
    color: Color, hp: i32, max_hp: i32, kind: AsteroidKind, verts: Vec<Vec2>, split: bool,
}

#[derive(Clone)]
struct Enemy {
    x: f32, y: f32, vx: f32, vy: f32, size: f32, hp: i32, max_hp: i32,
    color: Color, shoot_timer: f32, angle: f32,
    template_id: usize, hit_flash: f32, regen: f32,
    // Decoded traits for fast access (set once at creation)
    e_size: EnemySize, e_move: EnemyMove, e_weapon: EnemyWeapon, e_special: EnemySpecial,
}

struct EnemyRender {
    name: &'static str, id: usize, color: Color,
}

struct Boss {
    x: f32, y: f32, hp: i32, max_hp: i32, size: f32,
    timer: f32, angle: f32, vx: f32, vy: f32,
    color: Color, kind: BossKind, pattern: i32, phase: i32, hit_flash: f32,
    moving_in: bool, bullet_cooldown: f32,
}

struct Particle { x: f32, y: f32, vx: f32, vy: f32, life: f32, max_life: f32, size: f32, color: Color, gravity: f32, glow: f32 }

#[derive(Clone)]
struct PowerUp { x: f32, y: f32, kind: i32, vy: f32 }

struct FloatingText { x: f32, y: f32, text: String, life: f32, color: Color, size: f32 }

struct Drone { x: f32, y: f32, shoot_timer: f32 }

// === HELPERS ===
fn hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0*l-1.0).abs()) * s;
    let x = c * (1.0 - ((h/60.0)%2.0-1.0).abs());
    let m = l - c/2.0;
    let (r,g,b) = match h as i32 % 360 {
        0..=59 => (c,x,0.0), 60..=119 => (x,c,0.0),
        120..=179 => (0.0,c,x), 180..=239 => (0.0,x,c),
        240..=299 => (x,0.0,c), _ => (c,0.0,x),
    };
    Color::new(r+m, g+m, b+m, 1.0)
}

fn ast_verts(size: f32) -> Vec<Vec2> {
    let mut rng = ::rand::thread_rng();
    let n = 6 + rng.gen_range(0..7);
    (0..n).map(|i| {
        let a = i as f32 / n as f32 * TAU;
        let r = size * (0.5 + rng.gen::<f32>() * 0.5);
        Vec2::new(a.cos()*r, a.sin()*r)
    }).collect()
}

fn make_asteroid(sw: f32, sh: f32, px: f32, py: f32, wave: i32, diff: f32) -> Asteroid {
    let mut rng = ::rand::thread_rng();
    let side = rng.gen_range(0..4);
    let (x,y) = match side {
        0 => (rng.gen::<f32>()*sw, -40.0), 1 => (sw+40.0, rng.gen::<f32>()*sh),
        2 => (rng.gen::<f32>()*sw, sh+40.0), _ => (-40.0, rng.gen::<f32>()*sh),
    };
    let (mut dx, mut dy) = (px-x, py-y);
    let d = (dx*dx+dy*dy).sqrt().max(1.0); (dx, dy) = (dx/d, dy/d);
    let spd = 50.0 + rng.gen::<f32>()*100.0 + diff*25.0;
    let sz = 14.0 + rng.gen::<f32>()*26.0 + (wave as f32).min(12.0);
    let kind = match rng.gen_range(0..10) { 0..=2 if wave>3 => AsteroidKind::Fire, 3..=4 if wave>7 => AsteroidKind::Ice, _ => AsteroidKind::Normal };
    let (cr,cg,cb) = match kind {
        AsteroidKind::Normal => (120+rng.gen_range(0..80), 90+rng.gen_range(0..60), 90+rng.gen_range(0..60)),
        AsteroidKind::Fire => (220, 80+rng.gen_range(0..60), 20),
        AsteroidKind::Ice => (80, 150+rng.gen_range(0..60), 220),
    };
    let hp = (sz/10.0).ceil() as i32 + (wave/3) + (diff*0.5) as i32;
    Asteroid { x,y, vx: dx*spd+rng.gen::<f32>()*40.0-20.0, vy: dy*spd+rng.gen::<f32>()*40.0-20.0,
        size: sz, rot: rng.gen::<f32>()*TAU, rot_speed: rng.gen::<f32>()*2.0-1.0,
        color: Color::from_rgba(cr,cg,cb,255), hp, max_hp: hp, kind, verts: ast_verts(sz), split: false }
}

fn split_asteroid(a: &Asteroid) -> Vec<Asteroid> {
    let mut rng = ::rand::thread_rng();
    let ns = a.size * (0.45 + rng.gen::<f32>()*0.15);
    if ns < 8.0 { return vec![]; }
    (0..2).map(|_| {
        let a2 = rng.gen::<f32>() * TAU;
        let spd = 70.0 + rng.gen::<f32>()*70.0;
        let nhp = (ns/10.0).ceil() as i32;
        Asteroid {
            x: a.x + rng.gen::<f32>()*10.0-5.0, y: a.y + rng.gen::<f32>()*10.0-5.0,
            vx: a.vx*0.3 + a2.cos()*spd, vy: a.vy*0.3 + a2.sin()*spd,
            size: ns, rot: rng.gen::<f32>()*TAU, rot_speed: rng.gen::<f32>()*2.0-1.0,
            color: a.color, hp: nhp, max_hp: nhp, kind: a.kind,
            verts: ast_verts(ns), split: true }
    }).collect()
}

// Generate enemy from template
fn make_enemy_from_template(sw: f32, wave: i32, diff: f32, template_id: usize, elite: bool, diff_mode_hp_mult: f32) -> Enemy {
    let mut rng = ::rand::thread_rng();
    let (sz, mv, wp, sp, hue) = template_meta(template_id);
    let (sm, spm, hpm, _gv) = template_stats(template_id);

    let base_size = 7.0 + rng.gen::<f32>()*3.0;
    let size = base_size * sm * if elite { 1.25 } else { 1.0 };
    let base_hp = 3 + wave;
    let hp = (base_hp as f32 * hpm * (1.0 + diff*0.25) * diff_mode_hp_mult) as i32 + 2;
    let spd = (30.0 + wave as f32*5.0 + diff*15.0) * spm;

    let (sr, sg, sb) = (hue + rng.gen::<f32>()*20.0 - 10.0, 0.7 + rng.gen::<f32>()*0.3, 0.4 + rng.gen::<f32>()*0.3);
    let base_col = hsl(sr, sg, sb);
    let col = if elite { Color::new(base_col.r*1.3, base_col.g*1.3, base_col.b*1.3, 1.0) } else { base_col };

    Enemy {
        x: rng.gen::<f32>()*sw, y: -40.0,
        vx: rng.gen::<f32>()*spd*0.4 - spd*0.2,
        vy: spd*0.5 + rng.gen::<f32>()*spd*0.5,
        size: size.max(4.0), hp: hp.max(1), max_hp: hp.max(1),
        color: col, shoot_timer: if elite { 0.3 + rng.gen::<f32>()*1.0 } else { 0.5 + rng.gen::<f32>()*2.0 },
        angle: 0.0, template_id, hit_flash: 0.0,
        regen: if sp == EnemySpecial::Regen { 0.5 + rng.gen::<f32>()*0.5 } else { 0.0 },
        e_size: sz, e_move: mv, e_weapon: wp, e_special: sp,
    }
}

fn make_enemy_random(sw: f32, wave: i32, diff: f32, elite: bool, diff_mode_hp_mult: f32) -> Enemy {
    let id = ::rand::thread_rng().gen_range(0..template_count());
    make_enemy_from_template(sw, wave, diff, id, elite, diff_mode_hp_mult)
}

fn mkparticles(x: f32, y: f32, color: Color, n: i32, spd: f32, grav: f32) -> Vec<Particle> {
    let mut rng = ::rand::thread_rng();
    (0..n).map(|_| {
        let a = rng.gen::<f32>() * TAU;
        let s = rng.gen::<f32>() * spd;
        Particle { x, y, vx: a.cos()*s, vy: a.sin()*s, life: 0.2+rng.gen::<f32>()*0.6, max_life: 1.0,
            size: 1.0+rng.gen::<f32>()*3.0, color, gravity: grav, glow: if rng.gen_bool(0.2) { 1.0 } else { 0.0 } }
    }).collect()
}

fn explode(x: f32, y: f32, c1: Color, c2: Color, c3: Color, n: i32) -> Vec<Particle> {
    let mut p = mkparticles(x, y, c1, n*3/5, 200.0, 10.0);
    p.extend(mkparticles(x, y, c2, n*2/5, 150.0, 5.0));
    p.extend(mkparticles(x, y, c3, n/5, 100.0, 0.0)); p
}

fn save_highscore(score: i64) {
    let current = std::fs::read_to_string(HIGHSCORE_FILE).ok().and_then(|d| d.trim().parse().ok()).unwrap_or(0);
    if score > current { let _ = std::fs::write(HIGHSCORE_FILE, score.to_string()); }
}
fn load_highscore() -> i64 {
    std::fs::read_to_string(HIGHSCORE_FILE).ok().and_then(|d| d.trim().parse().ok()).unwrap_or(0)
}

// === SHOP ===
struct ShopItem { name: &'static str, desc: &'static str, cost_base: i64, lv: &'static str, key: KeyCode }
const SHOP_ITEMS: [ShopItem; 8] = [
    ShopItem { name: "CAN", desc: "Max HP +1", cost_base: 50, lv: "shop_hp_lv", key: KeyCode::Key1 },
    ShopItem { name: "KALKAN", desc: "Kalkan +0.5", cost_base: 30, lv: "shop_shield_lv", key: KeyCode::Key2 },
    ShopItem { name: "HIZ", desc: "Hız +0.3", cost_base: 40, lv: "shop_speed_lv", key: KeyCode::Key3 },
    ShopItem { name: "SİLAH", desc: "Silah Seviye +1", cost_base: 100, lv: "shop_weapon_lv", key: KeyCode::Key4 },
    ShopItem { name: "DRONE", desc: "Drone +1", cost_base: 150, lv: "shop_drone_lv", key: KeyCode::Key5 },
    ShopItem { name: "HASAR", desc: "Hasar +2", cost_base: 80, lv: "shop_damage_lv", key: KeyCode::Key6 },
    ShopItem { name: "ATEŞ", desc: "Atış Hızı", cost_base: 120, lv: "shop_firerate_lv", key: KeyCode::Key7 },
    ShopItem { name: "ENERJİ", desc: "Max Enerji +20", cost_base: 60, lv: "shop_energy_lv", key: KeyCode::Key8 },
];
fn shop_cost(base: i64, level: i32) -> i64 { base + (level as i64) * base / 2 }
const SHOP_ITEMS_PAGE2: [ShopItem; 8] = [
    ShopItem { name: "HIZ II", desc: "Hız +0.5", cost_base: 80, lv: "shop_speed2_lv", key: KeyCode::Key1 },
    ShopItem { name: "ATEŞ II", desc: "Atış Hızı +0.02", cost_base: 180, lv: "shop_firerate2_lv", key: KeyCode::Key2 },
    ShopItem { name: "CAN REGEN", desc: "Sürekli Can +0.02/s", cost_base: 150, lv: "shop_regen_lv", key: KeyCode::Key3 },
    ShopItem { name: "DASH", desc: "Dash Bekleme -0.1s", cost_base: 100, lv: "shop_dash_lv", key: KeyCode::Key4 },
    ShopItem { name: "MERMI", desc: "Mermi Boyutu +1", cost_base: 100, lv: "shop_bullet_lv", key: KeyCode::Key5 },
    ShopItem { name: "KOMBO", desc: "Kombo Süresi +0.5s", cost_base: 80, lv: "shop_combo_lv", key: KeyCode::Key6 },
    ShopItem { name: "ALTIN", desc: "Öldürme Başına +2 Altın", cost_base: 120, lv: "shop_gold_lv", key: KeyCode::Key7 },
    ShopItem { name: "ENERJİ II", desc: "Energy Regen +2/s", cost_base: 100, lv: "shop_energy_regen_lv", key: KeyCode::Key8 },
];

// === MAIN ===
#[macroquad::main("Yıldız Avcısı")]
async fn main() {
    let mut state = GameState::Menu;
    let mut shop_page = 0i32;
    let mut player = Player {
        x: 0.0, y: 0.0, angle: 0.0, speed: 3.0,
        hp: 3, max_hp: 3, shield: 1.0, max_shield: 1.0,
        energy: 100.0, max_energy: 100.0,
        weapon: WeaponType::Normal, weapon_level: 1, drones: 0,
        invincible: 0.0, dash_cooldown: 0.0,
        combo: 0, combotimer: 0.0, kills: 0, score: 0,
        gold: 0, damage_bonus: 0, fire_rate_bonus: 0.0,
        hp_regen: 0.0, hp_regen_accum: 0.0, dash_reduction: 0.0, bullet_size: 0.0, combo_bonus: 0.0,
        shop_hp_lv: 0, shop_shield_lv: 0, shop_speed_lv: 0,
        shop_weapon_lv: 0, shop_drone_lv: 0,
        shop_damage_lv: 0, shop_firerate_lv: 0, shop_energy_lv: 0,
        shop_speed2_lv: 0, shop_firerate2_lv: 0, shop_regen_lv: 0,
        shop_dash_lv: 0, shop_bullet_lv: 0, shop_combo_lv: 0,
        shop_gold_lv: 0, shop_energy_regen_lv: 0,
    };
    let mut pb: Vec<PBullet> = Vec::new();
    let mut eb: Vec<EBullet> = Vec::new();
    let mut asteroids: Vec<Asteroid> = Vec::new();
    let mut enemies: Vec<Enemy> = Vec::new();
    let mut particles: Vec<Particle> = Vec::new();
    let mut powerups: Vec<PowerUp> = Vec::new();
    let mut stars: Vec<Star> = Vec::new();
    let mut floating: Vec<FloatingText> = Vec::new();
    let mut drones: Vec<Drone> = Vec::new();
    let mut boss: Option<Boss> = None;
    let mut highscore = load_highscore();
    let mut wave = 1;
    let mut wave_timer = 2.0;
    let mut wave_announce = 0.0;
    let mut boss_killed = 0;
    let mut shake: f32 = 0.0;
    let mut flash: f32 = 0.0;
    let mut cooldown = 0.0;
    let mut weapon_switch = 0.0;
    let mut difficulty;
    let mut diff_mode = DiffMode::Normal;

    loop {
        let dt = get_frame_time().min(0.05);
        let (sw, sh) = (screen_width(), screen_height());

        while stars.len() < 220 {
            let mut rng = ::rand::thread_rng();
            stars.push(Star {
                x: rng.gen::<f32>()*sw, y: rng.gen::<f32>()*sh,
                speed: 5.0+rng.gen::<f32>()*45.0, size: 0.3+rng.gen::<f32>()*1.8,
                brightness: 0.2+rng.gen::<f32>()*0.8, layer: rng.gen::<f32>(),
            });
        }

        match state {
            GameState::Menu => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left) {
                    state = GameState::DiffSelect;
                }
                let t = get_time() as f32;
                clear_background(Color::from_rgba(3,3,15,255));
                for s in &stars { let b = s.brightness; draw_circle(s.x, s.y, s.size*(0.5+s.layer*0.5), Color::new(b,b*0.9,b,0.7)); }
                let ti = "YILDIZ AVCISI";
                draw_text(ti, sw/2.0-measure_text(ti,None,64,1.0).width/2.0, sh*0.25, 64.0, Color::new(0.2,0.5,(t*3.0).sin()*0.3+0.7,1.0));
                let sub = "STAR HUNTER";
                draw_text(sub, sw/2.0-measure_text(sub,None,28,1.0).width/2.0, sh*0.25+68.0, 28.0, Color::new(0.5,0.8,1.0,0.6));
                let cmds = ["WASD/OKLAR - HAREKET", "SOL TIK/SPACE - ATEŞ", "1/2/3/4 - SİLAH SEÇ (ENERJİ)", "SHIFT/SAĞ TIK - DASH", "TAB - STATLAR", "MAĞAZA İÇİN ESC (DALGA ARASI)"];
                for (i,c) in cmds.iter().enumerate() { let cw = measure_text(c,None,17,1.0).width; draw_text(c, sw/2.0-cw/2.0, sh*0.45+i as f32*26.0, 17.0, Color::new(0.5,0.6,0.8,0.8)); }
                let s = "BAŞLAMAK İÇİN SPACE'E BAS";
                let b = if (t*2.0).sin()>0.0{1.0}else{0.2};
                draw_text(s, sw/2.0-measure_text(s,None,24,1.0).width/2.0, sh*0.75, 24.0, Color::new(1.0,1.0,1.0,b));
                if highscore > 0 { let hs = format!("EN İYİ: {}", highscore); draw_text(&hs, sw/2.0-measure_text(&hs,None,22,1.0).width/2.0, sh*0.85, 22.0, Color::new(0.8,0.7,0.3,0.8)); }
                next_frame().await; continue;
            }

            GameState::DiffSelect => {
                if is_key_pressed(KeyCode::Key1) || is_key_pressed(KeyCode::Key2) || is_key_pressed(KeyCode::Key3) {
                    diff_mode = if is_key_pressed(KeyCode::Key1) { DiffMode::Easy }
                        else if is_key_pressed(KeyCode::Key2) { DiffMode::Normal }
                        else { DiffMode::Hard };
                    state = GameState::Playing; wave = 1; wave_timer = 1.0; wave_announce = 2.0;
                    let sx = sw/2.0; let sy = sh*0.8;
                    let (shp, smhp, ssh, mssh) = match diff_mode {
                        DiffMode::Easy => (6, 6, 2.0, 2.0),
                        DiffMode::Normal => (3, 3, 1.0, 1.0),
                        DiffMode::Hard => (2, 2, 0.5, 0.5),
                    };
                    shop_page = 0;
                    player = Player {
                        x: sx, y: sy, angle: 0.0, speed: 3.0, hp: shp, max_hp: smhp, shield: ssh, max_shield: mssh,
                        energy: 100.0, max_energy: 100.0, weapon: WeaponType::Normal, weapon_level: 1, drones: 0,
                        invincible: 2.0, dash_cooldown: 0.0, combo: 0, combotimer: 0.0, kills: 0, score: 0,
                        gold: 0, damage_bonus: 0, fire_rate_bonus: 0.0,
                        hp_regen: 0.0, hp_regen_accum: 0.0, dash_reduction: 0.0, bullet_size: 0.0, combo_bonus: 0.0,
                        shop_hp_lv: 0, shop_shield_lv: 0, shop_speed_lv: 0,
                        shop_weapon_lv: 0, shop_drone_lv: 0, shop_damage_lv: 0, shop_firerate_lv: 0, shop_energy_lv: 0,
                        shop_speed2_lv: 0, shop_firerate2_lv: 0, shop_regen_lv: 0,
                        shop_dash_lv: 0, shop_bullet_lv: 0, shop_combo_lv: 0,
                        shop_gold_lv: 0, shop_energy_regen_lv: 0,
                    };
                    pb.clear(); eb.clear(); asteroids.clear(); enemies.clear();
                    particles.clear(); powerups.clear(); drones.clear();
                    boss = None; shake = 0.0; flash = 0.0; boss_killed = 0; difficulty = 1.0;
                }
                let t = get_time() as f32;
                clear_background(Color::from_rgba(3,3,15,255));
                for s in &stars { let b = s.brightness; draw_circle(s.x, s.y, s.size*(0.5+s.layer*0.5), Color::new(b,b*0.9,b,0.7)); }
                draw_text("ZORLUK SEÇ", sw/2.0-measure_text("ZORLUK SEÇ",None,50,1.0).width/2.0, sh*0.3, 50.0, Color::new(0.2,0.5,(t*3.0).sin()*0.3+0.7,1.0));
                let opts = ["[1] KOLAY (6 CAN, 2 KALKAN)", "[2] NORMAL (3 CAN, 1 KALKAN)", "[3] ZOR (2 CAN, 0.5 KALKAN)"];
                for (i, opt) in opts.iter().enumerate() {
                    let col = match (i, t) {
                        (0, _) => Color::new(0.3,1.0,0.3,0.9),
                        (1, _) => Color::new(1.0,0.8,0.3,0.9),
                        (_, _) => Color::new(1.0,0.3,0.3,0.9),
                    };
                    let b = ((t*2.0+ i as f32).sin()*0.3+0.7) as f32;
                    draw_text(opt, sw/2.0-measure_text(opt,None,22,1.0).width/2.0, sh*0.5+i as f32*40.0, 22.0, Color::new(col.r* b, col.g*b, col.b*b, b));
                }
                draw_text("GERİ İÇİN ESC", sw/2.0-measure_text("GERİ İÇİN ESC",None,16,1.0).width/2.0, sh*0.85, 16.0, Color::new(0.5,0.5,0.7,0.6));
                if is_key_pressed(KeyCode::Escape) { state = GameState::Menu; }
                next_frame().await; continue;
            }

            GameState::Shopping => {
                for s in &mut stars { s.y += s.speed*dt*0.2; if s.y>sh { s.y=-2.0; s.x=::rand::thread_rng().gen::<f32>()*sw; } }
                clear_background(Color::from_rgba(5,5,20,255));
                for s in &stars { let b=s.brightness; draw_circle(s.x,s.y,s.size*(0.3+s.layer*0.7),Color::new(b,b*0.9,b,0.8)); }
                let _t = get_time() as f32;
                draw_text("MAĞAZA", sw/2.0-measure_text("MAĞAZA",None,50,1.0).width/2.0, 60.0, 50.0, Color::new(0.3,0.8,1.0,1.0));
                let info = format!("ALTIN: {} | DALGA: {} | SKOR: {}", player.gold, wave-1, player.score);
                draw_text(&info, sw/2.0-measure_text(&info,None,18,1.0).width/2.0, 90.0, 18.0, Color::new(0.7,0.8,1.0,0.8));
                let stats = format!("CAN: {}/{} KALKAN: {:.1}/{} HIZ: {} SİLAH: LV{} DRONE: {} HASAR: +{} ENERJİ: {:.0}/{}",
                    player.hp, player.max_hp, player.shield, player.max_shield,
                    player.speed, player.weapon_level, player.drones, player.damage_bonus, player.energy, player.max_energy);
                draw_text(&stats, sw/2.0-measure_text(&stats,None,13,1.0).width/2.0, 115.0, 13.0, Color::new(0.6,0.7,0.8,0.7));
                draw_text("Çıkmak için ESC", sw/2.0-measure_text("Çıkmak için ESC",None,14,1.0).width/2.0, 145.0, 14.0, Color::new(0.5,0.5,0.7,0.6));
                // Page switching
                if is_key_pressed(KeyCode::Q) { shop_page = 0; }
                if is_key_pressed(KeyCode::E) { shop_page = 1; }
                let _cur_page = if shop_page==0 { "1/2" } else { "2/2" };
                let page_key = format!("SAYFA: [Q]{} [E]{}", if shop_page==0{"*"}else{" "}, if shop_page==1{"*"}else{" "});
                draw_text(&page_key, sw/2.0+100.0, 145.0, 14.0, Color::new(0.5,0.7,1.0,0.7));
                let (items, item_names, item_vals, item_max) = if shop_page==0 {
                    let names = ["[1] CAN +1", "[2] KALKAN +0.5", "[3] HIZ +0.3", "[4] SİLAH SEVİYE",
                        "[5] DRONE +1", "[6] HASAR +2", "[7] ATIS HIZI", "[8] ENERJİ +20"];
                    let vals = [
                        player.shop_hp_lv, player.shop_shield_lv, player.shop_speed_lv,
                        player.shop_weapon_lv, player.shop_drone_lv, player.shop_damage_lv,
                        player.shop_firerate_lv, player.shop_energy_lv,
                    ];
                    let max = [10,10,8,5,5,8,8,10];
                    (&SHOP_ITEMS[..], names, vals, max)
                } else {
                    let names = ["[1] HIZ +0.5", "[2] ATIS HIZI +0.02", "[3] CAN REGEN", "[4] DASH CD -0.1s",
                        "[5] MERMI BOYUT +1", "[6] KOMBO +0.5s", "[7] ALTIN BONUS", "[8] ENERJİ REGEN +2/s"];
                    let vals = [
                        player.shop_speed2_lv, player.shop_firerate2_lv, player.shop_regen_lv,
                        player.shop_dash_lv, player.shop_bullet_lv, player.shop_combo_lv,
                        player.shop_gold_lv, player.shop_energy_regen_lv,
                    ];
                    let max = [5,5,5,5,5,5,5,5];
                    (&SHOP_ITEMS_PAGE2[..], names, vals, max)
                };
                let start_y = 175.0;
                for i in 0..8 {
                    let y = start_y + i as f32 * 36.0;
                    let maxed = item_vals[i] >= item_max[i];
                    let cost = if maxed { 0 } else { shop_cost(items[i].cost_base, item_vals[i]) };
                    let can_afford = player.gold >= cost && !maxed;
                    let col = if maxed { Color::new(0.4,0.4,0.4,0.5) } else if can_afford { Color::new(0.3,0.8,1.0,0.9) } else { Color::new(0.6,0.5,0.5,0.6) };
                    let buy = if maxed { "MAX" } else { &format!("{} ALTIN", cost) };
                    let desc = format!("{} (LV{}/{})", item_names[i], item_vals[i], item_max[i]);
                    let hcol = if is_key_down(items[i].key) { Color::new(1.0,1.0,1.0,0.3) } else { Color::new(0.0,0.0,0.0,0.0) };
                    draw_rectangle(sw/2.0-220.0, y-2.0, 440.0, 30.0, hcol);
                    draw_text(&desc, sw/2.0-210.0, y+18.0, 16.0, col);
                    draw_text(buy, sw/2.0+140.0, y+18.0, 16.0, col);
                    if !maxed {
                        let pw = 80.0 * (item_vals[i] as f32 / item_max[i] as f32);
                        draw_rectangle(sw/2.0-210.0, y+24.0, 80.0, 3.0, Color::new(0.2,0.2,0.2,0.4));
                        draw_rectangle(sw/2.0-210.0, y+24.0, pw, 3.0, Color::new(0.7,0.7,0.3,0.5));
                    }
                }
                if is_key_pressed(KeyCode::Escape) { state = GameState::Playing; wave_timer = 2.0; }
                // Page 1 purchases
                if shop_page == 0 {
                    let pv = [player.shop_hp_lv, player.shop_shield_lv, player.shop_speed_lv,
                        player.shop_weapon_lv, player.shop_drone_lv, player.shop_damage_lv,
                        player.shop_firerate_lv, player.shop_energy_lv];
                    let pm = [10,10,8,5,5,8,8,10];
                    for i in 0..8 {
                        if !(pv[i] >= pm[i]) && player.gold >= shop_cost(SHOP_ITEMS[i].cost_base, pv[i]) && is_key_pressed(SHOP_ITEMS[i].key) {
                            player.gold -= shop_cost(SHOP_ITEMS[i].cost_base, pv[i]);
                            match i {
                                0 => { player.max_hp += 1; player.hp = player.hp.min(player.max_hp); player.shop_hp_lv += 1; }
                                1 => { player.max_shield += 0.5; player.shop_shield_lv += 1; }
                                2 => { player.speed += 0.3; player.shop_speed_lv += 1; }
                                3 => { player.weapon_level = (player.weapon_level+1).min(5); player.shop_weapon_lv += 1; }
                                4 => { player.drones = (player.drones+1).min(5); player.shop_drone_lv += 1; }
                                5 => { player.damage_bonus += 2; player.shop_damage_lv += 1; }
                                6 => { player.fire_rate_bonus += 0.015; player.shop_firerate_lv += 1; }
                                7 => { player.max_energy += 20.0; player.shop_energy_lv += 1; }
                                _ => {}
                            }
                        }
                    }
                }
                // Page 2 purchases
                if shop_page == 1 {
                    let pv = [player.shop_speed2_lv, player.shop_firerate2_lv, player.shop_regen_lv,
                        player.shop_dash_lv, player.shop_bullet_lv, player.shop_combo_lv,
                        player.shop_gold_lv, player.shop_energy_regen_lv];
                    let pm = [5,5,5,5,5,5,5,5];
                    for i in 0..8 {
                        if !(pv[i] >= pm[i]) && player.gold >= shop_cost(SHOP_ITEMS_PAGE2[i].cost_base, pv[i]) && is_key_pressed(SHOP_ITEMS_PAGE2[i].key) {
                            player.gold -= shop_cost(SHOP_ITEMS_PAGE2[i].cost_base, pv[i]);
                            match i {
                                0 => { player.speed += 0.5; player.shop_speed2_lv += 1; }
                                1 => { player.fire_rate_bonus += 0.02; player.shop_firerate2_lv += 1; }
                                2 => { player.hp_regen += 0.02; player.shop_regen_lv += 1; }
                                3 => { player.dash_reduction += 0.1; player.shop_dash_lv += 1; }
                                4 => { player.bullet_size += 1.0; player.shop_bullet_lv += 1; }
                                5 => { player.combo_bonus += 0.5; player.shop_combo_lv += 1; }
                                6 => { player.shop_gold_lv += 1; }
                                7 => { player.shop_energy_regen_lv += 1; }
                                _ => {}
                            }
                        }
                    }
                }
                next_frame().await; continue;
            }

            GameState::Paused => {
                clear_background(Color::from_rgba(3,3,15,200));
                draw_text("DURAKLATILDI", sw/2.0-measure_text("DURAKLATILDI",None,50,1.0).width/2.0, sh/2.0-30.0, 50.0, Color::new(1.0,1.0,1.0,0.9));
                draw_text("DEVAM İÇİN ESC", sw/2.0-measure_text("DEVAM İÇİN ESC",None,22,1.0).width/2.0, sh/2.0+20.0, 22.0, Color::new(0.6,0.8,1.0,0.7));
                if is_key_pressed(KeyCode::Escape) { state = GameState::Playing; }
                next_frame().await; continue;
            }

            GameState::GameOver => {
                shake *= 0.95;
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left) { state = GameState::Menu; }
                let t = get_time() as f32;
                for p in &mut particles { p.x+=p.vx*dt; p.y+=p.vy*dt; p.vx*=0.96; p.vy*=0.96; p.life-=dt; p.vy+=p.gravity*dt; }
                particles.retain(|p| p.life>0.0);
                for s in &mut stars { s.y+=s.speed*dt*0.2; if s.y>sh { s.y=-2.0; s.x=::rand::thread_rng().gen::<f32>()*sw; } }
                clear_background(Color::from_rgba(3,3,12,255));
                for s in &stars { let b=s.brightness; draw_circle(s.x,s.y,s.size,Color::new(b,b*0.9,b,0.8)); }
                for p in &particles { let a=(p.life/p.max_life).max(0.0); draw_circle(p.x,p.y,p.size*a,Color::new(p.color.r,p.color.g,p.color.b,a)); }
                draw_text("OYUN BİTTİ", sw/2.0-measure_text("OYUN BİTTİ",None,60,1.0).width/2.0, sh*0.2, 60.0, Color::new(1.0,0.2,0.2,1.0));
                let st = format!("SKOR: {}  DALGA: {}  ÖLDÜRME: {}", player.score, wave-1, player.kills);
                draw_text(&st, sw/2.0-measure_text(&st,None,24,1.0).width/2.0, sh*0.32, 24.0, Color::new(1.0,1.0,1.0,0.9));
                let gt = format!("KALAN ALTIN: {}", player.gold);
                draw_text(&gt, sw/2.0-measure_text(&gt,None,20,1.0).width/2.0, sh*0.40, 20.0, Color::new(1.0,0.8,0.3,0.8));
                if player.score==highscore && player.score>0 {
                    let nr="YENİ REKOR!"; let b=(t*3.0).sin()*0.3+0.7;
                    draw_text(nr, sw/2.0-measure_text(nr,None,32,1.0).width/2.0, sh*0.50, 32.0, Color::new(1.0,0.8,0.2,b));
                } else { let hs=format!("EN İYİ: {}", highscore); draw_text(&hs, sw/2.0-measure_text(&hs,None,24,1.0).width/2.0, sh*0.50, 24.0, Color::new(0.8,0.7,0.3,0.8)); }
                let r="TEKRAR İÇİN SPACE'E BAS";
                let b=if (t*2.0).sin()>0.0{1.0}else{0.3};
                draw_text(r, sw/2.0-measure_text(r,None,20,1.0).width/2.0, sh*0.65, 20.0, Color::new(1.0,1.0,1.0,b));
                next_frame().await; continue;
            }

            GameState::Playing => {
                if is_key_pressed(KeyCode::Escape) { state = GameState::Paused; next_frame().await; continue; }

                if player.invincible > 0.0 { player.invincible -= dt; }
                if player.dash_cooldown > 0.0 { player.dash_cooldown -= dt; }
                if player.combotimer > 0.0 { player.combotimer -= dt; } else { player.combo = 0; }                shake *= 0.92; flash *= 0.9; cooldown -= dt; weapon_switch -= dt;
                if player.hp_regen > 0.0 && player.hp < player.max_hp { player.hp_regen_accum += player.hp_regen * dt; while player.hp_regen_accum >= 1.0 { player.hp = (player.hp+1).min(player.max_hp); player.hp_regen_accum -= 1.0; } }

                player.energy = (player.energy + dt*(match diff_mode {DiffMode::Easy=>8.0,DiffMode::Normal=>6.0,DiffMode::Hard=>4.0} + player.shop_energy_regen_lv as f32*2.0)).min(player.max_energy);
                player.shield = (player.shield + dt*(match diff_mode {DiffMode::Easy=>0.25,DiffMode::Normal=>0.15,DiffMode::Hard=>0.08})).min(player.max_shield);
                difficulty = 1.0 + player.gold as f32 / 150.0 + wave as f32 * 0.6;
                let diff_mult = match diff_mode { DiffMode::Easy => 0.6, DiffMode::Normal => 1.0, DiffMode::Hard => 1.5 };
                let diff_player_dmg_mult = match diff_mode { DiffMode::Easy => 1.3, DiffMode::Normal => 1.0, DiffMode::Hard => 0.7 };
                let diff_spd_mult = match diff_mode { DiffMode::Easy => 0.8, DiffMode::Normal => 1.0, DiffMode::Hard => 1.3 };
                let diff_mode_hp_mult = match diff_mode { DiffMode::Easy => 0.8, DiffMode::Normal => 1.2, DiffMode::Hard => 2.0 };
                let diff_enemy_count_mult = match diff_mode { DiffMode::Easy => 0.7, DiffMode::Normal => 1.0, DiffMode::Hard => 1.5 };

                while drones.len() < player.drones as usize && drones.len() < 5 {
                    drones.push(Drone { x: player.x, y: player.y, shoot_timer: 0.0 });
                }

                let (mut dx, mut dy) = (0.0, 0.0);
                if is_key_down(KeyCode::A)||is_key_down(KeyCode::Left) { dx-=1.0; }
                if is_key_down(KeyCode::D)||is_key_down(KeyCode::Right) { dx+=1.0; }
                if is_key_down(KeyCode::W)||is_key_down(KeyCode::Up) { dy-=1.0; }
                if is_key_down(KeyCode::S)||is_key_down(KeyCode::Down) { dy+=1.0; }
                if dx!=0.0&&dy!=0.0 { let inv=FRAC_1_SQRT_2; dx*=inv; dy*=inv; }

                if (is_key_pressed(KeyCode::LeftShift)||is_key_pressed(KeyCode::RightShift)||is_mouse_button_pressed(MouseButton::Right))
                    && player.dash_cooldown<=0.0 && (dx!=0.0||dy!=0.0) {
                    player.x+=dx*220.0; player.y+=dy*220.0;
                    player.x=player.x.clamp(PLAYER_SIZE, sw-PLAYER_SIZE);
                    player.y=player.y.clamp(PLAYER_SIZE, sh-PLAYER_SIZE);
                    player.dash_cooldown=(1.0 - player.dash_reduction).max(0.3); player.invincible=0.08; shake=3.0;
                    particles.extend(mkparticles(player.x,player.y,Color::new(0.3,0.6,1.0,0.8),15,150.0,0.0));
                }
                let ms = 220.0+player.speed*25.0;
                player.x += dx*ms*dt; player.y += dy*ms*dt;
                player.x = player.x.clamp(PLAYER_SIZE, sw-PLAYER_SIZE);
                player.y = player.y.clamp(PLAYER_SIZE, sh-PLAYER_SIZE);
                let (mx,my) = mouse_position();
                player.angle = (my-player.y).atan2(mx-player.x);

                if weapon_switch <= 0.0 {
                    if is_key_pressed(KeyCode::Key1) { player.weapon=WeaponType::Normal; weapon_switch=0.3; }
                    if is_key_pressed(KeyCode::Key2)&&player.energy>=15.0 { player.weapon=WeaponType::Spread; weapon_switch=0.3; }
                    if is_key_pressed(KeyCode::Key3)&&player.energy>=25.0 { player.weapon=WeaponType::Homing; weapon_switch=0.3; }
                    if is_key_pressed(KeyCode::Key4)&&player.energy>=40.0 { player.weapon=WeaponType::Laser; weapon_switch=0.3; }
                }

                let max_pb = 40 + player.weapon_level as usize;
                if (is_key_down(KeyCode::Space)||is_mouse_button_down(MouseButton::Left)) && cooldown<=0.0 && pb.len()<max_pb {
                    let pa = player.angle;
                    let dmg_base = ((8 + player.weapon_level*2 + player.damage_bonus) as f32 * diff_player_dmg_mult) as i32;
                    match player.weapon {
                        WeaponType::Normal => {
                            pb.push(PBullet { x: player.x+pa.cos()*PLAYER_SIZE, y: player.y+pa.sin()*PLAYER_SIZE,
                                vx: pa.cos()*650.0, vy: pa.sin()*650.0, kind: WeaponType::Normal,
                                damage: dmg_base, life: 2.5, size: 3.0+player.bullet_size, pierce: 0 });
                            cooldown = (0.09 - difficulty*0.004 - player.fire_rate_bonus).max(0.035);
                        }
                        WeaponType::Spread => {
                            player.energy -= 2.0;
                            for i in -2..=2 {
                                let a = pa + i as f32*0.12;
                                pb.push(PBullet { x: player.x+a.cos()*PLAYER_SIZE, y: player.y+a.sin()*PLAYER_SIZE,
                                    vx: a.cos()*550.0, vy: a.sin()*550.0, kind: WeaponType::Spread,
                                    damage: dmg_base*2/3+1, life: 2.0, size: 2.5+player.bullet_size, pierce: 0 });
                            }
                            cooldown = (0.22 - player.fire_rate_bonus*8.0).max(0.12);
                            if player.energy<=0.0 { player.weapon=WeaponType::Normal; }
                        }
                        WeaponType::Homing => {
                            player.energy -= 3.0;
                            pb.push(PBullet { x: player.x+pa.cos()*PLAYER_SIZE, y: player.y+pa.sin()*PLAYER_SIZE,
                                vx: pa.cos()*450.0, vy: pa.sin()*450.0, kind: WeaponType::Homing,
                                damage: dmg_base*2, life: 3.5, size: 5.0+player.bullet_size, pierce: 1 });
                            cooldown = (0.28 - player.fire_rate_bonus*8.0).max(0.15);
                            if player.energy<=0.0 { player.weapon=WeaponType::Normal; }
                        }
                        WeaponType::Laser => {
                            player.energy -= 5.0;
                            pb.push(PBullet { x: player.x+pa.cos()*PLAYER_SIZE, y: player.y+pa.sin()*PLAYER_SIZE,
                                vx: pa.cos()*800.0, vy: pa.sin()*800.0, kind: WeaponType::Laser,
                                damage: dmg_base*3, life: 1.8, size: 6.0+player.bullet_size, pierce: 3 });
                            cooldown = (0.4 - player.fire_rate_bonus*10.0).max(0.22);
                            shake=2.0;
                            if player.energy<=0.0 { player.weapon=WeaponType::Normal; }
                        }
                    }
                }

                // Drones AI
                let dc = drones.len();
                for (i, drone) in drones.iter_mut().enumerate() {
                    let ta = player.angle + PI + (i as f32/dc as f32)*TAU*0.5;
                    let td = 35.0+i as f32*5.0;
                    let (tx, ty) = (player.x+ta.cos()*td, player.y+ta.sin()*td);
                    drone.x += (tx-drone.x)*5.0*dt;
                    drone.y += (ty-drone.y)*5.0*dt;
                    drone.shoot_timer -= dt;
                    if drone.shoot_timer <= 0.0 {
                        let mut tgt = None; let mut bd = 999999.0;
                        for a in &asteroids { let d = (a.x-drone.x).hypot(a.y-drone.y); if d < bd { bd=d; tgt=Some((a.x,a.y)); } }
                        if tgt.is_none() { for e in &enemies { let d = (e.x-drone.x).hypot(e.y-drone.y); if d < bd { bd=d; tgt=Some((e.x,e.y)); } } }
                        if let Some((tx,ty)) = tgt {
                            let a = (ty-drone.y).atan2(tx-drone.x);
                            pb.push(PBullet { x: drone.x, y: drone.y, vx: a.cos()*350.0, vy: a.sin()*350.0,
                                kind: WeaponType::Normal, damage: 4+player.weapon_level+player.damage_bonus/2, life: 2.0, size: 2.0, pierce: 0 });
                            drone.shoot_timer = 0.5;
                        }
                    }
                }

                // Wave system
                wave_timer -= dt;
                if wave_timer <= 0.0 && boss.is_none() {
                    wave_announce = 2.5;
                    let count = ((8 + wave*2) as f32 * diff_enemy_count_mult).min(20.0) as i32;
                    for _ in 0..count { asteroids.push(make_asteroid(sw, sh, player.x, player.y, wave, difficulty * diff_mode_hp_mult)); }
                    if wave > 1 {
                        let ecount = ((wave/2 + 2) as f32 * diff_enemy_count_mult) as i32;
                        for _ in 0..ecount {
                            let elite = wave > 3 && ::rand::thread_rng().gen::<f32>() < (0.1 + wave as f32*0.02);
                            let e = make_enemy_random(sw, wave, difficulty, elite, diff_mode_hp_mult);
                            enemies.push(e);
                        }
                    }
                    // Boss every 3 waves
                    if wave % 3 == 0 {
                        let mut rng = ::rand::thread_rng();
                        let bkind = match wave {
                            w if w%9==0 => BossKind::Triplet,
                            w if w%6==0 => BossKind::Tank,
                            w if w%3==0 && rng.gen_bool(0.5) => BossKind::Barrage,
                            _ => BossKind::Default,
                        };
                        let (hp_mul, sz_mul) = match bkind {
                            BossKind::Tank => (2.0, 1.5), BossKind::Barrage => (1.2, 0.9),
                            BossKind::Triplet => (1.0, 0.8), BossKind::Default => (1.0, 1.0),
                        };
                        boss = Some(Boss {
                            x: sw/2.0, y: -60.0,
                            hp: ((200.0 + wave as f32*80.0 + boss_killed as f32*150.0) * hp_mul * diff_mode_hp_mult) as i32,
                            max_hp: ((200.0 + wave as f32*80.0 + boss_killed as f32*150.0) * hp_mul * diff_mode_hp_mult) as i32,
                            size: 45.0 + wave as f32*4.0 * sz_mul, timer: 0.0, angle: 0.0, vx: 0.0, vy: 30.0,
                            color: match bkind {
                                BossKind::Tank => Color::new(0.6,0.2,0.8,1.0), BossKind::Barrage => Color::new(0.9,0.3,0.1,1.0),
                                BossKind::Triplet => Color::new(0.2,0.6,0.9,1.0), BossKind::Default => hsl(rng.gen_range(0.0..360.0), 0.9, 0.5),
                            }, kind: bkind, pattern: rng.gen_range(0..4), phase: 0,
                            hit_flash: 0.0, moving_in: true, bullet_cooldown: 0.0,
                        });
                        floating.push(FloatingText { x: sw/2.0, y: sh*0.3, text: "BOSS GELİYOR!".to_string(), life: 3.0, color: Color::new(1.0,0.2,0.1,1.0), size: 40.0 });
                    }
                    wave += 1;
                    let rest = wave % 3 == 1 && wave > 2;
                    wave_timer = if rest { 15.0 } else { 8.0 + wave as f32 };
                    wave_timer = wave_timer.min(30.0);
                }

                // Update player bullets
                for b in &mut pb {
                    b.x += b.vx*dt; b.y += b.vy*dt; b.life -= dt;
                    if b.kind == WeaponType::Homing {
                        let mut tgt = None; let mut bd = 999999.0;
                        for a in &asteroids { let d=(a.x-b.x).hypot(a.y-b.y); if d<bd { bd=d; tgt=Some((a.x,a.y)); } }
                        if tgt.is_none() { for e in &enemies { let d=(e.x-b.x).hypot(e.y-b.y); if d<bd { bd=d; tgt=Some((e.x,e.y)); } } }
                        if let Some((tx,ty)) = tgt {
                            let ta = (ty-b.y).atan2(tx-b.x);
                            let ba = b.vy.atan2(b.vx);
                            let diff = (ta-ba+PI)%TAU-PI;
                            let na = ba + diff*4.0*dt;
                            let spd = (b.vx*b.vx+b.vy*b.vy).sqrt();
                            b.vx = na.cos()*spd; b.vy = na.sin()*spd;
                        }
                    }
                }
                pb.retain(|b| b.x>-60.0&&b.x<sw+60.0&&b.y>-60.0&&b.y<sh+60.0&&b.life>0.0);
                for b in &mut eb { b.x+=b.vx*dt; b.y+=b.vy*dt; b.life-=dt; }
                eb.retain(|b| b.x>-60.0&&b.x<sw+60.0&&b.y>-60.0&&b.y<sh+60.0&&b.life>0.0);
                for a in &mut asteroids { a.x+=a.vx*dt; a.y+=a.vy*dt; a.rot+=a.rot_speed*dt; }

                // Update enemies (per-template AI)
                for e in &mut enemies {
                    e.hit_flash *= 0.92;
                    let (dx, dy) = (player.x-e.x, player.y-e.y);
                    let d = (dx*dx+dy*dy).sqrt().max(1.0);
                    e.angle = dy.atan2(dx);

                    // Regen special
                    if e.e_special == EnemySpecial::Regen && e.hp < e.max_hp {
                        e.hp = (e.hp as f32 + e.regen * dt) as i32 + 1;
                        e.hp = e.hp.min(e.max_hp);
                    }

                    match e.e_move {
                        EnemyMove::Chase => {
                            e.vx += (dx/d)*(25.0+difficulty*8.0)*dt;
                            e.vy += (dy/d)*(35.0+difficulty*8.0)*dt;
                            e.vx *= 0.97; e.vy *= 0.97;
                            e.x += e.vx*dt; e.y += e.vy*dt;
                        }
                        EnemyMove::Strafe => {
                            let ideal = 180.0;
                            let (tx, ty) = (player.x - dx/d*ideal, player.y - dy/d*ideal);
                            e.vx += (tx-e.x)*2.0*dt + (dy/d)*40.0*dt;
                            e.vy += (ty-e.y)*2.0*dt + (-dx/d)*40.0*dt;
                            e.vx *= 0.95; e.vy *= 0.95;
                            e.x += e.vx*dt; e.y += e.vy*dt;
                        }
                        EnemyMove::Zigzag => {
                            let t = get_time() as f32;
                            e.vx += (dx/d)*(20.0+difficulty*6.0)*dt + (t*4.0+e.x*0.005).sin()*120.0*dt;
                            e.vy += (dy/d)*(35.0+difficulty*8.0)*dt;
                            e.vx *= 0.95; e.vy *= 0.95;
                            e.x += e.vx*dt; e.y += e.vy*dt;
                        }
                        EnemyMove::Orbit => {
                            let orbit_d = 150.0;
                            let ta = dy.atan2(dx) + PI/2.0;
                            e.vx += (dx/orbit_d + ta.cos())*40.0*dt;
                            e.vy += (dy/orbit_d + ta.sin())*40.0*dt;
                            e.vx *= 0.96; e.vy *= 0.96;
                            e.x += e.vx*dt; e.y += e.vy*dt;
                        }
                        EnemyMove::Rush => {
                            e.vx += (dx/d)*(50.0+difficulty*12.0)*dt;
                            e.vy += (dy/d)*(65.0+difficulty*12.0)*dt;
                            e.vx *= 0.96; e.vy *= 0.96;
                            e.x += e.vx*dt; e.y += e.vy*dt;
                        }
                    }

                    // Shooting based on weapon trait
                    e.shoot_timer -= dt;
                    if e.shoot_timer <= 0.0 && e.y > 0.0 && e.y < sh && e.e_weapon != EnemyWeapon::None {
                        let pa = (player.y-e.y).atan2(player.x-e.x);
                        let bullet_spd = (180.0 + difficulty*20.0) * diff_spd_mult;
                        let dmg_mult = diff_mult * diff_mode_hp_mult.sqrt();
                        match e.e_weapon {
                            EnemyWeapon::Single => {
                                eb.push(EBullet { x: e.x, y: e.y, vx: pa.cos()*bullet_spd, vy: pa.sin()*bullet_spd, damage: ((5.0+wave as f32/2.0)*dmg_mult) as i32, life: 4.0, size: 3.5 });
                                e.shoot_timer = 0.8 + ::rand::thread_rng().gen::<f32>()*0.8;
                            }
                            EnemyWeapon::Spread => {
                                for i in -1..=1 {
                                    let a2 = pa + i as f32*0.15;
                                    eb.push(EBullet { x: e.x, y: e.y, vx: a2.cos()*(bullet_spd*0.9), vy: a2.sin()*(bullet_spd*0.9), damage: ((4.0+wave as f32/3.0)*dmg_mult) as i32, life: 3.5, size: 3.0 });
                                }
                                e.shoot_timer = 1.2 + ::rand::thread_rng().gen::<f32>()*0.5;
                            }
                            EnemyWeapon::Burst => {
                                for _ in 0..3 {
                                    let r = ::rand::thread_rng().gen::<f32>()*0.1-0.05;
                                    let a2 = pa + r;
                                    eb.push(EBullet { x: e.x, y: e.y, vx: a2.cos()*bullet_spd*1.1, vy: a2.sin()*bullet_spd*1.1, damage: ((4.0+wave as f32/3.0)*dmg_mult) as i32, life: 3.0, size: 3.0 });
                                }
                                e.shoot_timer = 1.5 + ::rand::thread_rng().gen::<f32>()*0.5;
                            }
                            EnemyWeapon::Sniper => {
                                eb.push(EBullet { x: e.x, y: e.y, vx: pa.cos()*bullet_spd*3.5, vy: pa.sin()*bullet_spd*3.5, damage: ((12.0+wave as f32/1.5)*dmg_mult) as i32, life: 5.0, size: 5.0 });
                                e.shoot_timer = 2.0 + ::rand::thread_rng().gen::<f32>()*0.5;
                                particles.extend(mkparticles(e.x,e.y,Color::new(1.0,0.3,0.8,1.0),8,150.0,0.0));
                                shake = (shake+3.0).min(8.0);
                            }
                            _ => {}
                        }
                    }
                }
                enemies.retain(|e| e.x>-100.0&&e.x<sw+100.0&&e.y>-100.0&&e.y<sh+200.0);

                // Boss update
                if let Some(ref mut b) = boss {
                    b.timer += dt; b.angle += dt*0.3; b.hit_flash *= 0.9;
                    if b.moving_in { if b.y < 80.0 { b.vy += 50.0*dt; } else { b.vy *= 0.95; b.moving_in = false; } }
                    else {
                        b.vy *= 0.98;
                        let amp = if b.kind == BossKind::Triplet { 80.0 } else { 50.0 };
                        b.x += (b.timer*(if b.kind==BossKind::Triplet{0.6}else{0.4})).sin()*amp*dt;
                    }
                    b.x = b.x.clamp(60.0, sw-60.0); b.y = b.y.clamp(60.0, sh*0.3);
                    b.x += b.vx*dt; b.y += b.vy*dt;

                    if !b.moving_in {
                        b.bullet_cooldown -= dt;
                        if b.bullet_cooldown <= 0.0 && eb.len() < MAX_BOSS_BULLETS {
                            let mut rng = ::rand::thread_rng();
                            let base_spd = 110.0 + difficulty*12.0;
                            let cd_mul = match b.kind { BossKind::Barrage => 0.6, BossKind::Triplet => 1.3, BossKind::Tank => 1.2, _ => 1.0 };
                            match b.pattern {
                                0 => {
                                    let n = if b.kind == BossKind::Barrage { 16 } else { 12 };
                                    for i in 0..n { let a = i as f32/n as f32*TAU + b.timer;
                                        eb.push(EBullet { x: b.x, y: b.y, vx: a.cos()*base_spd, vy: a.sin()*base_spd, damage: 10+wave/2, life: 5.0, size: 4.5 }); }
                                    b.bullet_cooldown = 2.0 * cd_mul;
                                }
                                1 => {
                                    let pa = (player.y-b.y).atan2(player.x-b.x);
                                    let n = if b.kind == BossKind::Barrage { 5 } else { 3 };
                                    for i in -n/2..=n/2 { let a = pa + i as f32*0.1;
                                        let s = if b.kind==BossKind::Triplet{130.0+difficulty*12.0}else{170.0+difficulty*10.0};
                                        eb.push(EBullet { x: b.x, y: b.y, vx: a.cos()*s, vy: a.sin()*s, damage: 8+wave/3, life: 4.0, size: 4.0 }); }
                                    b.bullet_cooldown = 1.5 * cd_mul;
                                }
                                2 => {
                                    let n = if b.kind == BossKind::Barrage { 8 } else { 6 };
                                    for i in 0..n { let a = i as f32/n as f32*TAU + b.timer*2.0;
                                        let s = base_spd*0.8 + (b.timer*3.0).sin()*30.0;
                                        eb.push(EBullet { x: b.x, y: b.y, vx: a.cos()*s, vy: a.sin()*s, damage: 9+wave/3, life: 4.0, size: 4.0 }); }
                                    b.bullet_cooldown = 1.8 * cd_mul;
                                }
                                _ => {
                                    let n = if b.kind == BossKind::Barrage { 8 } else { 5 };
                                    for _ in 0..n { let a = rng.gen::<f32>()*TAU;
                                        eb.push(EBullet { x: b.x, y: b.y, vx: a.cos()*(base_spd+rng.gen::<f32>()*50.0), vy: a.sin()*(base_spd+rng.gen::<f32>()*50.0), damage: 7+wave/3, life: 3.5, size: 3.5 }); }
                                    b.bullet_cooldown = 1.2 * cd_mul;
                                }
                            }
                            b.pattern = rng.gen_range(0..4);
                            if b.kind == BossKind::Triplet {
                                for side in -1..=1 {
                                    let a = b.timer*1.5 + side as f32;
                                    let td = 30.0;
                                    let (tx, ty) = (b.x + a.cos()*td, b.y + a.sin()*td);
                                    let ta = (player.y-ty).atan2(player.x-tx);
                                    eb.push(EBullet { x: tx, y: ty, vx: ta.cos()*120.0, vy: ta.sin()*120.0, damage: 5+wave/4, life: 4.0, size: 3.0 });
                                }
                            }
                        }
                    }
                    let hp_pct = b.hp as f32 / b.max_hp as f32;
                    if hp_pct < 0.25 { b.phase = 2; } else if hp_pct < 0.55 { b.phase = 1; }
                    if b.phase > 0 { b.bullet_cooldown *= (0.82 - b.phase as f32*0.05); }
                }

                // === COLLISIONS ===

                // 1. Player bullets vs Asteroids
                let mut pb_alive = Vec::with_capacity(pb.len());
                let mut ast_alive: Vec<Asteroid> = Vec::new();
                let mut new_asts: Vec<Asteroid> = Vec::new();
                for mut b in pb.into_iter() {
                    let mut hit = false;
                    for a in &mut asteroids {
                        if (b.x-a.x).hypot(b.y-a.y) < a.size + b.size {
                            hit = true; a.hp -= b.damage; b.pierce -= 1;
                            particles.extend(mkparticles(b.x,b.y, a.color, 5, 80.0, 5.0));
                            shake = (shake+1.5).min(6.0);
                            if a.hp <= 0 {
                                let pts = (a.size as i64 * 3 + 20) * (1 + player.combo/5) as i64;
                                player.score += pts; player.kills += 1;
                                player.gold += 1 + ::rand::thread_rng().gen_range(0..2) + player.shop_gold_lv as i64;
                                player.combo += 1; player.combotimer = 1.5 + player.combo_bonus;
                                floating.push(FloatingText { x: a.x, y: a.y, text: format!("+{}", pts), life: 1.2, color: Color::new(1.0,0.9,0.3,1.0), size: 18.0 });
                                particles.extend(explode(a.x,a.y,a.color,Color::new(1.0,0.6,0.2,1.0),Color::new(1.0,1.0,0.5,1.0),30));
                                shake = (shake+3.0).min(10.0);
                                new_asts.extend(split_asteroid(a));
                                if ::rand::thread_rng().gen::<f32>() < 0.10 { powerups.push(PowerUp { x: a.x, y: a.y, kind: ::rand::thread_rng().gen_range(0..8), vy: 30.0 }); }
                            } else { particles.extend(mkparticles(b.x,b.y, a.color, 5, 80.0, 5.0)); }
                            break;
                        }
                    }
                    if hit && b.pierce < 0 { continue; }
                    if hit { b.damage = (b.damage as f32 * 0.7) as i32; if b.damage < 1 { continue; } }
                    pb_alive.push(b);
                }
                for a in asteroids.into_iter() { if a.hp > 0 { ast_alive.push(a); } }
                ast_alive.extend(new_asts); asteroids = ast_alive; pb = pb_alive;

                // 2. Player bullets vs Enemies
                let mut pb_alive2 = Vec::with_capacity(pb.len());
                let mut enemies_alive = Vec::new();
                for b in pb.into_iter() {
                    let mut hit = false;
                    for e in &mut enemies {
                        if (b.x-e.x).hypot(b.y-e.y) < e.size + b.size {
                            hit = true; e.hit_flash = 1.0;
                            if e.e_special == EnemySpecial::Shield && e.hp >= e.max_hp * 3 / 4 {
                                e.hp = e.max_hp / 2;
                                e.e_special = EnemySpecial::None;
                                particles.extend(explode(e.x,e.y,Color::new(0.3,0.6,1.0,1.0),Color::new(0.5,0.8,1.0,1.0),Color::new(0.7,0.9,1.0,1.0),20));
                                break;
                            }
                            e.hp -= b.damage;
                            particles.extend(mkparticles(b.x,b.y, e.color, 6, 100.0, 5.0));
                            if e.hp <= 0 {
                                let pts = ((50+wave*5) * (1+player.combo/5)) as i64;
                                player.score += pts; player.kills += 1;
                                let gold_add = match e.e_size { EnemySize::Tiny|EnemySize::Small => 2, EnemySize::Medium => 3, EnemySize::Large => 4, EnemySize::Huge => 5 };
                                player.gold += gold_add + player.shop_gold_lv as i64;
                                player.combo += 1; player.combotimer = 1.5 + player.combo_bonus;
                                floating.push(FloatingText {
                                    x: e.x, y: e.y-10.0,
                                    text: format!("{} +{}", template_name(e.template_id), pts),
                                    life: 1.5, color: e.color, size: 14.0 });
                                if e.e_special == EnemySpecial::Bomb {
                                    particles.extend(explode(e.x,e.y,Color::new(1.0,0.5,0.1,1.0),Color::new(1.0,0.8,0.2,1.0),Color::new(1.0,1.0,0.5,1.0),50));
                                    shake = (shake+8.0).min(15.0);
                                    for a2 in &mut asteroids { if (e.x-a2.x).hypot(e.y-a2.y) < 100.0 { a2.hp -= 5; } }
                                    if (e.x-player.x).hypot(e.y-player.y) < 100.0 && player.invincible <= 0.0 { player.hp -= 2; player.invincible = 1.5; }
                                } else {
                                    particles.extend(explode(e.x,e.y,e.color,Color::new(1.0,0.5,0.2,1.0),Color::new(1.0,1.0,0.6,1.0),30));
                                }
                                shake = (shake+4.0).min(10.0);
                                if ::rand::thread_rng().gen::<f32>() < 0.12 { powerups.push(PowerUp { x: e.x, y: e.y, kind: ::rand::thread_rng().gen_range(0..8), vy: 30.0 }); }
                            }
                            break;
                        }
                    }
                    if !hit { pb_alive2.push(b); }
                }
                for e in enemies.into_iter() { if e.hp > 0 { enemies_alive.push(e); } }
                enemies = enemies_alive; pb = pb_alive2;

                // 3. Player bullets vs Boss
                if let Some(ref mut b) = boss {
                    let mut pb_alive3 = Vec::new();
                    for b2 in pb.into_iter() {
                        if (b2.x-b.x).hypot(b2.y-b.y) < b.size + b2.size {
                            b.hp -= b2.damage; b.hit_flash = 1.0; shake = (shake+2.0).min(8.0);
                            particles.extend(mkparticles(b2.x,b2.y, Color::new(1.0,0.8,0.3,1.0), 8, 120.0, 0.0));
                            if b.hp <= 0 {
                                let pts = (400 + wave*120 + boss_killed*250) as i64;
                                player.score += pts;
                                player.gold += 15i64 + ::rand::thread_rng().gen_range(0i64..11) + boss_killed as i64*5 + player.shop_gold_lv as i64*2;
                                boss_killed += 1;
                                floating.push(FloatingText { x: sw/2.0, y: sh*0.3, text: format!("BOSS YOK EDİLDİ! +{}", pts), life: 3.0, color: Color::new(1.0,0.9,0.2,1.0), size: 36.0 });
                                particles.extend(explode(b.x,b.y,Color::new(1.0,0.3,0.1,1.0),Color::new(1.0,0.8,0.2,1.0),Color::new(1.0,1.0,0.5,1.0),80));
                                shake = 20.0; flash = 0.8;
                                for _ in 0..8 { let mut rng = ::rand::thread_rng();
                                    powerups.push(PowerUp { x: b.x+rng.gen::<f32>()*60.0-30.0, y: b.y+rng.gen::<f32>()*60.0-30.0, kind: rng.gen_range(0..8), vy: 20.0 }); }
                                boss = None; break;
                            }
                            pb_alive3.push(b2);
                        } else { pb_alive3.push(b2); }
                    }
                    if boss.as_ref().map_or(false, |b| b.hp <= 0) { boss = None; }
                    pb = pb_alive3;
                }

                // 4. Enemy bullets vs Player
                let mut eb_alive = Vec::new();
                for b in eb.into_iter() {
                    if player.invincible <= 0.0 && (b.x-player.x).hypot(b.y-player.y) < PLAYER_SIZE*0.7 + b.size {
                        player.hp -= 1; player.invincible = 0.5 * diff_mult.recip().min(1.5); shake = 8.0; flash = 0.3; player.combo = 0;
                        particles.extend(mkparticles(player.x,player.y,Color::new(1.0,0.4,0.4,1.0),20,200.0,10.0));
                        if player.hp <= 0 && player.shield > (0.3*diff_mult.recip()).min(2.0) {
                            player.shield -= (0.5*diff_mult).max(0.3); player.hp = 1; player.invincible = (1.0 * diff_mult.recip()).min(2.0);
                            floating.push(FloatingText { x: player.x, y: player.y-20.0, text: "KALKAN!".to_string(), life: 1.0, color: Color::new(0.3,0.6,1.0,1.0), size: 20.0 });
                        } else if player.hp <= 0 {
                            state = GameState::GameOver;
                            if player.score > highscore { highscore = player.score; save_highscore(player.score); }
                            particles.extend(explode(player.x,player.y,Color::new(1.0,0.2,0.1,1.0),Color::new(1.0,0.7,0.2,1.0),Color::new(1.0,1.0,0.5,1.0),100));
                            shake = 30.0; flash = 1.5;
                        }
                    } else { eb_alive.push(b); }
                }
                eb = eb_alive;

                // 5. Player vs Asteroids
                if player.invincible <= 0.0 {
                    for a in &asteroids {
                        if (player.x-a.x).hypot(player.y-a.y) < a.size + PLAYER_SIZE*0.6 {
                            let dmg = ((if a.kind==AsteroidKind::Fire { 3 } else { 2 }) as f32 * diff_mult) as i32;
                            player.hp -= dmg; player.invincible = 0.8 * diff_mult.recip().min(1.5); shake = 10.0; flash = 0.4; player.combo = 0;
                            particles.extend(mkparticles(player.x,player.y,Color::new(1.0,0.4,0.4,1.0),25,250.0,10.0));
                            if player.hp <= 0 && player.shield > (0.3*diff_mult.recip()).min(2.0) { player.shield -= (0.5*diff_mult).max(0.3); player.hp = 1; player.invincible = (1.0*diff_mult.recip()).min(2.0);
                                floating.push(FloatingText { x: player.x, y: player.y-20.0, text: "KALKAN!".to_string(), life: 1.0, color: Color::new(0.3,0.6,1.0,1.0), size: 20.0 }); }
                            else if player.hp <= 0 { state = GameState::GameOver;
                                if player.score > highscore { highscore = player.score; save_highscore(player.score); }
                                particles.extend(explode(player.x,player.y,Color::new(1.0,0.2,0.1,1.0),Color::new(1.0,0.7,0.2,1.0),Color::new(1.0,1.0,0.5,1.0),100));
                                shake = 30.0; flash = 1.5; }
                            break;
                        }
                    }
                }

                // 6. Player vs Enemies (contact damage)
                if player.invincible <= 0.0 {
                    for e in &enemies {
                        if (player.x-e.x).hypot(player.y-e.y) < e.size + PLAYER_SIZE*0.6 {
                            let dmg = ((if e.e_special == EnemySpecial::Bomb { 4 } else { 2 }) as f32 * diff_mult) as i32;
                            player.hp -= dmg; player.invincible = 0.8 * diff_mult.recip().min(1.5); shake = 10.0; flash = 0.4; player.combo = 0;
                            particles.extend(mkparticles(player.x,player.y,Color::new(1.0,0.4,0.4,1.0),25,250.0,10.0));
                            if e.e_special == EnemySpecial::Bomb {
                                particles.extend(explode(e.x,e.y,Color::new(1.0,0.5,0.1,1.0),Color::new(1.0,0.8,0.2,1.0),Color::new(1.0,1.0,0.5,1.0),30));
                            }
                            if player.hp <= 0 && player.shield > (0.3*diff_mult.recip()).min(2.0) { player.shield -= (0.5*diff_mult).max(0.3); player.hp = 1; player.invincible = (1.0*diff_mult.recip()).min(2.0); }
                            else if player.hp <= 0 { state = GameState::GameOver;
                                if player.score > highscore { highscore = player.score; save_highscore(player.score); }
                                particles.extend(explode(player.x,player.y,Color::new(1.0,0.2,0.1,1.0),Color::new(1.0,0.7,0.2,1.0),Color::new(1.0,1.0,0.5,1.0),100));
                                shake = 30.0; flash = 1.5; }
                            break;
                        }
                    }
                }

                // --- POWERUPS ---
                for p in &mut powerups { p.y += p.vy*dt; }
                let mut alive_pu = Vec::new();
                for p in powerups.into_iter() {
                    if (player.x-p.x).hypot(player.y-p.y) < PLAYER_SIZE+14.0 {
                        match p.kind { 0 => player.hp = (player.hp+1).min(player.max_hp), 1 => player.shield = (player.shield+1.0).min(player.max_shield), 2 => player.speed += 0.5,
                            3 => player.weapon_level = (player.weapon_level+1).min(5), 4 => player.drones = (player.drones+1).min(5), 5 => player.score += 200, 6 => { player.max_hp += 1; player.hp = (player.hp+1).min(player.max_hp); }, _ => player.fire_rate_bonus += 0.02 }
                        let txt = match p.kind { 0=>"+HP", 1=>"KALKAN", 2=>"HIZ", 3=>"SİLAH", 4=>"DRONE", 5=>"+200", 6=>"+MAX HP", _=>"ATEŞ HIZI" };
                        floating.push(FloatingText { x: p.x, y: p.y, text: txt.to_string(), life: 1.0, color: Color::new(0.3,1.0,0.3,1.0), size: 16.0 });
                        particles.extend(mkparticles(p.x,p.y,Color::new(0.3,1.0,0.3,1.0),12,100.0,0.0));
                    } else if p.y < sh+30.0 { alive_pu.push(p); }
                }
                powerups = alive_pu;
                if powerups.len() > MAX_POWERUPS { powerups.drain(0..powerups.len()-MAX_POWERUPS); }

                // PARTICLES
                for p in &mut particles { p.x+=p.vx*dt; p.y+=p.vy*dt; p.vx*=0.96; p.vy*=0.96; p.vy+=p.gravity*dt; p.life-=dt; }
                particles.retain(|p| p.life>0.0);
                if particles.len() > MAX_PARTICLES { particles.drain(0..particles.len()-MAX_PARTICLES); }

                for s in &mut stars { s.y += s.speed*dt*(0.1+s.layer*0.3); if s.y>sh { s.y=-2.0; s.x=::rand::thread_rng().gen::<f32>()*sw; } }
                for ft in &mut floating { ft.y -= dt*40.0; ft.life -= dt; }
                floating.retain(|ft| ft.life>0.0);

                // === RENDER ===
                clear_background(Color::from_rgba(3,3,12,255));
                let (sx, sy) = if shake > 0.5 { let mut rng = ::rand::thread_rng(); (rng.gen::<f32>()*shake*2.0-shake, rng.gen::<f32>()*shake*2.0-shake) } else { (0.0, 0.0) };
                if flash > 0.05 { draw_rectangle(-10.0,-10.0,sw+20.0,sh+20.0, Color::new(1.0,1.0,1.0,flash*0.3)); }

                for s in &stars { let b = s.brightness*(0.5+s.layer*0.5); draw_circle(s.x+sx, s.y+sy, s.size*(0.3+s.layer*0.7), Color::new(b,b*0.9,b,0.5+s.layer*0.5)); }

                for p in &powerups {
                    let t = get_time() as f32; let g = (t*5.0+p.x).sin()*0.3+0.7;
                    let (cr,cg,cb) = match p.kind { 0=>(0.2,g,0.2),1=>(g*0.3,g*0.5,1.0),2=>(g,g,0.2),3=>(g,0.3,0.8),4=>(0.3,g,g),5=>(g,0.7,0.3),6=>(g,0.3,0.3),_=>(1.0,g*0.5,0.0) };
                    draw_circle(p.x+sx, p.y+sy, 10.0, Color::new(cr,cg,cb,1.0));
                    draw_circle(p.x+sx, p.y+sy, 6.0, Color::new(1.0,1.0,1.0,0.4));
                    let lbl = match p.kind { 0=>"+HP",1=>"KALKAN",2=>"HIZ",3=>"SİLAH",4=>"DRONE",5=>"+200",6=>"+CAN",_=>"+ATEŞ" };
                    let lw = measure_text(lbl,None,11,1.0).width; draw_text(lbl, p.x+sx-lw/2.0, p.y+sy-12.0, 11.0, Color::new(1.0,1.0,1.0,0.7));
                }

                for b in &pb {
                    let col = match b.kind { WeaponType::Normal => Color::new(0.6,0.8,1.0,1.0), WeaponType::Spread => Color::new(0.3,1.0,0.5,1.0), WeaponType::Homing => Color::new(1.0,0.5,0.8,1.0), WeaponType::Laser => Color::new(1.0,0.2,0.1,1.0) };
                    draw_circle(b.x+sx-b.vx*dt*0.3, b.y+sy-b.vy*dt*0.3, b.size*0.4, Color::new(col.r*0.3,col.g*0.3,col.b*0.3,0.3));
                    draw_circle(b.x+sx, b.y+sy, b.size, col);
                    draw_circle(b.x+sx, b.y+sy, b.size*0.4, Color::new(1.0,1.0,1.0,0.6));
                    if b.kind == WeaponType::Laser { draw_circle(b.x+sx, b.y+sy, b.size*2.0, Color::new(1.0,0.3,0.1,0.2)); }
                }

                for b in &eb { draw_circle(b.x+sx, b.y+sy, b.size, Color::new(1.0,0.3,0.2,1.0)); draw_circle(b.x+sx-b.vx*dt*0.3, b.y+sy-b.vy*dt*0.3, b.size*0.5, Color::new(0.8,0.2,0.1,0.3)); }

                for a in &asteroids {
                    let (cos, sin) = (a.rot.cos(), a.rot.sin());
                    let pts: Vec<(f32,f32)> = a.verts.iter().map(|v| { let rx = v.x*cos - v.y*sin; let ry = v.x*sin + v.y*cos; (rx+a.x+sx, ry+a.y+sy) }).collect();
                    for i in 0..pts.len() { let j = (i+1)%pts.len(); draw_line(pts[i].0, pts[i].1, pts[j].0, pts[j].1, 1.5, a.color); }
                    if a.hp < a.max_hp { let bw = a.size*1.5; draw_rectangle(a.x+sx-bw/2.0, a.y+sy-a.size-8.0, bw, 3.0, Color::new(0.3,0.3,0.3,0.6));
                        draw_rectangle(a.x+sx-bw/2.0, a.y+sy-a.size-8.0, bw*(a.hp as f32/a.max_hp as f32), 3.0, Color::new(0.3,0.8,0.3,0.8)); }
                    if a.kind == AsteroidKind::Fire { let g = (get_time() as f32*5.0).sin()*0.2+0.5; draw_circle(a.x+sx, a.y+sy, a.size*1.3, Color::new(1.0,0.3,0.1,0.12*g)); draw_circle(a.x+sx, a.y+sy, a.size*0.8, Color::new(1.0,0.5,0.1,0.08*g)); }
                    if a.kind == AsteroidKind::Ice { let g = (get_time() as f32*3.0).sin()*0.2+0.5; draw_circle(a.x+sx, a.y+sy, a.size*1.3, Color::new(0.3,0.6,1.0,0.10*g)); }
                }

                // Enemy rendering with template name
                for e in &enemies {
                    let col = if e.hit_flash > 0.5 { Color::new(1.0,1.0,1.0,1.0) } else { e.color };
                    let ep = [Vec2::new(e.x+e.angle.cos()*e.size*1.2+sx, e.y+e.angle.sin()*e.size*1.2+sy),
                        Vec2::new(e.x+(e.angle+2.3).cos()*e.size*0.9+sx, e.y+(e.angle+2.3).sin()*e.size*0.9+sy),
                        Vec2::new(e.x+(e.angle-2.3).cos()*e.size*0.9+sx, e.y+(e.angle-2.3).sin()*e.size*0.9+sy)];
                    draw_triangle(ep[0],ep[1],ep[2],col);
                    draw_triangle_lines(ep[0],ep[1],ep[2],1.5,Color::new(col.r+0.3,col.g+0.3,col.b+0.3,0.8));
                    // Special indicators
                    if e.e_special == EnemySpecial::Shield {
                        let t = get_time() as f32; draw_circle_lines(e.x+sx, e.y+sy, e.size*1.6, 2.0, Color::new(0.3,0.6,1.0,(t*3.0).sin()*0.2+0.3)); }
                    if e.e_special == EnemySpecial::Bomb {
                        let t = get_time() as f32; draw_circle(e.x+sx, e.y+sy, e.size*1.2, Color::new(1.0,0.4,0.1,(t*6.0).sin()*0.1+0.12)); }
                    if e.e_special == EnemySpecial::Regen {
                        let t = get_time() as f32; draw_circle(e.x+sx, e.y+sy, e.size*1.3, Color::new(0.3,1.0,0.4,(t*4.0).sin()*0.08+0.08)); }
                    if e.e_weapon == EnemyWeapon::Sniper {
                        let t = get_time() as f32; draw_circle(e.x+sx, e.y+sy, e.size*0.4, Color::new(1.0,0.3,0.8,(t*4.0).sin()*0.15+0.3)); }
                    if e.hp < e.max_hp {
                        draw_rectangle(e.x+sx-e.size*0.8, e.y+sy+e.size+2.0, e.size*1.6, 2.5, Color::new(0.3,0.3,0.3,0.6));
                        draw_rectangle(e.x+sx-e.size*0.8, e.y+sy+e.size+2.0, e.size*1.6*(e.hp as f32/e.max_hp as f32), 2.5, Color::new(1.0,0.3,0.2,0.8)); }
                }

                // Boss
                if let Some(ref b) = boss {
                    let t = get_time() as f32; let pulse = (t*3.0).sin()*0.1+0.9;
                    let col = if b.hit_flash > 0.2 { Color::new(1.0,1.0,1.0,1.0) } else { b.color };
                    for ring in 0..4 { let r = b.size*(1.0-ring as f32*0.18)*pulse; let a = 0.35-ring as f32*0.07; draw_circle(b.x+sx, b.y+sy, r, Color::new(col.r,col.g,col.b,a)); }
                    draw_circle_lines(b.x+sx, b.y+sy, b.size*pulse, 2.5, col);
                    draw_circle_lines(b.x+sx, b.y+sy, b.size*0.65, 1.5, Color::new(1.0,1.0,1.0,0.3));
                    draw_circle(b.x+sx, b.y+sy, b.size*0.25, Color::new(1.0,0.5,0.2,0.8));
                    if b.kind == BossKind::Triplet { for i in -1..=1 { let a = t*1.5 + i as f32; let td = 30.0; draw_circle(b.x+sx+a.cos()*td, b.y+sy+a.sin()*td, 8.0, Color::new(0.3,0.6,0.9,0.8)); draw_circle_lines(b.x+sx+a.cos()*td, b.y+sy+a.sin()*td, 10.0, 1.0, Color::new(0.3,0.6,1.0,0.4)); } }
                    if b.kind == BossKind::Tank { draw_circle_lines(b.x+sx, b.y+sy, b.size*1.4, 3.0, Color::new(0.6,0.2,0.8,0.4)); }
                    if b.kind == BossKind::Barrage { let bt = (t*5.0).sin()*0.3+0.5; draw_circle(b.x+sx, b.y+sy, b.size*1.1, Color::new(1.0,0.3,0.1,bt*0.15)); }
                    let (bw, bh) = (350.0, 12.0); let bx = sw/2.0-bw/2.0; let by = 15.0;
                    draw_rectangle(bx, by, bw, bh, Color::new(0.2,0.2,0.3,0.8));
                    let hp_pct = b.hp as f32 / b.max_hp as f32;
                    draw_rectangle(bx, by, bw*hp_pct, bh, Color::new(1.0-hp_pct*0.7, hp_pct*0.7, 0.2, 0.9));
                    draw_rectangle_lines(bx, by, bw, bh, 1.5, Color::new(1.0,1.0,1.0,0.5));
                    let btxt = format!("{} BOSS ({}/{})", match b.kind { BossKind::Default => "", BossKind::Barrage => "BARRAGE", BossKind::Tank => "TANK", BossKind::Triplet => "TRIPLET" }, b.hp, b.max_hp);
                    draw_text(&btxt, bx, by+bh*0.8, 11.0, Color::new(1.0,1.0,1.0,0.7));
                }

                for dr in &drones { draw_circle(dr.x+sx, dr.y+sy, 5.0, Color::new(0.2,0.8,1.0,0.9)); draw_circle(dr.x+sx, dr.y+sy, 2.5, Color::new(1.0,1.0,1.0,0.6)); draw_circle_lines(dr.x+sx, dr.y+sy, 7.0, 1.0, Color::new(0.3,0.7,1.0,0.4)); }

                // Player
                if player.invincible <= 0.0 || (get_time() as f32*12.0).sin() > 0.0 {
                    let (px, py) = (player.x+sx, player.y+sy); let pa = player.angle;
                    if player.shield > 0.3 { let sh_sz = PLAYER_SIZE*2.0 + (get_time() as f32*3.0).sin()*2.0; let sh_a = 0.12+(player.shield/player.max_shield)*0.12; draw_circle(px, py, sh_sz, Color::new(0.3,0.6,1.0,sh_a)); draw_circle_lines(px, py, sh_sz, 1.5, Color::new(0.4,0.7,1.0,sh_a*1.5)); }
                    if player.dash_cooldown > 0.0 { let dcd=PLAYER_SIZE*2.3; let prog=1.0-(player.dash_cooldown/0.6); draw_circle_lines(px,py,dcd,2.0,Color::new(0.3,0.5,0.8,0.3)); let segs=20; for i in 0..segs { let a1=-PI/2.0+i as f32/segs as f32*TAU*prog; let a2=-PI/2.0+(i+1)as f32/segs as f32*TAU*prog; draw_line(px+a1.cos()*dcd,py+a1.sin()*dcd,px+a2.cos()*dcd,py+a2.sin()*dcd,2.0,Color::new(0.3,0.8,1.0,0.5)); } }
                    let tip=Vec2::new(px+pa.cos()*PLAYER_SIZE*1.3,py+pa.sin()*PLAYER_SIZE*1.3); let left=Vec2::new(px+(pa+2.3).cos()*PLAYER_SIZE*0.9,py+(pa+2.3).sin()*PLAYER_SIZE*0.9); let right=Vec2::new(px+(pa-2.3).cos()*PLAYER_SIZE*0.9,py+(pa-2.3).sin()*PLAYER_SIZE*0.9); let eng=Vec2::new(px+(pa+PI).cos()*PLAYER_SIZE*1.5,py+(pa+PI).sin()*PLAYER_SIZE*1.5); let f=0.5+(get_time() as f32*18.0).sin()*0.3; draw_circle(eng.x,eng.y,7.0*f,Color::new(0.8,0.4,0.1,0.6*f)); draw_circle(eng.x,eng.y,3.5*f,Color::new(1.0,0.7,0.2,0.4*f)); draw_triangle(tip,left,right,Color::new(0.2,0.6,1.0,1.0)); draw_triangle_lines(tip,left,right,1.5,Color::new(0.5,0.8,1.0,0.8)); draw_circle(px+pa.cos()*PLAYER_SIZE*0.3,py+pa.sin()*PLAYER_SIZE*0.3,3.5,Color::new(0.6,0.9,1.0,0.8));
                }

                for p in &particles { let a=(p.life/p.max_life).max(0.0); if p.glow>0.5{draw_circle(p.x+sx,p.y+sy,p.size*a*3.0,Color::new(p.color.r,p.color.g,p.color.b,a*0.3));} draw_circle(p.x+sx,p.y+sy,p.size*a,Color::new(p.color.r,p.color.g,p.color.b,a)); }
                for ft in &floating { let a=(ft.life/1.5).max(0.0); let w=measure_text(&ft.text,None,ft.size as u16,1.0).width; draw_text(&ft.text, ft.x+sx-w/2.0, ft.y+sy, ft.size, Color::new(ft.color.r,ft.color.g,ft.color.b,a)); }

                if wave_announce > 0.0 { wave_announce -= dt; let a=(wave_announce/2.0).min(1.0); let txt=format!("DALGA {} ({} DÜŞMAN)", wave-1, enemies.len()+asteroids.len()); draw_text(&txt, sw/2.0-measure_text(&txt,None,32,1.0).width/2.0, sh*0.5, 32.0, Color::new(1.0,0.8,0.2,a)); }

                // HUD
                let hp_s: String = (0..player.hp).map(|_| "\u{2764}").collect();
                draw_text(&hp_s, 15.0, 28.0, 18.0, Color::new(1.0,0.3,0.3,1.0));
                let sp = player.shield/player.max_shield; draw_rectangle(15.0,35.0,70.0,3.5,Color::new(0.2,0.2,0.5,0.5)); draw_rectangle(15.0,35.0,70.0*sp,3.5,Color::new(0.3,0.5,1.0,0.8));
                let ep = player.energy/player.max_energy; draw_rectangle(15.0,41.5,70.0,3.5,Color::new(0.3,0.2,0.3,0.5)); draw_rectangle(15.0,41.5,70.0*ep,3.5,Color::new(0.8,0.3,0.8,0.8));
                draw_text(&format!("SKOR: {}", player.score), 15.0, 68.0, 16.0, Color::new(0.8,0.8,1.0,0.9));
                draw_text(&format!("DALGA: {}", wave-1), 15.0, 86.0, 14.0, Color::new(0.6,0.8,1.0,0.7));
                draw_text(&format!("ÖLDÜRME: {}", player.kills), 15.0, 102.0, 12.0, Color::new(0.6,0.6,0.8,0.6));
                draw_text(&format!("ALTIN: {}", player.gold), 15.0, 116.0, 13.0, Color::new(1.0,0.8,0.3,0.9));
                let dname = match diff_mode { DiffMode::Easy => "KOLAY", DiffMode::Normal => "NORMAL", DiffMode::Hard => "ZOR" };
                draw_text(dname, sw-measure_text(dname,None,14,1.0).width-10.0, 28.0, 14.0, match diff_mode { DiffMode::Easy => Color::new(0.3,1.0,0.3,0.8), DiffMode::Normal => Color::new(1.0,0.8,0.3,0.8), DiffMode::Hard => Color::new(1.0,0.3,0.3,0.8) });

                if boss.is_none() && enemies.is_empty() && asteroids.len() < 4 && wave_timer > 2.0 {
                    let t = get_time() as f32; let ba = (t*2.0).sin()*0.3+0.7;
                    draw_text("[MAĞAZA İÇİN ESC]", sw/2.0-measure_text("[MAĞAZA İÇİN ESC]",None,20,1.0).width/2.0, sh-50.0, 20.0, Color::new(0.3,0.8,1.0,ba));
                }

                if player.combo >= 3 {
                    let cc = Color::new(1.0,0.7-(player.combo as f32*0.04).min(0.5),0.1,0.8+(get_time() as f32*5.0).sin()*0.2);
                    draw_text(&format!("COMBO x{}!", player.combo), 15.0, 134.0, 16.0, cc);
                }

                let wt = format!("{} [{:02}E]", match player.weapon { WeaponType::Normal=>"NORMAL", WeaponType::Spread=>"SAÇMA", WeaponType::Homing=>"GÜDÜMLÜ", WeaponType::Laser=>"LAZER" }, player.energy as i32);
                let wc = match player.weapon { WeaponType::Normal=>Color::new(0.6,0.8,1.0,0.8), WeaponType::Spread=>Color::new(0.3,1.0,0.5,0.8), WeaponType::Homing=>Color::new(1.0,0.5,0.8,0.8), WeaponType::Laser=>Color::new(1.0,0.2,0.1,0.8) };
                draw_text(&wt, 15.0, sh-20.0, 14.0, wc);
                if player.drones > 0 { draw_text(&format!("DRONE: {}/{}", drones.len(), player.drones), 15.0, sh-38.0, 12.0, Color::new(0.3,0.7,1.0,0.7)); }
                draw_text("1:NORMAL 2:SAÇMA 3:GÜDÜMLÜ 4:LAZER", sw-measure_text("1:NORMAL 2:SAÇMA 3:GÜDÜMLÜ 4:LAZER",None,11,1.0).width-10.0, sh-15.0, 11.0, Color::new(0.4,0.4,0.6,0.6));

                // Stats panel (Tab key)
                if is_key_down(KeyCode::Tab) {
                    let bx = sw/2.0 - 220.0; let by = sh/2.0 - 180.0; let bw = 440.0; let bh = 360.0;
                    draw_rectangle(bx, by, bw, bh, Color::new(0.05,0.05,0.15,0.92));
                    draw_rectangle_lines(bx, by, bw, bh, 1.5, Color::new(0.3,0.6,1.0,0.6));
                    draw_text("STATLAR", bx+10.0, by+25.0, 20.0, Color::new(0.3,0.8,1.0,1.0));
                    let stats_lines = [
                        format!("CAN: {}/{}", player.hp, player.max_hp),
                        format!("KALKAN: {:.1}/{}", player.shield, player.max_shield),
                        format!("ENERJİ: {:.0}/{}", player.energy, player.max_energy),
                        format!("HIZ: {:.1}", player.speed),
                        format!("SİLAH LV: {} [HASAR +{}]", player.weapon_level, player.damage_bonus),
                        format!("ATEŞ HIZI BONUS: +{:.1}%", player.fire_rate_bonus*100.0),
                        format!("DRONE: {}/{}", drones.len(), player.drones),
                        format!("ALTIN: {} | SKOR: {}", player.gold, player.score),
                        format!("ÖLDÜRME: {} | DALGA: {}", player.kills, wave-1),
                        format!("KOMBO: x{}", player.combo),
                        format!("CAN REGEN: +{:.2}/s | DASH CD: {:.2}s", player.hp_regen, (1.0-player.dash_reduction).max(0.3)),
                        format!("MERMI BOYUT: +{:.0} | KOMBO: +{:.1}s", player.bullet_size, player.combo_bonus),
                        format!("ALTIN BONUS: +{}/kill | ENERJİ REGEN: +{:.0}/s", player.shop_gold_lv, player.shop_energy_regen_lv as f32*2.0),
                    ];
                    for (i, line) in stats_lines.iter().enumerate() {
                        draw_text(line, bx+15.0, by+50.0+i as f32*21.0, 15.0, Color::new(0.7,0.8,1.0,0.9));
                    }
                }

                if boss.is_none() && asteroids.len() < 3 && enemies.is_empty() && is_key_pressed(KeyCode::Escape) {
                    state = GameState::Shopping; next_frame().await; continue;
                }
            }
        }
        next_frame().await;
    }
}
