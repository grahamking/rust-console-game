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

const PLAYER_LIVES: u32 = 3;
const PLAYER_ENERGY: u32 = 100;

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

    // Render start positions
    fn start(&mut self, p1_lives: u32, p2_lives: u32) -> Result<(), Box<dyn Error>>;

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
        let next = w.position[entity_id].moved(quantity, direction);
        if is_on_board(next, w.width, w.height) {
            w.position[entity_id] = next;
        } else if w.bounce[entity_id] {
            w.velocity[entity_id] = (quantity, direction.opposite());
        } else {
            debug!(
                "dead because not on board. {} not in {},{}",
                next, w.width, w.height,
            );
            w.alive[entity_id] = false;
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
fn collision_system(w: &mut World) {
    let ids = alive_entities(w);
    for (id1, idx) in ids.iter().enumerate() {
        let p1 = w.position[id1];
        for &id2 in ids.iter().skip(*idx) {
            if id1 == id2 {
                continue;
            }
            if p1.does_hit(w.position[id2]) {
                debug!("{} hits {}", w.name[id1], w.name[id2]);
                w.alive[id1] = false;
                w.alive[id2] = false;
            }
        }
    }
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
    position: Vec<Pos>,
    hitbox: Vec<Rectangle>,
    energy: Vec<u32>,
    shield: Vec<bool>,
    bounce: Vec<bool>,
    explode: Vec<(bool, bool)>, // (will explode, is exploding)
}

impl World {
    fn reset(&mut self) {
        self.name = Vec::new();
        self.alive = Vec::new();
        self.lifetime = Vec::new();
        self.sprite = Vec::new();
        self.velocity = Vec::new();
        self.position = Vec::new();
        self.hitbox = Vec::new();
        self.energy = Vec::new();
        self.shield = Vec::new();
        self.bounce = Vec::new();
        self.explode = Vec::new();

        self.add_players();
    }
    fn add_players(&mut self) {
        self.player1 = new_player(self, "Player 1".to_string(), "1".to_string(), 1);
        self.player2 = new_player(self, "Player 2".to_string(), "2".to_string(), 2);
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
        frame_num: 0,
        texture_vertical: vec![texture.clone()],
        texture_horizontal: vec![texture.clone()],
        texture_explosion: vec![None],
    });
    w.energy.push(PLAYER_ENERGY);
    w.shield.push(false);
    w.bounce.push(true);
    w.explode.push((false, false));

    // placeholders, set later in to_start_positions
    w.position.push(Pos::nil());
    w.hitbox.push(Rectangle {
        top_left: Pos::nil(),
        bottom_right: Pos::nil(),
    });

    id
}

fn new_missile(w: &mut World, start_pos: Pos, dir: Dir, color_idx: usize) {
    w.name.push(format!("Missile {}", w.name.len()));
    w.alive.push(true);
    w.lifetime.push(Lifetime::Temporary(w.missile_range));
    w.position.push(start_pos);
    w.velocity.push((2, dir));
    w.sprite.push(Sprite {
        color_idx,
        is_bold: false,
        frame_num: 0,
        texture_vertical: vec!["*".to_string()],
        texture_horizontal: vec!["*".to_string()],
        texture_explosion: vec![Some("#".to_string())],
    });
    w.energy.push(0);
    w.shield.push(false);
    w.bounce.push(false);
    w.explode.push((true, false));
    w.hitbox.push(Rectangle {
        top_left: start_pos,
        bottom_right: start_pos,
    });
}

struct Rectangle {
    top_left: Pos,
    bottom_right: Pos,
}

enum Lifetime {
    Permanent,
    Temporary(u32),
}

struct Sprite {
    frame_num: u32,
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
        "Player 1   Move: w a s d.    Fire modifier: Shift",
        "Player 2   Move: Arrow keys. Fire modifier: Alt  ",
        "Esc to quit                                      ",
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
        missile_range: width as u32 / 4,

        name: Vec::new(),
        alive: Vec::new(),
        lifetime: Vec::new(),
        sprite: Vec::new(),
        velocity: Vec::new(),
        position: Vec::new(),
        hitbox: Vec::new(),
        energy: Vec::new(),
        shield: Vec::new(),
        bounce: Vec::new(),
        explode: Vec::new(),
        // remember to add to reset() as well
    };
    world.add_players();

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
        thread::sleep(Duration::from_millis(HIT_PAUSE_MS));
        let name = if !world.alive[world.player1] {
            &world.name[world.player1]
        } else {
            &world.name[world.player2]
        };
        out.banner(&[&format!("{} is hit!", name), "Press any key to continue"])?;
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

    let mut system = vec![System::Move, System::Lifetime, System::Collision];
    let render = Render {};

    to_start_positions(w);
    out.start(w.p1_lives, w.p2_lives)?;

    let mut is_quit = false;
    while !is_quit && both_players_standing(w) {
        for ie in input::events()?.iter() {
            match ie {
                InputEvent::Quit => {
                    is_quit = true;
                    break;
                }
                InputEvent::Move { entity_id, dir } if *entity_id == 1 => {
                    w.velocity[w.player1].1 = *dir
                }
                InputEvent::Move { entity_id, dir } if *entity_id == 2 => {
                    w.velocity[w.player2].1 = *dir
                }
                InputEvent::Fire { entity_id, kind } => {
                    let id = match entity_id {
                        1 => w.player1,
                        2 => w.player2,
                        _ => panic!("impossible player id"),
                    };
                    let (mut pos, dir) = (w.position[id], w.velocity[id].1);
                    if dir == Dir::None {
                        continue; // can't fire when not moving
                    }

                    // move ahead of the player
                    pos = pos.moved(2, dir);
                    if !is_on_board(pos, w.width, w.height) {
                        continue;
                    }

                    match kind {
                        input::FireKind::Up => {
                            new_missile(w, pos, dir, w.sprite[id].color_idx);
                        }
                        input::FireKind::Down => {
                            w.shield[id] = !w.shield[id];
                        }
                        input::FireKind::Left => {
                            // TODO ray
                            /*
                            let dist_to_edge = match dir {
                                Dir::Left => pos.x - 1,
                                Dir::Right => w - 2 - pos.x,
                                Dir::Up => pos.y - 2,
                                Dir::Down => h - 2 - pos.y - 1,
                                Dir::None => 0,
                            };
                            missiles.push(entity::new_ray(
                                pos,
                                dir,
                                p.color_idx,
                                dist_to_edge,
                                w,
                                h,
                            ));
                            */
                        }
                        input::FireKind::Right => {
                            // TODO: teleport
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
    w.position[p1] = p1_pos;
    w.velocity[p1].1 = Dir::None;
    w.hitbox[p1].top_left = p1_pos;
    w.hitbox[p1].bottom_right = p1_pos;

    let p2_pos = Pos {
        x: quarter * 3,
        y: w.height / 2,
        invalid: false,
    };
    w.position[p2] = p2_pos;
    w.velocity[p2].1 = Dir::None;
    w.hitbox[p2].top_left = p2_pos;
    w.hitbox[p2].bottom_right = p2_pos;
}

fn is_on_board(pos: Pos, w: u32, h: u32) -> bool {
    !pos.invalid && 1 <= pos.x && pos.x < w - 1 && 2 <= pos.y && pos.y < h - 2
}
