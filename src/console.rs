use crossterm::{cursor, execute, queue, style, terminal};
//use log::debug;
use std::error::Error;
use std::io::{stdout, Stdout, Write};

const TITLE: &str = "Hash Bang";

lazy_static! {
    static ref COLORS: Vec<style::Color> =
        vec![style::Color::Grey, style::Color::Yellow, style::Color::Cyan,];
}

pub struct ConsoleOutput {
    w: u16,
    h: u16,
    writer: Stdout,
}

pub fn new() -> ConsoleOutput {
    let (w, h) = terminal::size().unwrap();
    ConsoleOutput {
        w,
        h,
        writer: stdout(),
    }
}

impl ConsoleOutput {
    fn draw_board(&mut self, world: &crate::World) -> Result<(), Box<dyn Error>> {
        let top = 1;
        let bottom = self.h - 2;
        self.draw_status(world)?;

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

    fn draw_status(&mut self, world: &crate::World) -> Result<(), Box<dyn Error>> {
        let third_width = self.w / 3;
        let mut player1 = format!(
            "Player 1: {} / {}. Energy: {}",
            world.p1_lives,
            crate::PLAYER_LIVES,
            world.energy[world.player1],
        );
        if world.shield[world.player1] {
            player1 += ". SHIELD ON.";
        }
        let mut player2 = format!(
            "Player 2: {} / {}. Energy: {}",
            world.p2_lives,
            crate::PLAYER_LIVES,
            world.energy[world.player2],
        );
        if world.shield[world.player2] {
            player2 += ". SHIELD ON.";
        }
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

    fn render(&mut self, w: &mut crate::World) -> Result<(), Box<dyn Error>> {
        queue!(self.writer, terminal::Clear(terminal::ClearType::All))?;
        self.draw_board(w)?;

        for id in crate::alive_entities(w) {
            let sprite = &w.sprite[id];
            let (_, dir) = w.velocity[id];
            let is_player = id == w.player1 || id == w.player2;
            let tx: &str = if is_player && w.shield[id] {
                "@"
            } else if w.explode[id].1 {
                sprite.texture_explosion[0].as_ref().unwrap()
            } else if dir.is_vertical() {
                &sprite.texture_vertical[0]
            } else {
                &sprite.texture_horizontal[0]
            };
            if sprite.is_bold {
                queue!(self.writer, style::SetAttribute(style::Attribute::Bold))?;
            }
            for pos in w.position[id].iter() {
                queue!(
                    self.writer,
                    cursor::MoveTo(pos.x as u16, pos.y as u16),
                    style::SetForegroundColor(COLORS[sprite.color_idx]),
                    style::Print(tx),
                )?;
            }
            queue!(
                self.writer,
                style::SetAttribute(style::Attribute::Reset),
                style::ResetColor,
            )?;
        }
        self.writer.flush()?;
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
