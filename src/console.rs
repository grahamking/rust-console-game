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
    fn draw_board(
        &mut self,
        p1_lives: usize,
        p1_energy: u32,
        p2_lives: usize,
        p2_energy: u32,
    ) -> Result<(), Box<dyn Error>> {
        let top = 1;
        let bottom = self.h - 2;
        self.draw_status(p1_lives, p1_energy, p2_lives, p2_energy)?;

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

    fn draw_status(
        &mut self,
        p1_lives: usize,
        p1_energy: u32,
        p2_lives: usize,
        p2_energy: u32,
    ) -> Result<(), Box<dyn Error>> {
        let third_width = self.w / 3;
        let player1 = format!(
            "Player 1: {} / {}. Energy: {}",
            p1_lives,
            crate::PLAYER_LIVES,
            p1_energy
        );
        let player2 = format!(
            "Player 2: {} / {}. Energy: {}",
            p2_lives,
            crate::PLAYER_LIVES,
            p2_energy
        );
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
        self.draw_board(
            w.p1_lives as usize,
            w.energy[w.player1],
            w.p2_lives as usize,
            w.energy[w.player2],
        )?;

        for entity_id in crate::alive_entities(w) {
            let sprite = &w.sprite[entity_id];
            let (_, dir) = w.velocity[entity_id];
            let tx = if dir.is_vertical() {
                &sprite.texture_vertical[0]
            } else {
                &sprite.texture_horizontal[0]
            };
            if sprite.is_bold {
                queue!(self.writer, style::SetAttribute(style::Attribute::Bold))?;
            }
            for pos in w.position[entity_id].iter() {
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

        /*

        // draw missiles
        for m in missiles {
            queue!(self.writer, style::SetForegroundColor(COLORS[m.color_idx]))?;
            if m.is_exploding() {
                for ep in m.explosion() {
                    queue!(
                        self.writer,
                        cursor::MoveTo(ep.x, ep.y),
                        style::Print(&m.texture_horizontal)
                    )?;
                }
            } else if m.is_alive {
                let tx = if m.dir.is_vertical() {
                    &m.texture_vertical
                } else {
                    &m.texture_horizontal
                };
                let opposite = m.dir.opposite();
                let mut draw_pos = m.pos;
                for _ in 0..m.speed {
                    if crate::is_on_board(draw_pos.x, draw_pos.y, self.w, self.h) {
                        queue!(
                            self.writer,
                            cursor::MoveTo(draw_pos.x, draw_pos.y),
                            style::Print(tx),
                        )?;
                    }
                    draw_pos = draw_pos.moved(opposite);
                }
            }
            queue!(self.writer, style::ResetColor)?;
        }
        */
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
