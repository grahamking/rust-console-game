use crossterm::event::KeyCode;
use crossterm::{cursor, event, execute, queue, style, terminal}; // QueueableCommand};
use log::debug;
use simplelog::{Config, LevelFilter, WriteLogger};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{stdout, Stdout, Write};
use std::sync::MutexGuard;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, Mutex};
use std::{thread, time::Duration};

const FRAME_GAP_MS: u64 = 40;
const START_LIVES: usize = 5;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Dir {
    None,
    Up,
    Down,
    Left,
    Right,
}

impl Display for Dir {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match *self {
            Dir::None => write!(f, "None"),
            Dir::Up => write!(f, "Up"),
            Dir::Down => write!(f, "Down"),
            Dir::Left => write!(f, "Left"),
            Dir::Right => write!(f, "Right"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Pos {
    x: u16,
    y: u16,
}
impl Pos {
    fn mov(&mut self, dir: Dir) {
        match dir {
            Dir::Up => {
                if self.y > 0 {
                    self.y -= 1
                }
            }
            Dir::Down => self.y += 1,
            Dir::Left => {
                if self.x > 0 {
                    self.x -= 1
                }
            }
            Dir::Right => self.x += 1,
            Dir::None => (),
        }
    }
    fn reverse(&mut self, dir: Dir) -> Dir {
        let new_dir = match dir {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
            Dir::None => Dir::None,
        };
        self.mov(new_dir);
        new_dir
    }
    fn hit(&self, pos: Pos) -> bool {
        self.x == pos.x && self.y == pos.y
    }
}

struct Player {
    name: String,
    pos: Pos,
    dir: Dir,
    lives: usize,
}
impl Player {
    fn new(name: String, x: u16, y: u16, dir: Dir) -> Player {
        Player {
            pos: Pos { x, y },
            name,
            dir,
            lives: 5,
        }
    }
}

struct Missile {
    pos: Pos,
    dir: Dir,
}

fn main() -> Result<(), Box<dyn Error>> {
    WriteLogger::init(
        LevelFilter::Trace,
        Config::default(),
        File::create("console.log").unwrap(),
    )?;

    let g = Game::new()?;
    g.main_loop()
}

struct Game {
    data: GameData,
}

struct GameData {
    w: u16,
    h: u16,
    stdout: Mutex<Stdout>,
    players: Mutex<(Player, Player)>,
    missiles: Mutex<Vec<Missile>>,
    is_running: AtomicBool,
    winner: Mutex<Option<String>>,
    hit: Mutex<Option<String>>,
}

impl Game {
    fn new() -> Result<Arc<Game>, Box<dyn Error>> {
        let (w, h) = terminal::size()?;
        let gd = GameData {
            w,
            h,
            stdout: Mutex::new(stdout()),
            players: Mutex::new((
                Player::new("Player One".to_string(), 0, 0, Dir::None),
                Player::new("Player Two".to_string(), 0, 0, Dir::None),
            )),
            missiles: Mutex::new(Vec::new()),
            is_running: AtomicBool::new(true),
            winner: Mutex::new(None),
            hit: Mutex::new(None),
        };
        let g = Arc::new(Game { data: gd });
        g.set_start_positions();

        terminal::enable_raw_mode()?;
        queue!(
            g.data.stdout.lock().unwrap(),
            terminal::Clear(terminal::ClearType::All),
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;

        g.draw_board()?;

        let g3 = g.clone();
        let _ = thread::spawn(move || g3.input_loop());

        Ok(g)
    }

    fn main_loop(&self) -> Result<(), Box<dyn Error>> {
        let mut move_players = true;

        while self.data.is_running.load(Ordering::Relaxed) {
            let (is_end, is_reset) = self.draw_frame(move_players);
            if is_end {
                break;
            }
            if is_reset {
                let mut hit = self.data.hit.lock().unwrap();
                self.draw_center_text(&format!("{} HIT !", hit.as_ref().unwrap()));
                thread::sleep(Duration::from_secs(1));
                *hit = None;
                self.reset_board()?;
            }
            move_players = !move_players;
            thread::sleep(Duration::from_millis(FRAME_GAP_MS));
        }
        self.data.is_running.store(false, Ordering::Relaxed);

        let winner = self.data.winner.lock().unwrap().take().unwrap();
        self.draw_center_text(&format!("{} WINS !", winner));
        thread::sleep(Duration::from_secs(2));

        self.cleanup();
        Ok(())
    }

    fn cleanup(&self) {
        let mut sl = self.data.stdout.lock().unwrap();
        execute!(
            sl,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, self.data.h - 1),
            cursor::Show
        )
        .unwrap();
        terminal::disable_raw_mode().unwrap();
    }

    fn input_loop(&self) {
        while self.data.is_running.load(Ordering::Relaxed) {
            if event::poll(Duration::from_millis(FRAME_GAP_MS)).unwrap() {
                self.read_key().unwrap();
            }
        }
    }

    fn read_key(&self) -> Result<(), Box<dyn Error>> {
        let ev = event::read()?;
        let e = match ev {
            event::Event::Key(e) => e,
            _ => {
                return Ok(());
            }
        };
        match e.code {
            // quit
            KeyCode::Esc => {
                // make the main loop and hence the program exit
                self.data.is_running.store(false, Ordering::Relaxed);
            }

            // player one keys
            KeyCode::Char('w') => self.data.players.lock().unwrap().0.dir = Dir::Up,
            KeyCode::Char('s') => self.data.players.lock().unwrap().0.dir = Dir::Down,
            KeyCode::Char('a') => self.data.players.lock().unwrap().0.dir = Dir::Left,
            KeyCode::Char('d') => self.data.players.lock().unwrap().0.dir = Dir::Right,
            KeyCode::Tab => {
                let p = &self.data.players.lock().unwrap().0;
                if p.dir != Dir::None {
                    self.fire(p.pos, p.dir);
                }
            }

            // player two keys
            KeyCode::Up => self.data.players.lock().unwrap().1.dir = Dir::Up,
            KeyCode::Down => self.data.players.lock().unwrap().1.dir = Dir::Down,
            KeyCode::Left => self.data.players.lock().unwrap().1.dir = Dir::Left,
            KeyCode::Right => self.data.players.lock().unwrap().1.dir = Dir::Right,
            KeyCode::Char('m') => {
                let p = &self.data.players.lock().unwrap().1;
                if p.dir != Dir::None {
                    self.fire(p.pos, p.dir);
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn draw_frame(&self, move_players: bool) -> (bool, bool) {
        let (mut is_end, mut is_reset) = (false, false);
        let data = &self.data;
        let mut stdout = data.stdout.lock().unwrap();
        let mut p_lock = data.players.lock().unwrap();
        let mut missiles = data.missiles.lock().unwrap();
        for m in missiles.iter_mut() {
            animate_missile(&mut stdout, m, "*", data.w, data.h);
            if m.pos.hit(p_lock.0.pos) {
                p_lock.0.lives -= 1;
                if p_lock.0.lives == 0 {
                    is_end = true;
                    *data.winner.lock().unwrap() = Some(p_lock.1.name.clone());
                    break;
                }
                *data.hit.lock().unwrap() = Some(p_lock.0.name.clone());
                is_reset = true;
                break;
            }
            if m.pos.hit(p_lock.1.pos) {
                p_lock.1.lives -= 1;
                if p_lock.1.lives == 0 {
                    is_end = true;
                    *data.winner.lock().unwrap() = Some(p_lock.0.name.clone());
                    break;
                }
                *data.hit.lock().unwrap() = Some(p_lock.1.name.clone());
                is_reset = true;
                break;
            }
        }
        missiles.retain(|m| m.dir != Dir::None);

        if move_players && !is_end && !is_reset {
            animate_player(&mut stdout, &mut p_lock.0, "1", data.w, data.h);
            animate_player(&mut stdout, &mut p_lock.1, "2", data.w, data.h);
        }
        stdout.flush().unwrap();

        (is_end, is_reset)
    }

    fn fire(&self, start_pos: Pos, dir: Dir) {
        self.data.missiles.lock().unwrap().push(Missile {
            pos: start_pos,
            dir,
        });
    }

    fn draw_status(
        &self,
        stdout: &mut MutexGuard<Stdout>,
        p1: usize,
        p2: usize,
    ) -> Result<(), Box<dyn Error>> {
        let third_width = self.data.w / 3;
        let player1 = format!("Player 1: {} / {}", p1, START_LIVES);
        let player2 = format!("Player 2: {} / {}", p2, START_LIVES);
        queue!(
            stdout,
            cursor::MoveTo(third_width - player1.len() as u16 / 2, 0),
            style::Print(player1),
            cursor::MoveTo(2 * third_width - player2.len() as u16 / 2, 0),
            style::Print(player2),
        )?;
        Ok(())
    }

    fn set_start_positions(&self) {
        let (w, h) = (self.data.w, self.data.h);
        let quarter = w / 4;
        let mut p_lock = self.data.players.lock().unwrap();

        p_lock.0.dir = Dir::None;
        p_lock.0.pos = Pos {
            x: quarter,
            y: h / 2,
        };

        p_lock.1.dir = Dir::None;
        p_lock.1.pos = Pos {
            x: quarter * 3,
            y: h / 2,
        };
    }

    fn reset_board(&self) -> Result<(), Box<dyn Error>> {
        {
            let mut stdout = self.data.stdout.lock().unwrap();
            queue!(stdout, terminal::Clear(terminal::ClearType::All))?;
        }
        self.data.missiles.lock().unwrap().clear();
        self.set_start_positions();
        self.draw_board()
    }

    fn draw_board(&self) -> Result<(), Box<dyn Error>> {
        let (w, h) = terminal::size()?;
        let top = 1;
        let bottom = h - 2;
        let mut stdout = self.data.stdout.lock().unwrap();
        let players = self.data.players.lock().unwrap();

        self.draw_status(&mut stdout, players.0.lives, players.1.lives)?;

        // top border
        queue!(stdout, cursor::MoveTo(0, top))?;
        line(&mut *stdout, w)?;

        // side borders
        for i in top + 1..bottom {
            queue!(
                stdout,
                cursor::MoveTo(0, i),
                style::Print("|"),
                cursor::MoveTo(w - 1, i),
                style::Print("|"),
            )?;
        }

        // bottom border
        queue!(stdout, cursor::MoveTo(0, bottom))?;
        line(&mut *stdout, w)?;

        // player start positions
        queue!(
            stdout,
            cursor::MoveTo(players.0.pos.x, players.0.pos.y),
            style::Print("1")
        )
        .unwrap();
        queue!(
            stdout,
            cursor::MoveTo(players.1.pos.x, players.1.pos.y),
            style::Print("2")
        )
        .unwrap();

        stdout.flush()?;
        Ok(())
    }

    fn draw_center_text(&self, s: &str) {
        let mut stdout = self.data.stdout.lock().unwrap();
        queue!(
            stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(self.data.w / 2 - s.len() as u16 / 2, self.data.h / 2),
            style::Print(s)
        )
        .unwrap();
        stdout.flush().unwrap();
    }
}

fn animate_player(stdout: &mut Stdout, p: &mut Player, s: &str, w: u16, h: u16) {
    let (prev_x, prev_y) = (p.pos.x, p.pos.y);
    p.pos.mov(p.dir);
    if is_on_board(p.pos.x, p.pos.y, w, h) {
        queue!(
            stdout,
            cursor::MoveTo(prev_x, prev_y),
            style::Print(" "),
            cursor::MoveTo(p.pos.x, p.pos.y),
            style::Print(s)
        )
        .unwrap();
    } else {
        debug!(
            "{} reverse before. {}, {}, {}",
            p.name, p.pos.x, p.pos.y, p.dir
        );
        // we're off the board, move back on
        p.dir = p.pos.reverse(p.dir);
        debug!(
            "{} reverse after. {}, {}, {}",
            p.name, p.pos.x, p.pos.y, p.dir
        );
    }
}

fn animate_missile(stdout: &mut Stdout, m: &mut Missile, s: &str, w: u16, h: u16) {
    let (prev_x, prev_y) = (m.pos.x, m.pos.y);
    queue!(stdout, cursor::MoveTo(prev_x, prev_y), style::Print(" ")).unwrap();
    m.pos.mov(m.dir);
    if is_on_board(m.pos.x, m.pos.y, w, h) {
        queue!(stdout, cursor::MoveTo(m.pos.x, m.pos.y), style::Print(s)).unwrap();
    } else {
        m.pos.reverse(m.dir);
        m.dir = Dir::None;
    }
}

fn is_on_board(cx: u16, cy: u16, w: u16, h: u16) -> bool {
    1 <= cx && cx < w - 1 && 2 <= cy && cy < h - 2
}

fn line<T: Write>(writer: &mut T, width: u16) -> Result<(), crossterm::ErrorKind> {
    queue!(writer, style::Print("-".repeat(width as usize)))
}
