use crossterm::{cursor, execute, queue, style, terminal};
use std::error::Error;
use std::io::{stdout, Stdout, Write};

const START_LIVES: usize = 5;
const TITLE: &str = "Hash Bang";

pub struct ConsoleOutput {
    w: u16,
    h: u16,
    prev_p1: crate::Pos,
    prev_p2: crate::Pos,
    writer: Stdout,
}

pub fn new() -> ConsoleOutput {
    let (w, h) = terminal::size().unwrap();
    ConsoleOutput {
        w,
        h,
        prev_p1: crate::Pos { x: 10, y: 10 }, // the 10/10 are never used
        prev_p2: crate::Pos { x: 10, y: 10 },
        writer: stdout(),
    }
}

impl ConsoleOutput {
    fn draw_board(&mut self, p1: &crate::Player, p2: &crate::Player) -> Result<(), Box<dyn Error>> {
        let top = 1;
        let bottom = self.h - 2;
        self.draw_status(p1.lives, p2.lives)?;

        let mut stdout = &self.writer;

        // top border
        queue!(stdout, cursor::MoveTo(0, top))?;
        line(&mut stdout, self.w)?;

        // side borders
        for i in top + 1..bottom {
            queue!(
                stdout,
                cursor::MoveTo(0, i),
                style::Print("|"),
                cursor::MoveTo(self.w - 1, i),
                style::Print("|"),
            )?;
        }

        // bottom border
        queue!(stdout, cursor::MoveTo(0, bottom))?;
        line(&mut stdout, self.w)?;

        // player start positions
        queue!(
            stdout,
            cursor::MoveTo(p1.pos.x, p1.pos.y),
            style::Print("1")
        )?;
        queue!(
            stdout,
            cursor::MoveTo(p2.pos.x, p2.pos.y),
            style::Print("2")
        )?;

        stdout.flush()?;
        Ok(())
    }

    fn draw_status(&mut self, p1_lives: usize, p2_lives: usize) -> Result<(), Box<dyn Error>> {
        let third_width = self.w / 3;
        let player1 = format!("Player 1: {} / {}", p1_lives, START_LIVES);
        let player2 = format!("Player 2: {} / {}", p2_lives, START_LIVES);
        queue!(
            self.writer,
            cursor::MoveTo(third_width - player1.len() as u16 / 2, 0),
            style::Print(player1),
            cursor::MoveTo(2 * third_width - player2.len() as u16 / 2, 0),
            style::Print(player2),
        )?;
        Ok(())
    }
}

impl crate::Output for ConsoleOutput {
    fn init(&mut self) -> Result<(), Box<dyn Error>> {
        terminal::enable_raw_mode()?;
        execute!(
            self.writer,
            terminal::Clear(terminal::ClearType::All),
            terminal::SetTitle(TITLE),
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;
        Ok(())
    }

    fn start(&mut self, p1: &crate::Player, p2: &crate::Player) -> Result<(), Box<dyn Error>> {
        queue!(self.writer, terminal::Clear(terminal::ClearType::All))?;
        self.draw_board(p1, p2)?;
        self.prev_p1 = p1.pos;
        self.prev_p2 = p2.pos;
        Ok(())
    }

    fn render(
        &mut self,
        p1: &crate::Player,
        p2: &crate::Player,
        missiles: &[crate::Missile],
    ) -> Result<(), Box<dyn Error>> {
        let (prev1, prev2) = (self.prev_p1, self.prev_p2);
        queue!(
            self.writer,
            cursor::MoveTo(prev1.x, prev1.y),
            style::Print(" "),
            cursor::MoveTo(prev2.x, prev2.y),
            style::Print(" "),
            cursor::MoveTo(p1.pos.x, p1.pos.y),
            style::Print("1"),
            cursor::MoveTo(p2.pos.x, p2.pos.y),
            style::Print("2"),
        )?;

        for m in missiles {
            queue!(
                self.writer,
                cursor::MoveTo(m.prev.x, m.prev.y),
                style::Print(" "),
            )?;
            if let Some(p) = m.pos {
                queue!(self.writer, cursor::MoveTo(p.x, p.y), style::Print("*"),)?;
            }
        }
        self.writer.flush()?;

        self.prev_p1 = p1.pos;
        self.prev_p2 = p2.pos;

        Ok(())
    }

    fn dimensions(&self) -> Result<(u16, u16), Box<dyn Error>> {
        Ok((self.w, self.h))
    }

    fn banner(&mut self, msg: &[&str]) -> Result<(), Box<dyn Error>> {
        let (w, h) = (self.w, self.h);
        queue!(self.writer, terminal::Clear(terminal::ClearType::All))?;
        let mut msg_top = h / 2 - msg.len() as u16 / 2;
        for m in msg {
            queue!(
                self.writer,
                cursor::MoveTo(w / 2 - m.len() as u16 / 2, msg_top),
                style::Print(m),
            )?;
            msg_top += 1;
        }
        self.writer.flush()?;
        Ok(())
    }

    fn print(&mut self, x: u16, y: u16, s: &str) -> Result<(), Box<dyn Error>> {
        execute!(&self.writer, cursor::MoveTo(x, y), style::Print(s))?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), Box<dyn Error>> {
        execute!(
            self.writer,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
            cursor::Show
        )?;
        terminal::disable_raw_mode()?;
        Ok(())
    }
}

fn line<T: Write>(writer: &mut T, width: u16) -> Result<(), crossterm::ErrorKind> {
    queue!(writer, style::Print("-".repeat(width as usize)))
}
