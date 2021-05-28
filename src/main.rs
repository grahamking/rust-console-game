//use log::debug;
use simplelog::{Config, LevelFilter, WriteLogger};
use std::error::Error;
use std::fs::File;
use std::thread;
use std::time::Duration;

// All modules have to be declared here so that they can see each other
mod console;

mod dir;
use dir::Dir;

mod input;
use input::InputEvent;

const DEBUG: bool = false;

const FRAME_GAP_MS: u64 = 40;
const BANNER_PAUSE_S: u64 = 1;
const EXPLODE_FRAMES: u16 = 4;

trait Output {
    // Setup graphics
    fn init(&mut self) -> Result<(), Box<dyn Error>>;

    // Width and height of display, in whatever units makes sense
    fn dimensions(&self) -> Result<(u16, u16), Box<dyn Error>>;

    // Render start positions
    fn start(&mut self, p1: &Player, p2: &Player) -> Result<(), Box<dyn Error>>;

    // Update display, called every frame
    fn render(&mut self, p1: &Player, p2: &Player, m: &[Missile]) -> Result<(), Box<dyn Error>>;

    // Display a banner, possibly multi-line. Caller must reset screen afterwards.
    fn banner(&mut self, msg: &[&str]) -> Result<(), Box<dyn Error>>;

    // Draw a string. Debug, unused.
    fn print(&mut self, x: u16, y: u16, s: &str) -> Result<(), Box<dyn Error>>;

    // Reset screen, quit
    fn cleanup(&mut self) -> Result<(), Box<dyn Error>>;
}

struct Missile {
    w: u16, // board width
    h: u16, // board height
    prev: Pos,
    pos: Option<Pos>,
    dir: Dir,
    range: i16,
    explode_timer: Option<u16>,
    //explosion_pos: Option<Vec<Pos>>,
}

impl Missile {
    fn update(&mut self, w: u16, h: u16) {
        if self.pos.is_none() {
            return;
        }
        if self.explode_timer.is_some() {
            self.update_explosion();
        } else {
            self.update_movement(w, h);
        }
    }

    fn update_movement(&mut self, w: u16, h: u16) {
        let mut p = self.pos.unwrap();
        self.prev = p;
        p = p.moved(self.dir).moved(self.dir); // move twice as fast as player
        if is_on_board(p.x, p.y, w, h) {
            self.pos = Some(p);
        } else {
            self.pos = None;
        }
        self.range -= 2;

        // trigger explosion
        if self.range <= 0 && !self.is_exploding() {
            self.explode_timer = Some(EXPLODE_FRAMES);
        }
    }

    fn update_explosion(&mut self) {
        *self.explode_timer.as_mut().unwrap() -= 1;
        if self.explode_timer.unwrap() == 0 {
            self.pos = None; // mark for deletion
        }
    }

    fn hit(&self, p: &Player) -> bool {
        match self.pos {
            None => {
                return false;
            }
            Some(pos) => {
                // missiles move two squares per tick, so check both
                if pos.hit(p.pos) || pos.moved(self.dir.opposite()).hit(p.pos) {
                    return true;
                }
            }
        }
        if self.is_exploding() {
            for pos in self.explosion() {
                if pos.hit(p.pos) {
                    return true;
                }
            }
        }
        false
    }

    // Keep this missile on the board?
    fn is_alive(&self) -> bool {
        self.pos.is_some() && (self.range > 0 || self.is_exploding())
    }

    fn is_exploding(&self) -> bool {
        self.explode_timer.is_some()
    }

    // The positions affected by an explosion of this missile
    fn explosion(&self) -> Vec<crate::Pos> {
        //if self.explosion_pos.is_some() {
        //   return self.explosion_pos.as_ref().unwrap();
        //}
        let mut v = Vec::new(); // todo cache it
        if self.pos.is_none() {
            return v;
        }
        let pos = self.pos.unwrap();
        let left = pos.x - 2;
        let top = pos.y - 2;
        for x in left..=left + 5 {
            for y in top..=top + 5 {
                if is_on_board(x, y, self.w, self.h) {
                    v.push(Pos { x, y });
                }
            }
        }
        v
    }
}

struct Player {
    name: String,
    pos: Pos,
    dir: Dir,
    lives: usize,
}
impl Player {
    fn new(name: &str) -> Player {
        Player {
            name: name.to_string(),
            pos: Pos { x: 0, y: 0 },
            dir: Dir::None,
            lives: 5,
        }
    }

    fn update(&mut self, w: u16, h: u16) {
        let next_pos = self.pos.moved(self.dir);
        if is_on_board(next_pos.x, next_pos.y, w, h) {
            self.pos = next_pos;
        } else {
            // bounce back onto the board
            self.dir = self.dir.opposite();
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Pos {
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
    fn hit(&self, pos: Pos) -> bool {
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
    let mut p1 = Player::new("Player 1");
    let mut p2 = Player::new("Player 2");
    out.banner(&[
        "Player 1   Move: w a s d, Fire: Tab ",
        "Player 2   Move: Arrow keys. Fire: m",
        "Esc to quit                         ",
        "Press any key to start",
    ])?;

    while p1.lives > 0 && p2.lives > 0 {
        input::wait_for_keypress();
        if game_loop(&mut out, &mut p1, &mut p2, w, h)? {
            break; // user pressed quit
        }
    }

    let winner = if p1.lives == 0 {
        Some(p2.name)
    } else if p2.lives == 0 {
        Some(p1.name)
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
    p1: &mut Player,
    p2: &mut Player,
    w: u16,
    h: u16,
) -> Result<bool, Box<dyn Error>> {
    to_start_positions(p1, p2, w, h);
    out.start(&p1, &p2)?;
    let missile_range: i16 = w as i16 / 4;

    let mut missiles: Vec<Missile> = Vec::new();

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
                    let (mut pos, dir) = match player_id {
                        1 => (p1.pos, p1.dir),
                        2 => (p2.pos, p2.dir),
                        _ => panic!("impossible player id"),
                    };
                    if dir == Dir::None {
                        continue; // can't fire when not moving
                    }

                    // move ahead of the player
                    pos = pos.moved(dir).moved(dir);
                    if !is_on_board(pos.x, pos.y, w, h) {
                        continue;
                    }

                    missiles.push(Missile {
                        w,
                        h,
                        prev: pos,
                        pos: Some(pos),
                        range: missile_range,
                        explode_timer: None,
                        dir,
                    });
                }
                _ => panic!("player_id not 1 or 2, shouldn't happen"),
            }
        }
        if is_exit || is_done {
            continue;
        }

        p1.update(w, h);
        p2.update(w, h);
        for m in missiles.iter_mut() {
            m.update(w, h);
            if m.hit(p1) {
                p1.lives -= 1;
                out.banner(&["Player 1 is hit!", "Press any key to continue"])?;
                thread::sleep(Duration::from_secs(BANNER_PAUSE_S));
                is_done = true;
                break;
            }
            if m.hit(p2) {
                p2.lives -= 1;
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

        missiles.retain(|m| m.is_alive());

        thread::sleep(Duration::from_millis(FRAME_GAP_MS));
    }

    Ok(is_exit)
}

fn to_start_positions(p1: &mut Player, p2: &mut Player, display_width: u16, display_height: u16) {
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
