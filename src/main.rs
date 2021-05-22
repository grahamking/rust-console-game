use crossterm::event::KeyCode;
use crossterm::{cursor, event, execute, queue, style, terminal}; // QueueableCommand};
use std::error::Error;
use std::io::{stdout, Stdout, Write};
use std::sync::MutexGuard;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, Mutex};
use std::{thread, time::Duration};

const FRAME_GAP_MS: u64 = 40;
const START_LIVES: usize = 5;

#[derive(Copy, Clone)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Copy, Clone)]
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
        }
    }
    fn reverse(&mut self, dir: Dir) -> Dir {
        let new_dir = match dir {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        };
        self.mov(new_dir);
        new_dir
    }
    fn hit(&self, pos: Pos) -> bool {
        self.x == pos.x && self.y == pos.y
    }
}

struct Character {
    name: String,
    pos: Pos,
    dir: Dir,
    lives: usize,
}
impl Character {
    fn new(name: String, x: u16, y: u16, dir: Dir) -> Character {
        Character {
            pos: Pos { x, y },
            name,
            dir,
            lives: 5,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let g = Game::new()?;
    let winner = g.main_loop();
    match winner {
        Ok(None) => Ok(()),
        Ok(Some(winner)) => {
            println!("\n{} WINS !!\n", winner);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

struct Game {
    data: GameData,
}

struct GameData {
    w: u16,
    h: u16,
    stdout: Mutex<Stdout>,
    players: Mutex<(Character, Character)>,
    missiles: Mutex<Vec<Character>>,
    is_running: AtomicBool,
    winner: Mutex<Option<String>>,
}

impl Game {
    fn new() -> Result<Arc<Game>, Box<dyn Error>> {
        let (w, h) = terminal::size()?;
        let quarter = w / 4;
        let gd = GameData {
            w,
            h,
            stdout: Mutex::new(stdout()),
            players: Mutex::new((
                Character::new("Player One".to_string(), quarter, h / 2, Dir::Right),
                Character::new("Player Two".to_string(), quarter * 3, h / 2, Dir::Left),
            )),
            missiles: Mutex::new(Vec::new()),
            is_running: AtomicBool::new(true),
            winner: Mutex::new(None),
        };
        let g = Arc::new(Game { data: gd });

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

    // Returns the player who won, or None if quit from keyboard
    fn main_loop(&self) -> Result<Option<String>, Box<dyn Error>> {
        let mut move_players = true;

        while self.data.is_running.load(Ordering::Relaxed) {
            if self.draw_frame(move_players) {
                break;
            }
            move_players = !move_players;
            thread::sleep(Duration::from_millis(FRAME_GAP_MS));
        }
        self.data.is_running.store(false, Ordering::Relaxed);
        // wait for input_loop to exit
        thread::sleep(Duration::from_millis(FRAME_GAP_MS + 5));

        self.cleanup();

        Ok(self.data.winner.lock().unwrap().take())
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
                self.fire(p.pos, p.dir);
            }

            // player two keys
            KeyCode::Up => self.data.players.lock().unwrap().1.dir = Dir::Up,
            KeyCode::Down => self.data.players.lock().unwrap().1.dir = Dir::Down,
            KeyCode::Left => self.data.players.lock().unwrap().1.dir = Dir::Left,
            KeyCode::Right => self.data.players.lock().unwrap().1.dir = Dir::Right,
            KeyCode::Char('m') => {
                let p = &self.data.players.lock().unwrap().1;
                self.fire(p.pos, p.dir);
            }
            _ => (),
        }
        Ok(())
    }

    fn draw_frame(&self, move_players: bool) -> bool {
        let mut is_end = false;
        let data = &self.data;
        let mut stdout = data.stdout.lock().unwrap();
        let mut p_lock = data.players.lock().unwrap();
        for m in data.missiles.lock().unwrap().iter_mut() {
            animate(&mut stdout, m, "*", data.w, data.h);
            if m.pos.hit(p_lock.0.pos) {
                queue!(stdout, style::Print("BOOM")).unwrap();
                p_lock.0.lives -= 1;
                if p_lock.0.lives == 0 {
                    is_end = true;
                    *data.winner.lock().unwrap() = Some(p_lock.1.name.clone());
                    break;
                }
                self.draw_status(&mut stdout, p_lock.0.lives, p_lock.1.lives)
                    .unwrap();
            }
            if m.pos.hit(p_lock.1.pos) {
                queue!(stdout, style::Print("BOOM")).unwrap();
                p_lock.1.lives -= 1;
                if p_lock.1.lives == 0 {
                    is_end = true;
                    *data.winner.lock().unwrap() = Some(p_lock.0.name.clone());
                    break;
                }
                self.draw_status(&mut stdout, p_lock.0.lives, p_lock.1.lives)
                    .unwrap();
            }
        }
        if move_players {
            animate(&mut stdout, &mut p_lock.0, "1", data.w, data.h);
            animate(&mut stdout, &mut p_lock.1, "2", data.w, data.h);
        }
        stdout.flush().unwrap();
        is_end
    }

    fn fire(&self, start_pos: Pos, dir: Dir) {
        self.data.missiles.lock().unwrap().push(Character {
            name: "missile".to_string(),
            pos: start_pos,
            dir,
            lives: 0,
        });
    }

    fn draw_status(
        &self,
        stdout: &mut MutexGuard<Stdout>,
        p1: usize,
        p2: usize,
    ) -> Result<(), Box<dyn Error>> {
        let third_width = self.data.w / 3;
        let name_size = "Player X: ".len() as u16 + START_LIVES as u16;
        let player1 = "Player 1: ".to_string() + &"#".repeat(p1) + &" ".repeat(START_LIVES - p1);
        let player2 = "Player 2: ".to_string() + &"#".repeat(p2) + &" ".repeat(START_LIVES - p2);
        queue!(
            stdout,
            cursor::MoveTo(third_width - name_size / 2, 0),
            style::Print(player1),
            cursor::MoveTo(2 * third_width - name_size / 2, 0),
            style::Print(player2),
        )?;
        Ok(())
    }

    fn draw_board(&self) -> Result<(), Box<dyn Error>> {
        let (w, h) = terminal::size()?;
        let top = 1;
        let bottom = h - 2;
        let mut stdout = self.data.stdout.lock().unwrap();

        self.draw_status(&mut stdout, START_LIVES, START_LIVES)?;

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
        let players = self.data.players.lock().unwrap();
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
}

fn animate(stdout: &mut Stdout, p: &mut Character, s: &str, w: u16, h: u16) {
    queue!(stdout, cursor::MoveTo(p.pos.x, p.pos.y), style::Print(" "),).unwrap();
    p.pos.mov(p.dir);
    if !is_on_board(p, w, h) {
        p.dir = p.pos.reverse(p.dir);
    }
    queue!(stdout, cursor::MoveTo(p.pos.x, p.pos.y), style::Print(s),).unwrap();
}

fn is_on_board(c: &Character, w: u16, h: u16) -> bool {
    1 <= c.pos.x && c.pos.x < w - 1 && 2 <= c.pos.y && c.pos.y < h - 2
}

fn line<T: Write>(writer: &mut T, width: u16) -> Result<(), crossterm::ErrorKind> {
    queue!(writer, style::Print("-".repeat(width as usize)))
}
