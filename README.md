# Yıldız Avcısı (Star Hunter)

![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)
![Macroquad](https://img.shields.io/badge/Macroquad-0.4-blue)

A top-down space shooter built in Rust with [macroquad](https://github.com/not-fl3/macroquad).

## Gameplay

- **WASD / Arrow Keys** — Move
- **Left Click / Space** — Shoot
- **1 / 2 / 3 / 4** — Switch weapons (Normal, Spread, Homing, Laser)
- **Shift / Right Click** — Dash
- **Tab** — Stats panel
- **Esc** — Open shop (between waves) / Pause

## Features

- 100 unique enemy templates with procedural traits (size, movement, weapon, special abilities)
- 4 weapon types with energy system
- Drone companions
- Boss battles every 3 waves
- 2-page upgrade shop with 16 items
- 3 difficulty modes (Easy / Normal / Hard)
- Elite enemy variants on higher waves
- Combo system for score multipliers
- Powerups: HP, Shield, Speed, Weapon, Drone, Gold, Max HP, Fire Rate

## Build

```bash
cargo run --release
```

## Controls (Menu)

| Key | Action |
|-----|--------|
| Space / Enter | Start game |
| 1 / 2 / 3 | Select difficulty |

## Shop Items

### Page 1 (Core)
| Key | Item | Effect |
|-----|------|--------|
| 1 | CAN | Max HP +1 |
| 2 | KALKAN | Shield +0.5 |
| 3 | HIZ | Speed +0.3 |
| 4 | SILAH | Weapon Level +1 |
| 5 | DRONE | Drone +1 |
| 6 | HASAR | Damage +2 |
| 7 | ATES | Fire Rate |
| 8 | ENERJI | Max Energy +20 |

### Page 2 (Advanced)
| Key | Item | Effect |
|-----|------|--------|
| 1 | HIZ II | Speed +0.5 |
| 2 | ATES II | Fire Rate +0.02 |
| 3 | CAN REGEN | Passive HP Regen |
| 4 | DASH | Dash Cooldown -0.1s |
| 5 | MERMI | Bullet Size +1 |
| 6 | KOMBO | Combo Duration +0.5s |
| 7 | ALTIN | Gold per Kill +2 |
| 8 | ENERJI II | Energy Regen +2/s |

*Switch pages with **Q** (Page 1) / **E** (Page 2)*
