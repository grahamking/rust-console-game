use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug)]
pub struct Pos {
    pub x: u16,
    pub y: u16,
}
impl Pos {
    // A new position moved one unit in given direction
    pub fn moved(&self, dir: crate::Dir) -> Pos {
        let (mut x, mut y) = (self.x, self.y);
        match dir {
            crate::Dir::Up => {
                if self.y > 0 {
                    y -= 1
                }
            }
            crate::Dir::Down => y += 1,
            crate::Dir::Left => {
                if self.x > 0 {
                    x -= 1
                }
            }
            crate::Dir::Right => x += 1,
            crate::Dir::None => (),
        }
        Pos { x, y }
    }
    pub fn does_hit(&self, pos: Pos) -> bool {
        self.x == pos.x && self.y == pos.y
    }
}
impl Display for Pos {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{},{}", self.x, self.y)
    }
}
