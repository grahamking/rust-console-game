//use log::debug;
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

mod entity;
use entity::Entity;

const PLAYER_LIVES: usize = 3;

const DEBUG: bool = true;

const FRAME_GAP_MS: u64 = 50;
const BANNER_PAUSE_S: u64 = 1;
const HIT_PAUSE_MS: u64 = 600;

const PLAYER_1_ID: usize = 0;
const PLAYER_2_ID: usize = 1;

trait Output {
    // Setup graphics
    fn init(&mut self) -> Result<(), Box<dyn Error>>;

    // Width and height of display, in whatever units makes sense
    fn dimensions(&self) -> Result<(u16, u16), Box<dyn Error>>;

    // Render start positions
    fn start(&mut self, p1: &Entity, p2: &Entity) -> Result<(), Box<dyn Error>>;

    // Update display, called every frame
    fn render(&mut self, p1: &Entity, p2: &Entity, m: &[Entity]) -> Result<(), Box<dyn Error>>;

    // Display a banner, possibly multi-line. Caller must reset screen afterwards.
    fn banner(&mut self, msg: &[&str]) -> Result<(), Box<dyn Error>>;

    // Draw a string. Debug, unused.
    fn print(&mut self, x: u16, y: u16, s: &str) -> Result<(), Box<dyn Error>>;

    // Reset screen, quit
    fn cleanup(&mut self) -> Result<(), Box<dyn Error>>;
}

struct GameData<T: Output> {
    board_width: u16,
    board_height: u16,
    out: T,
    players: Vec<Entity>,
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

    let (w, h) = out.dimensions()?;
    let p1 = entity::new_player("Player 1", "1".to_string(), 1, w, h);
    let p2 = entity::new_player("Player 2", "2".to_string(), 2, w, h);

    let mut gd = GameData {
        board_width: w,
        board_height: h,
        out,
        players: vec![p1, p2],
    };

    gd.out.banner(&[
        "Player 1   Move: w a s d.    Fire modifier: Shift",
        "Player 2   Move: Arrow keys. Fire modifier: Alt  ",
        "Esc to quit                                      ",
        "Press any key to start",
    ])?;

    while gd.players[PLAYER_1_ID].is_alive && gd.players[PLAYER_2_ID].is_alive {
        input::wait_for_keypress();
        if game_loop(&mut gd)? {
            break; // user pressed quit
        }

        // game over?
        if gd.players.iter().any(|p| !p.is_alive) {
            break;
        }

        // a player must have been hit, freeze the screen
        thread::sleep(Duration::from_millis(HIT_PAUSE_MS));
        for p in gd.players.iter() {
            if p.is_hit {
                gd.out.banner(&[
                    &format!("{} is hit!", p.name.as_ref().unwrap()),
                    "Press any key to continue",
                ])?;
                thread::sleep(Duration::from_secs(BANNER_PAUSE_S));
                break;
            }
        }
    }
    if gd.players.iter().any(|p| !p.is_alive) {
        winner_banner(&mut gd)?;
    }

    gd.out.cleanup()?;

    Ok(())
}

fn winner_banner<T: Output>(gd: &mut GameData<T>) -> Result<(), Box<dyn Error>> {
    let mut winner = None;
    for p in gd.players.iter() {
        if p.is_alive {
            winner = p.name.as_ref();
        }
    }
    if winner.is_some() {
        gd.out.banner(&[&format!("{} wins!", winner.unwrap())])?;
        thread::sleep(Duration::from_secs(2));
    }
    Ok(())
}

// Returns Ok(true) when it's time to exit
fn game_loop<T: Output>(gd: &mut GameData<T>) -> Result<bool, Box<dyn Error>> {
    let out = &mut gd.out;
    let (w, h) = (gd.board_width, gd.board_height);
    for p in gd.players.iter_mut() {
        p.is_hit = false;
    }

    to_start_positions(&mut gd.players, w, h);
    out.start(&gd.players[PLAYER_1_ID], &gd.players[PLAYER_2_ID])?;
    let missile_range: i16 = w as i16 / 4;

    let mut missiles: Vec<Entity> = Vec::new();

    let mut is_hit = false;
    let mut is_quit = false;
    while !is_hit && !is_quit {
        for ie in input::events()?.iter() {
            match ie {
                InputEvent::Quit => {
                    is_quit = true;
                    break;
                }
                InputEvent::Move { entity_id, dir } if *entity_id == 1 => {
                    gd.players[PLAYER_1_ID].dir = *dir
                }
                InputEvent::Move { entity_id, dir } if *entity_id == 2 => {
                    gd.players[PLAYER_2_ID].dir = *dir
                }
                InputEvent::Fire { entity_id, kind } => {
                    let p = match entity_id {
                        1 => &gd.players[PLAYER_1_ID],
                        2 => &gd.players[PLAYER_2_ID],
                        _ => panic!("impossible player id"),
                    };
                    let (mut pos, dir) = (p.pos, p.dir);
                    if dir == Dir::None {
                        continue; // can't fire when not moving
                    }

                    // move ahead of the player
                    pos = pos.moved(dir).moved(dir);
                    if !is_on_board(pos.x, pos.y, w, h) {
                        continue;
                    }

                    match kind {
                        input::FireKind::Up => {
                            missiles.push(entity::new_missile(
                                pos,
                                dir,
                                missile_range,
                                p.color_idx,
                                w,
                                h,
                            ));
                        }
                        input::FireKind::Down => {
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
                        }
                        input::FireKind::Left => (),
                        input::FireKind::Right => (),
                    }
                }
                _ => panic!("entity_id not 1 or 2, shouldn't happen"),
            }
        }
        if is_quit {
            continue;
        }

        for p in gd.players.iter_mut() {
            p.update();
        }
        for m in missiles.iter_mut() {
            m.update();
            for p in gd.players.iter_mut() {
                if m.does_hit(p) {
                    p.hit();
                    is_hit = true;
                }
            }
        }
        out.render(
            &gd.players[PLAYER_1_ID],
            &gd.players[PLAYER_2_ID],
            &missiles,
        )?;
        missiles.retain(|m| m.is_alive);
        thread::sleep(Duration::from_millis(FRAME_GAP_MS));
    }
    Ok(is_quit)
}

fn to_start_positions(players: &mut Vec<Entity>, display_width: u16, display_height: u16) {
    let (w, h) = (display_width, display_height);
    let quarter = w / 4;

    players[PLAYER_1_ID].pos = Pos {
        x: quarter,
        y: h / 2,
    };
    players[PLAYER_1_ID].dir = Dir::None;

    players[PLAYER_2_ID].pos = Pos {
        x: quarter * 3,
        y: h / 2,
    };
    players[PLAYER_2_ID].dir = Dir::None;
}

fn is_on_board(cx: u16, cy: u16, w: u16, h: u16) -> bool {
    1 <= cx && cx < w - 1 && 2 <= cy && cy < h - 2
}
