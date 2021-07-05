use log::debug;
use simplelog::{Config, LevelFilter, WriteLogger};
use std::error::Error;
use std::fs::File;
use std::thread;
use std::time::Duration;

#[macro_use]
extern crate lazy_static;

mod console;

mod dir;
use dir::Dir;

mod pos;
use pos::Pos;

mod input;
use input::InputEvent;

mod weapon;
use weapon::Weapon;

const PLAYER_LIVES: u32 = 10;
const MAX_ENERGY: u32 = 100;
const LIFETIME_RAY: u32 = 10;
const EXPLODE_DURATION: u32 = 5;
const ENERGY_MISSILE: u32 = 3;
const ENERGY_RAY: u32 = 25;
const ENERGY_SHIELD: u32 = 3; // deduct this every ENERGY_EVERY
const ENERGY_EVERY: u32 = 5; // new energy every x turns

const DEBUG: bool = true;
const DEBUG_SPEED: bool = false;

const FRAME_GAP_MS: u64 = 50;
const BANNER_PAUSE_S: u64 = 1;
const HIT_PAUSE_MS: u64 = 600;

trait Output {
    // Setup graphics
    fn init(&mut self) -> Result<(), Box<dyn Error>>;

    // Width and height of display, in whatever units makes sense
    fn dimensions(&self) -> Result<(u16, u16), Box<dyn Error>>;

    // Update display, called every frame
    fn render(&mut self, w: &mut World) -> Result<(), Box<dyn Error>>;

    // Display a banner, possibly multi-line. Caller must reset screen afterwards.
    fn banner(&mut self, msg: &[&str]) -> Result<(), Box<dyn Error>>;

    // Draw a string. Debug, unused.
    fn print(&mut self, x: u16, y: u16, s: &str) -> Result<(), Box<dyn Error>>;

    // Reset screen, quit
    fn cleanup(&mut self) -> Result<(), Box<dyn Error>>;
}

enum System {
    Move,
    Lifetime,
    Collision,
    EnergyReload(u32),
    Explode,
}

impl System {
    fn step(&mut self, world: &mut World) {
        match self {
            System::Move => {
                move_system(world);
            }
            System::Lifetime => {
                lifetime_system(world);
            }
            System::Collision => {
                collision_system(world);
            }
            System::EnergyReload(n) => {
                if *n == 0 {
                    energy_system(world);
                }
                *n = (*n + 1) % ENERGY_EVERY;
            }
            System::Explode => {
                explode_system(world);
            }
        }
    }
}

struct Render {}
impl Render {
    fn render<T: Output>(&self, w: &mut World, out: &mut T) {
        out.render(w).unwrap();
    }
}

// Use velocity to update position
fn move_system(w: &mut World) {
    for entity_id in alive_entities(w) {
        let (quantity, direction) = w.velocity[entity_id];
        if quantity == 0 {
            continue;
        }
        let next: Vec<Pos> = w.position[entity_id]
            .iter()
            .map(|p| p.moved(quantity, direction))
            .collect();
        for (idx, next_pos) in next.into_iter().enumerate() {
            if w.is_on_board(next_pos) {
                w.position[entity_id][idx] = next_pos;
            } else if w.bounce[entity_id] {
                w.velocity[entity_id] = (quantity, direction.opposite());
                break;
            } else {
                debug!(
                    "dead because not on board. {} not in {},{}",
                    next_pos, w.width, w.height,
                );
                w.alive[entity_id] = false;
                break;
            }
        }
    }
}

// Decrease lifetime, mark entities as not alive
fn lifetime_system(w: &mut World) {
    for entity_id in alive_entities(w) {
        if let Lifetime::Temporary(n) = w.lifetime[entity_id] {
            let next = n - 1;
            if next > 0 {
                w.lifetime[entity_id] = Lifetime::Temporary(next);
            } else {
                w.alive[entity_id] = false;
            }
        }
    }
}

