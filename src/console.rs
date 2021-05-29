use crossterm::{cursor, execute, queue, style, terminal};
use std::error::Error;
use std::io::{stdout, Stdout, Write};

const START_LIVES: usize = 5;
const TITLE: &str = "Hash Bang";

lazy_static! {
    static ref COLORS: Vec<style::Color> =
        vec![style::Color::Grey, style::Color::Yellow, style::Color::Cyan,];
}

pub struct ConsoleOutput {
    w: u16,
    h: u16,
    prev_p1: crate::Pos,
    prev_p2: crate::Pos,
    writer: Stdout,
    to_clear: Vec<crate::Pos>,
}

pub fn new() -> ConsoleOutput {
    let (w, h) = terminal::size().unwrap();
    ConsoleOutput {
        w,
        h,
        prev_p1: crate::Pos { x: 10, y: 10 }, // the 10/10 are never used
        prev_p2: crate::Pos { x: 10, y: 10 },
        writer: stdout(),
        to_clear: Vec::new(),
    }
}

impl ConsoleOutput {
    fn draw_board(&mut self, p1_lives: usize, p2_lives: usize) -> Result<(), Box<dyn Error>> {
        let top = 1;
        let bottom = self.h - 2;
        self.draw_status(p1_lives, p2_lives)?;

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
            style::SetForegroundColor(COLORS[1]),
            style::Print(player1),
            cursor::MoveTo(2 * third_width - player2.len() as u16 / 2, 0),
            style::SetForegroundColor(COLORS[2]),
            style::Print(player2),
            style::ResetColor,
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

    fn start(&mut self, p1: &crate::Entity, p2: &crate::Entity) -> Result<(), Box<dyn Error>> {
        queue!(self.writer, terminal::Clear(terminal::ClearType::All))?;
        self.draw_board(*p1.lives.as_ref().unwrap(), *p2.lives.as_ref().unwrap())?;
        self.prev_p1 = p1.pos;
        self.prev_p2 = p2.pos;
        Ok(())
    }

    fn render(
        &mut self,
        p1: &crate::Entity,
        p2: &crate::Entity,
        missiles: &[crate::Entity],
    ) -> Result<(), Box<dyn Error>> {
        let (prev1, prev2) = (self.prev_p1, self.prev_p2);

        // clear previous things
        queue!(
            self.writer,
            cursor::MoveTo(prev1.x, prev1.y),
            style::Print(" "),
            cursor::MoveTo(prev2.x, prev2.y),
            style::Print(" "),
        )?;
        for p in self.to_clear.iter() {
            queue!(self.writer, cursor::MoveTo(p.x, p.y), style::Print(" "),)?;
        }
        self.to_clear.clear();

        // draw players
        queue!(
            self.writer,
            cursor::MoveTo(p1.pos.x, p1.pos.y),
            style::SetAttribute(style::Attribute::Bold),
            style::SetForegroundColor(COLORS[1]),
            style::Print("1"),
            cursor::MoveTo(p2.pos.x, p2.pos.y),
            style::SetForegroundColor(COLORS[2]),
            style::Print("2"),
            style::SetAttribute(style::Attribute::Reset),
            style::ResetColor,
        )?;

        // draw missiles
        for m in missiles {
            queue!(
                self.writer,
                cursor::MoveTo(m.prev.x, m.prev.y),
                style::Print(" "),
            )?;
            queue!(self.writer, style::SetForegroundColor(COLORS[m.color_idx]))?;
            if m.is_exploding() {
                for ep in m.explosion() {
                    queue!(self.writer, cursor::MoveTo(ep.x, ep.y), style::Print("#"))?;
                    self.to_clear.push(ep);
                }
            } else if m.is_alive {
                queue!(
                    self.writer,
                    cursor::MoveTo(m.pos.x, m.pos.y),
                    style::Print("*")
                )?;
            }
            queue!(self.writer, style::ResetColor)?;
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
