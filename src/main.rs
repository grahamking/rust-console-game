use crossterm::event::KeyCode;
use crossterm::{cursor, event, execute, queue, style, terminal}; // QueueableCommand};
use std::error::Error;
use std::io::{stdout, Stdout, Write};
use std::sync::{Arc, Mutex};
use std::{thread, time};

enum Dir {
    Up,
    Down,
    Left,
    Right,
}

enum Player {
    One,
    Two,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut g = Game::new()?;
    g.main_loop()?;

    Ok(())
}

struct Game {
    stdout: Arc<Mutex<Stdout>>,
    w: u16,
    h: u16,
}

impl Game {
    fn new() -> Result<Game, Box<dyn Error>> {
        let (w, h) = terminal::size()?;
        let g = Game {
            stdout: Arc::new(Mutex::new(stdout())),
            w,
            h,
        };

        terminal::enable_raw_mode()?;
        queue!(
            g.stdout.lock().unwrap(),
            terminal::Clear(terminal::ClearType::All),
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;

        draw_board(g.stdout.clone())?;

        let thread_stdout = g.stdout.clone();
        let _ = thread::spawn(move || mover(thread_stdout));

        Ok(g)
    }

    fn main_loop(&mut self) -> Result<(), Box<dyn Error>> {
        'top: loop {
            if let event::Event::Key(e) = event::read()? {
                match e.code {
                    // quit
                    KeyCode::Char('q') => break 'top,

                    // player one keys
                    KeyCode::Up => self.mov(Player::One, Dir::Up),
                    KeyCode::Down => self.mov(Player::One, Dir::Down),
                    KeyCode::Left => self.mov(Player::One, Dir::Left),
                    KeyCode::Right => self.mov(Player::One, Dir::Right),
                    KeyCode::Char('m') => self.fire(Player::One),

                    // player two keys
                    KeyCode::Char('w') => self.mov(Player::Two, Dir::Up),
                    KeyCode::Char('s') => self.mov(Player::Two, Dir::Down),
                    KeyCode::Char('a') => self.mov(Player::Two, Dir::Left),
                    KeyCode::Char('d') => self.mov(Player::Two, Dir::Right),
                    KeyCode::Tab => self.fire(Player::Two),

                    _ => println!("KEY: {:?}", e.code),
                }
            }
        }

        Ok(())
    }

    fn mov(&self, player: Player, dir: Dir) {
        todo!();
    }

    fn fire(&self, player: Player) {
        todo!();
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let mut sl = self.stdout.lock().unwrap();
        execute!(
            sl,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, self.h - 1),
            cursor::Show
        )
        .unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}

// action thread
fn mover(stdout: Arc<Mutex<Stdout>>) {
    let (w, _h) = terminal::size().unwrap();
    for i in 1..w - 2 {
        let mut s_tmp = stdout.lock().unwrap();
        queue!(s_tmp, cursor::MoveTo(i, 3), style::Print("1")).unwrap();
        s_tmp.flush().unwrap();
        drop(s_tmp);
        thread::sleep(time::Duration::from_millis(30));
        queue!(
            stdout.lock().unwrap(),
            cursor::MoveTo(i, 3),
            style::Print(" ")
        )
        .unwrap();
    }
}

fn draw_board(stdout: Arc<Mutex<Stdout>>) -> Result<(), Box<dyn Error>> {
    let (w, h) = terminal::size()?;
    let mut stdout = stdout.lock().unwrap();
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
    stdout.flush()?;

    Ok(())
}

fn line<T: Write>(writer: &mut T, width: u16) -> Result<(), crossterm::ErrorKind> {
    queue!(writer, style::Print("-".repeat(width as usize)))
}
