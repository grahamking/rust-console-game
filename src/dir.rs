use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Dir {
    None,
    Up,
    Down,
    Left,
    Right,
}

const DIRS: [Dir; 5] = [Dir::None, Dir::Up, Dir::Down, Dir::Left, Dir::Right];

impl Dir {
    pub fn from_num(n: u8) -> Dir {
        DIRS[n as usize]
    }

    pub fn opposite(&self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
            Dir::None => Dir::None,
        }
    }
    pub fn is_vertical(&self) -> bool {
        *self == Dir::Up || *self == Dir::Down
    }
    pub fn _is_horizontal(&self) -> bool {
        *self == Dir::Left || *self == Dir::Right
    }

    pub fn as_num(&self) -> u8 {
        match self {
            Dir::None => 0,
            Dir::Up => 1,
            Dir::Down => 2,
            Dir::Left => 3,
            Dir::Right => 4,
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