// Check for collisions
// We don't need to worry about blocks/obstacles because move system runs first
// and prevent us comming into contact with them.
fn collision_system(w: &mut World) {
    let ids = alive_entities(w);
    'top: for (id1, idx) in ids.iter().enumerate() {
        for &id2 in ids.iter().skip(*idx) {
            if id1 == id2 {
                continue;
            }
            for p1 in w.position[id1].iter() {
                for p2 in w.position[id2].iter() {
                    if p1.does_hit(*p2) {
                        debug!("{} hits {}", w.name[id1], w.name[id2]);
                        // unshielded entites die on contact
                        if !w.shield[id1] {
                            w.alive[id1] = false;
                        }
                        if !w.shield[id2] {
                            w.alive[id2] = false;
                        }
                        break 'top;
                    }
                }
            }
        }
    }
}

// Add energy at regular intervals, deduct energy for shield
fn energy_system(w: &mut World) {
    w.energy.iter_mut().for_each(|n| {
        if *n < MAX_ENERGY {
            *n += 1;
        }
    });
    let shielded: Vec<usize> = w
        .shield
        .iter()
        .enumerate()
        .filter_map(|(id, has_shield)| if *has_shield { Some(id) } else { None })
        .collect();
    for id in shielded {
        let e = &mut w.energy[id];
        if *e > ENERGY_SHIELD {
            *e -= ENERGY_SHIELD;
        } else {
            // ran out of energy, shield off
            w.shield[id] = false;
        }
    }
}

// switch missiles to exploding
fn explode_system(w: &mut World) {
    // entity ids that:
    // - explode
    // - are not yet exploding
    // - are within EXPLODE_DURATION of their end of life
    let to_explode: Vec<usize> = w
        .explode
        .iter()
        .enumerate()
        .filter_map(|(id, (will_explode, is_exploding))| {
            if *will_explode && !is_exploding {
                Some(id)
            } else {
                None
            }
        })
        .filter(|&id| matches!(w.lifetime[id], Lifetime::Temporary(n) if n <= EXPLODE_DURATION))
        .collect();

    to_explode.iter().for_each(|&id| {
        w.explode[id].1 = true; // set is_exploding
        w.position[id] = explosion(w, w.position[id][0]);
        w.velocity[id] = (0, Dir::None);
    });
}

// Positions for an explosion originating at p
fn explosion(w: &World, p: Pos) -> Vec<Pos> {
    let mut v = Vec::with_capacity(25);
    let src_x: i32 = p.x as i32;
    let src_y: i32 = p.y as i32;
    for x in src_x - 2..=src_x + 2 {
        if x < 0 {
            continue;
        }
        for y in src_y - 2..=src_y + 2 {
            if y < 0 {
                continue;
            }
            let e = Pos {
                x: x as u32,
                y: y as u32,
                invalid: false,
            };
            if w.is_on_board(e) {
                v.push(e);
            }
        }
    }
    v
}

struct World {
    width: u32,
    height: u32,
    player1: usize,
    player2: usize,
    p1_lives: u32,
    p2_lives: u32,
    missile_range: u32,

    name: Vec<String>,
    alive: Vec<bool>,

    // components
    lifetime: Vec<Lifetime>, // how long it displays for
    sprite: Vec<Sprite>,
    velocity: Vec<(u32, Dir)>, // (quantity, direction)
    position: Vec<Vec<Pos>>,
    energy: Vec<u32>,
    shield: Vec<bool>,
    bounce: Vec<bool>,
    explode: Vec<(bool, bool)>,         // (will explode, is exploding)
    active_weapon: Vec<Option<Weapon>>, // Is player using ray or missile?
}

