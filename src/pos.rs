use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug)]
pub struct Pos {
    pub x: u32,
    pub y: u32,
    pub invalid: bool,
}
impl Pos {
    // A new position moved amount units in given direction
    pub fn moved(&self, amount: u32, dir: crate::Dir) -> Pos {
        let (mut x, mut y) = (self.x, self.y);
        let mut invalid = false;
        match dir {
            crate::Dir::Up => {
                if self.y > amount {
                    y -= amount;
                } else {
                    invalid = true;
                }
            }
            crate::Dir::Down => y += amount,
            crate::Dir::Left => {
                if self.x > amount {
                    x -= amount;
                } else {
                    invalid = true;
                }
            }
            crate::Dir::Right => x += amount,
            crate::Dir::None => (),
        }
        Pos { x, y, invalid }
    }
    pub fn does_hit(&self, pos: Pos) -> bool {
        self.x == pos.x && self.y == pos.y
    }
    pub fn nil() -> Pos {
        Pos {
            x: 0,
            y: 0,
            invalid: true,
        }
    }
}
impl Display for Pos {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if self.invalid {
            write!(f, "INVALID {},{}", self.x, self.y)
        } else {
            write!(f, "{},{}", self.x, self.y)
        }
    }
}
