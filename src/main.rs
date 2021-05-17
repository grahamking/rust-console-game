use crossterm::event::KeyCode;
use crossterm::{cursor, event, execute, queue, style, terminal}; // QueueableCommand};
use std::error::Error;
use std::io::{stdout, Stdout, Write};
use std::sync::{Arc, Mutex};
use std::{thread, time};

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
    pos: Pos,
    dir: Dir,
}
impl Character {
    fn new(x: u16, y: u16, dir: Dir) -> Character {
        Character {
            pos: Pos { x, y },
            dir,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut g = Game::new()?;
    g.main_loop()?;

    Ok(())
}

struct Game {
    data: Arc<GameData>,
}

struct GameData {
    w: u16,
    h: u16,
    stdout: Mutex<Stdout>,
    players: Mutex<(Character, Character)>,
    missiles: Mutex<Vec<Character>>,
}

impl Game {
    fn new() -> Result<Game, Box<dyn Error>> {
        let (w, h) = terminal::size()?;
        let quarter = w / 4;
        let gd = GameData {
            w,
            h,
            stdout: Mutex::new(stdout()),
            players: Mutex::new((
                Character::new(quarter, h / 2, Dir::Right),
                Character::new(quarter * 3, h / 2, Dir::Left),
            )),
            missiles: Mutex::new(Vec::new()),
        };
        let g = Game { data: Arc::new(gd) };

        terminal::enable_raw_mode()?;
        queue!(
            g.data.stdout.lock().unwrap(),
            terminal::Clear(terminal::ClearType::All),
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;

        g.draw_board()?;

        let gd2 = g.data.clone();
        let _ = thread::spawn(move || draw_loop(gd2));

        Ok(g)
    }

    fn main_loop(&mut self) -> Result<(), Box<dyn Error>> {
        'top: loop {
            if let event::Event::Key(e) = event::read()? {
                match e.code {
                    // quit
                    KeyCode::Char('q') => break 'top,

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
            }
        }

        Ok(())
    }

    fn fire(&self, start_pos: Pos, dir: Dir) {
        self.data.missiles.lock().unwrap().push(Character {
            pos: start_pos,
            dir,
        });
    }

    fn draw_board(&self) -> Result<(), Box<dyn Error>> {
        let (w, h) = terminal::size()?;
        let mut stdout = self.data.stdout.lock().unwrap();
        line(&mut *stdout, w)?;
        let top = 0;
        let bottom = h - 2;

        for i in top + 1..bottom {
            queue!(
                stdout,
                cursor::MoveTo(0, i),
                style::Print("|"),
                cursor::MoveTo(w - 1, i),
                style::Print("|"),
            )?;
        }
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

fn draw_loop(data: Arc<GameData>) {
    let mut move_players = true;
    loop {
        let mut stdout = data.stdout.lock().unwrap();
        let mut p_lock = data.players.lock().unwrap();
        if move_players {
            animate(&mut stdout, &mut p_lock.0, "1", data.w, data.h);
            animate(&mut stdout, &mut p_lock.1, "2", data.w, data.h);
        }
        for m in data.missiles.lock().unwrap().iter_mut() {
            animate(&mut stdout, m, "*", data.w, data.h);
            if m.pos.hit(p_lock.0.pos) {
                queue!(stdout, style::Print("BOOM"));
            }
            if m.pos.hit(p_lock.1.pos) {
                queue!(stdout, style::Print("BOOM"));
            }

            // TODO: move draw_loop to main loop
            // move keyboard to thread
            // main loop checks a sync bool every loop
            // keyboard 'q' sets the bool to mark we should exit
            // change 'q' key to Esc because q too close to player 1 keys
        }
        stdout.flush().unwrap();
        drop(p_lock);
        drop(stdout);
        move_players = !move_players;
        thread::sleep(time::Duration::from_millis(30));
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
    1 <= c.pos.x && c.pos.x < w - 1 && 1 <= c.pos.y && c.pos.y < h - 2
}

impl Drop for Game {
    fn drop(&mut self) {
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
}

fn line<T: Write>(writer: &mut T, width: u16) -> Result<(), crossterm::ErrorKind> {
    queue!(writer, style::Print("-".repeat(width as usize)))
}