impl World {
    fn reset(&mut self) {
        self.name = Vec::new();
        self.alive = Vec::new();
        self.lifetime = Vec::new();
        self.sprite = Vec::new();
        self.velocity = Vec::new();
        self.position = Vec::new();
        self.energy = Vec::new();
        self.shield = Vec::new();
        self.bounce = Vec::new();
        self.explode = Vec::new();
        self.active_weapon = Vec::new();

        self.add_players();
        self.add_obstacles();
    }
    fn add_players(&mut self) {
        self.player1 = new_player(self, "Player 1".to_string(), "1".to_string(), 1);
        self.player2 = new_player(self, "Player 2".to_string(), "2".to_string(), 2);
    }
    fn add_obstacles(&mut self) {
        let x = self.width / 2;
        let third = self.height / 3;
        for y in third..third * 2 {
            let p = Pos {
                x,
                y,
                invalid: false,
            };
            new_bar(self, p, Dir::Up);
        }
    }
    fn is_on_board(&self, pos: Pos) -> bool {
        // check if off board left or right
        let x_fit = !pos.invalid && 1 <= pos.x && pos.x < self.width - 1;
        if !x_fit {
            return false;
        }
        // check if off board top and bottom
        let y_fit = 2 <= pos.y && pos.y < self.height - 2;
        if !y_fit {
            return false;
        }
        // check if hits an obstacle
        for (entity_id, _) in self
            .lifetime
            .iter()
            .enumerate()
            .filter(|(_, l)| **l == Lifetime::Solid)
        {
            // all blocks are size 1 so far so [0] is OK
            if self.position[entity_id][0].does_hit(pos) {
                return false;
            }
        }

        true
    }
}

fn both_players_alive(w: &World) -> bool {
    w.p1_lives > 0 && w.p2_lives > 0
}

fn both_players_standing(w: &World) -> bool {
    w.alive[w.player1] && w.alive[w.player2]
}

// entity ids of the living entitites
fn alive_entities(w: &World) -> Vec<usize> {
    w.alive
        .iter()
        .enumerate()
        .filter_map(|(idx, is_alive)| if *is_alive { Some(idx) } else { None })
        .collect()
}

fn new_player(w: &mut World, name: String, texture: String, color_idx: usize) -> usize {
    let id = w.name.len();
    w.name.push(name);
    w.alive.push(true);
    w.lifetime.push(Lifetime::Permanent);
    w.velocity.push((1, Dir::None));
    w.sprite.push(Sprite {
        color_idx,
        is_bold: true,
        _frame_num: 0,
        texture_vertical: vec![texture.clone()],
        texture_horizontal: vec![texture],
        texture_explosion: vec![None],
    });
    w.energy.push(MAX_ENERGY);
    w.shield.push(false);
    w.bounce.push(true);
    w.explode.push((false, false));
    w.active_weapon.push(Some(Weapon::Missile));

    // placeholder, set later in to_start_positions
    w.position.push(vec![Pos::nil()]);

    id
}

fn new_missile(w: &mut World, start_pos: Pos, dir: Dir, color_idx: usize) {
    w.name.push(format!("Missile {}", w.name.len()));
    w.alive.push(true);
    w.lifetime.push(Lifetime::Temporary(w.missile_range));
    w.position.push(vec![start_pos, start_pos.moved(1, dir)]);
    w.velocity.push((2, dir));
    w.sprite.push(Sprite {
        color_idx,
        is_bold: false,
        _frame_num: 0,
        texture_vertical: vec!["*".to_string()],
        texture_horizontal: vec!["*".to_string()],
        texture_explosion: vec![Some("#".to_string())],
    });
    w.energy.push(0);
    w.shield.push(false);
    w.bounce.push(false);
    w.explode.push((true, false));
    w.active_weapon.push(None);
}

fn new_ray(w: &mut World, start_pos: Pos, dir: Dir, color_idx: usize) {
    let dist_to_edge = match dir {
        Dir::Left => start_pos.x - 1,
        Dir::Right => w.width - 2 - start_pos.x,
        Dir::Up => start_pos.y - 2,
        Dir::Down => w.height - 2 - start_pos.y - 1,
        Dir::None => 0,
    };
    let mut positions = Vec::with_capacity(dist_to_edge as usize);
    let mut p = start_pos;
    (0..dist_to_edge).for_each(|_| {
        positions.push(p);
        p = p.moved(1, dir);
    });
    w.position.push(positions);

    w.name.push(format!("Ray {}", w.name.len()));
    w.alive.push(true);
    w.lifetime.push(Lifetime::Temporary(LIFETIME_RAY));
    w.velocity.push((1, dir));
    w.sprite.push(Sprite {
        color_idx,
        is_bold: false,
        _frame_num: 0,
        texture_vertical: vec!["|".to_string()],
        texture_horizontal: vec!["-".to_string()],
        texture_explosion: vec![None],
    });
    w.energy.push(0);
    w.shield.push(true); // does not get destroyed by a collision
    w.bounce.push(false);
    w.explode.push((false, false));
    w.active_weapon.push(None);
}

