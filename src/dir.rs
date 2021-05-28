use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Dir {
    None,
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub fn opposite(&self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
            Dir::None => Dir::None,
        }
    }
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
