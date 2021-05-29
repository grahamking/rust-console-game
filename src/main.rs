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

mod input;
use input::InputEvent;

mod entity;
use entity::Entity;

const DEBUG: bool = false;

const FRAME_GAP_MS: u64 = 40;
const BANNER_PAUSE_S: u64 = 1;

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

#[derive(Copy, Clone, Debug)]
pub struct Pos {
    x: u16,
    y: u16,
}
impl Pos {
    // A new position moved one unit in given direction
    fn moved(&self, dir: Dir) -> Pos {
        let (mut x, mut y) = (self.x, self.y);
        match dir {
            Dir::Up => {
                if self.y > 0 {
                    y -= 1
                }
            }
            Dir::Down => y += 1,
            Dir::Left => {
                if self.x > 0 {
                    x -= 1
                }
            }
            Dir::Right => x += 1,
            Dir::None => (),
        }
        Pos { x, y }
    }
    fn does_hit(&self, pos: Pos) -> bool {
        self.x == pos.x && self.y == pos.y
    }
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
    let mut p1 = entity::new_player("Player 1", 1, w, h);
    let mut p2 = entity::new_player("Player 2", 2, w, h);
    out.banner(&[
        "Player 1   Move: w a s d, Fire: Tab ",
        "Player 2   Move: Arrow keys. Fire: m",
        "Esc to quit                         ",
        "Press any key to start",
    ])?;

    while p1.is_alive && p2.is_alive {
        input::wait_for_keypress();
        if game_loop(&mut out, &mut p1, &mut p2, w, h)? {
            break; // user pressed quit
        }
    }

    let winner = if !p1.is_alive {
        p2.name
    } else if !p2.is_alive {
        p1.name
    } else {
        None
    };
    if winner.is_some() {
        out.banner(&[&format!("{} wins!", winner.unwrap())])?;
        thread::sleep(Duration::from_secs(2));
    }

    out.cleanup()?;

    Ok(())
}

// Returns Ok(true) when it's time to exit
fn game_loop(
    out: &mut impl Output,
    p1: &mut Entity,
    p2: &mut Entity,
    w: u16,
    h: u16,
) -> Result<bool, Box<dyn Error>> {
    to_start_positions(p1, p2, w, h);
    out.start(&p1, &p2)?;
    let missile_range: i16 = w as i16 / 4;

    let mut missiles: Vec<Entity> = Vec::new();

    let mut is_exit = false;
    let mut is_done = false;
    while !is_done && !is_exit {
        for ie in input::events()?.iter() {
            match ie {
                InputEvent::Quit => {
                    is_exit = true;
                    break;
                }
                InputEvent::Move { player_id, dir } if *player_id == 1 => p1.dir = *dir,
                InputEvent::Move { player_id, dir } if *player_id == 2 => p2.dir = *dir,
                InputEvent::Fire { player_id } => {
                    let p = match player_id {
                        1 => &p1,
                        2 => &p2,
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

                    missiles.push(entity::new_missile(
                        pos,
                        dir,
                        missile_range,
                        p.color_idx,
                        w,
                        h,
                    ));
                }
                _ => panic!("player_id not 1 or 2, shouldn't happen"),
            }
        }
        if is_exit || is_done {
            continue;
        }

        p1.update();
        p2.update();
        for m in missiles.iter_mut() {
            m.update();
            if m.does_hit(p1) {
                p1.hit();
                out.banner(&["Player 1 is hit!", "Press any key to continue"])?;
                thread::sleep(Duration::from_secs(BANNER_PAUSE_S));
                is_done = true;
                break;
            }
            if m.does_hit(p2) {
                p2.hit();
                out.banner(&["Player 2 is hit!", "Press any key to continue"])?;
                thread::sleep(Duration::from_secs(BANNER_PAUSE_S));
                is_done = true;
                break;
            }
        }
        if is_exit || is_done {
            continue;
        }

        out.render(&p1, &p2, &missiles)?;

        missiles.retain(|m| m.is_alive);

        thread::sleep(Duration::from_millis(FRAME_GAP_MS));
    }

    Ok(is_exit)
}

fn to_start_positions(p1: &mut Entity, p2: &mut Entity, display_width: u16, display_height: u16) {
    let (w, h) = (display_width, display_height);
    let quarter = w / 4;

    p1.pos = Pos {
        x: quarter,
        y: h / 2,
    };
    p1.dir = Dir::None;
    p2.pos = Pos {
        x: quarter * 3,
        y: h / 2,
    };
    p2.dir = Dir::None;
}

fn is_on_board(cx: u16, cy: u16, w: u16, h: u16) -> bool {
    1 <= cx && cx < w - 1 && 2 <= cy && cy < h - 2
}