fn new_bar(w: &mut World, start_pos: Pos, dir: Dir) {
    w.name.push(format!("Bar {}", w.name.len()));
    w.alive.push(true);
    w.lifetime.push(Lifetime::Solid);
    w.position.push(vec![start_pos]);
    w.velocity.push((0, dir));
    w.sprite.push(Sprite {
        color_idx: 0,
        is_bold: false,
        _frame_num: 0,
        texture_vertical: vec!["┃".to_string()],
        texture_horizontal: vec!["━".to_string()],
        texture_explosion: vec![Some("#".to_string())],
    });
    w.energy.push(0);
    w.shield.push(true);
    w.bounce.push(false);
    w.explode.push((false, false));
    w.active_weapon.push(None);
}

#[derive(PartialEq)]
enum Lifetime {
    Solid,          // obstacle: does not get damaged, stops things
    Permanent,      // player: always on screen
    Temporary(u32), // missile/ray: displays for a while then vanishes
}

struct Sprite {
    _frame_num: u32,
    color_idx: usize,
    is_bold: bool,
    texture_vertical: Vec<String>, // actually just the char to print, but sounds fancy
    texture_horizontal: Vec<String>,
    texture_explosion: Vec<Option<String>>,
}

fn main() -> Result<(), Box<dyn Error>> {
    if DEBUG {
        WriteLogger::init(
            LevelFilter::Trace,
            Config::default(),
            File::create("hashbang.log").unwrap(),
        )?;
    }

    let mut out = console::new();
    out.init()?;

    out.banner(&[
        "R U S T   C O N S O L E   G A M E",
        "",
        "Instructions:",
        "Player 1   Move: w a s d.    Fire: Shift + direction. Toggle shield: e. Change weapon: q",
        "Player 2   Move: Arrow keys. Fire: Alt + direction. Toggle shield: .. Change weapon: ,",
        "",
        "Esc to quit",
        "Press any key to start",
    ])?;

    let (width, height) = out.dimensions()?;
    let mut world = World {
        // static
        width: width as u32,
        height: height as u32,
        player1: 0,
        player2: 0,
        p1_lives: PLAYER_LIVES,
        p2_lives: PLAYER_LIVES,
        missile_range: width as u32 / 6,

        name: Vec::new(),
        alive: Vec::new(),
        lifetime: Vec::new(),
        sprite: Vec::new(),
        velocity: Vec::new(),
        position: Vec::new(),
        energy: Vec::new(),
        shield: Vec::new(),
        bounce: Vec::new(),
        explode: Vec::new(),
        active_weapon: Vec::new(),
        // remember to add to reset() as well
    };
    world.add_players();
    world.add_obstacles();

    while both_players_alive(&world) {
        input::wait_for_keypress();
        if game_loop(&mut world, &mut out)? {
            break; // user pressed quit
        }

        // game over?
        if !both_players_alive(&world) {
            break;
        }

        // a player must have been hit, freeze the screen
        let p1a = world.alive[world.player1];
        let p2a = world.alive[world.player2];
        let name = if !p1a && !p2a {
            let mut s = world.name[world.player1].clone();
            s.push_str(" and ");
            s.push_str(&world.name[world.player2]);
            s
        } else if !p1a {
            world.name[world.player1].clone()
        } else {
            world.name[world.player2].clone()
        };
        thread::sleep(Duration::from_millis(HIT_PAUSE_MS));
        out.banner(&[&format!("{} hit!", &name), "Press any key to continue"])?;
        thread::sleep(Duration::from_secs(BANNER_PAUSE_S));

        world.reset();
    }

    if !both_players_alive(&world) {
        winner_banner(&mut world, &mut out)?;
    }
    out.cleanup()?;

    Ok(())
}

fn winner_banner<T: Output>(w: &mut World, out: &mut T) -> Result<(), Box<dyn Error>> {
    let winner = if w.p1_lives == 0 {
        &w.name[w.player2]
    } else {
        &w.name[w.player1]
    };
    out.banner(&[&format!("{} wins!", winner)])?;
    thread::sleep(Duration::from_secs(2));
    Ok(())
}

// Returns Ok(true) when it's time to exit
fn game_loop<T: Output>(w: &mut World, out: &mut T) -> Result<bool, Box<dyn Error>> {
    w.alive[w.player1] = true;
    w.alive[w.player2] = true;

    let mut system = vec![
        System::Move,
        System::Lifetime,
        System::Collision,
        System::EnergyReload(0),
        System::Explode,
    ];
    let render = Render {};

    to_start_positions(w);

    let mut is_quit = false;
    while !is_quit && both_players_standing(w) {
        for ie in input::events()?.iter() {
            match ie {
                InputEvent::Quit => {
                    is_quit = true;
                    break;
                }

                InputEvent::Move { entity_id, dir } if *entity_id == 1 => {
                    let cur = &mut w.velocity[w.player1].1;
                    if cur.opposite() == *dir {
                        *cur = Dir::None;
                    } else {
                        *cur = *dir;
                    }
                }
                InputEvent::Move { entity_id, dir } if *entity_id == 2 => {
                    let cur = &mut w.velocity[w.player2].1;
                    if cur.opposite() == *dir {
                        *cur = Dir::None;
                    } else {
                        *cur = *dir;
                    }
                }

                InputEvent::ToggleShield { entity_id } if *entity_id == 1 => {
                    w.shield[w.player1] = !w.shield[w.player1];
                }
                InputEvent::ToggleShield { entity_id } if *entity_id == 2 => {
                    w.shield[w.player2] = !w.shield[w.player2];
                }

                InputEvent::ChangeWeapon { entity_id } if *entity_id == 1 => {
                    w.active_weapon[w.player1].as_mut().unwrap().next();
                }
                InputEvent::ChangeWeapon { entity_id } if *entity_id == 2 => {
                    w.active_weapon[w.player2].as_mut().unwrap().next();
                }

                InputEvent::Fire { entity_id, kind } => {
                    let id = match entity_id {
                        1 => w.player1,
                        2 => w.player2,
                        _ => panic!("impossible player id"),
                    };
                    let mut pos = w.position[id][0];
                    let dir = match kind {
                        input::FireKind::Up => Dir::Up,
                        input::FireKind::Down => Dir::Down,
                        input::FireKind::Left => Dir::Left,
                        input::FireKind::Right => Dir::Right,
                    };

                    // move ahead of the player
                    pos = pos.moved(2, dir);
                    if !w.is_on_board(pos) {
                        continue;
                    }

                    let e = w.energy[id];
                    match w.active_weapon[id].as_ref().unwrap() {
                        Weapon::Missile => {
                            if e > ENERGY_MISSILE {
                                new_missile(w, pos, dir, w.sprite[id].color_idx);
                                w.energy[id] -= ENERGY_MISSILE;
                            }
                        }
                        Weapon::Ray => {
                            if e > ENERGY_RAY {
                                new_ray(w, pos, dir, w.sprite[id].color_idx);
                                w.energy[id] -= ENERGY_RAY;
                            }
                        }
                    }
                }
                _ => panic!("entity_id not 1 or 2, shouldn't happen"),
            }
        } // end input event handling

        if is_quit {
            continue;
        }

        for s in system.iter_mut() {
            s.step(w);
        }
        render.render(w, out);

        if DEBUG_SPEED {
            thread::sleep(Duration::from_secs(1));
        } else {
            thread::sleep(Duration::from_millis(FRAME_GAP_MS));
        }
    }

    if !w.alive[w.player1] {
        w.p1_lives -= 1;
    }
    if !w.alive[w.player2] {
        w.p2_lives -= 1;
    }

    Ok(is_quit)
}

fn to_start_positions(w: &mut World) {
    let quarter: u32 = w.width / 4;
    let p1 = w.player1;
    let p2 = w.player2;

    let p1_pos = Pos {
        x: quarter,
        y: w.height / 2,
        invalid: false,
    };
    w.position[p1][0] = p1_pos;
    w.velocity[p1].1 = Dir::None;

    let p2_pos = Pos {
        x: quarter * 3,
        y: w.height / 2,
        invalid: false,
    };
    w.position[p2][0] = p2_pos;
    w.velocity[p2].1 = Dir::None;
}
