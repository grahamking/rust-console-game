use log::debug;

const EXPLODE_FRAMES: u16 = 4;

#[derive(Debug)]
pub struct Entity {
    pub is_alive: bool,
    pub name: Option<String>,
    pub dir: crate::Dir,
    pub pos: crate::Pos,
    pub lives: Option<usize>,
    pub prev: crate::Pos,
    pub color_idx: usize, // indexes console.rs:COLORS, or equivalent in other output
    pub texture_vertical: String, // actually just the char to print, but sounds fancy
    pub texture_horizontal: String,
    pub texture_explosion: Option<String>,
    pub speed: u16,
    pub range: Option<i16>,
    pub is_hit: bool,

    is_bounce: bool,
    is_explodable: bool,

    w: u16, // board width
    h: u16, // board height

    explode_timer: Option<u16>,
}

pub fn new_player(name: &str, pchar: String, color_idx: usize, w: u16, h: u16) -> Entity {
    Entity {
        w,
        h,
        color_idx,
        texture_vertical: pchar.clone(),
        texture_horizontal: pchar,
        name: Some(name.to_string()),
        prev: crate::Pos { x: 0, y: 0 },
        pos: crate::Pos { x: 0, y: 0 },
        dir: crate::Dir::None,
        lives: Some(crate::PLAYER_LIVES),
        is_alive: true,
        is_bounce: true,
        is_hit: false,

        speed: 1,
        is_explodable: false,
        range: None,
        explode_timer: None,
        texture_explosion: None,
    }
}

pub fn new_missile(
    start_pos: crate::Pos,
    dir: crate::Dir,
    range: i16,
    color_idx: usize,
    w: u16,
    h: u16,
) -> Entity {
    Entity {
        w,
        h,
        dir,
        color_idx,
        texture_vertical: "*".to_string(),
        texture_horizontal: "*".to_string(),
        texture_explosion: Some("#".to_string()),
        prev: start_pos,
        pos: start_pos,
        is_alive: true,
        speed: 2,
        is_bounce: false,
        range: Some(range),
        explode_timer: None,
        is_explodable: true,
        is_hit: false,

        name: None,
        lives: None,
    }
}

pub fn new_ray(
    start_pos: crate::Pos,
    dir: crate::Dir,
    color_idx: usize,
    range: u16,
    w: u16,
    h: u16,
) -> Entity {
    Entity {
        w,
        h,
        dir,
        color_idx,
        texture_vertical: "|".to_string(),
        texture_horizontal: "-".to_string(),
        prev: start_pos,
        pos: start_pos,
        is_alive: true,
        speed: range,
        is_bounce: false,
        range: Some(range as i16),
        is_hit: false,

        explode_timer: None,
        texture_explosion: None,
        is_explodable: false,
        name: None,
        lives: None,
    }
}

impl Entity {
    pub fn update(&mut self) {
        if !self.is_alive {
            return;
        }
        if self.explode_timer.is_some() {
            self.update_explosion();
        } else {
            self.update_movement();
        }
    }

    fn update_movement(&mut self) {
        let mut dist_moved = 1;
        let mut next_pos = self.pos.moved(self.dir);
        for _ in 1..self.speed {
            next_pos = next_pos.moved(self.dir);
            dist_moved += 1;
        }
        if crate::is_on_board(next_pos.x, next_pos.y, self.w, self.h) {
            self.prev = self.pos;
            self.pos = next_pos;
        } else if self.is_bounce {
            self.dir = self.dir.opposite(); // bounce back onto the board
        } else {
            debug!(
                "dead because not on board. {} not in {},{}",
                next_pos, self.w, self.h
            );

            self.is_alive = false;
        }
        if self.range.is_some() {
            *self.range.as_mut().unwrap() -= dist_moved;
        }
        if self.is_explodable && *self.range.as_ref().unwrap() <= 0 && !self.is_exploding() {
            self.explode_timer = Some(EXPLODE_FRAMES);
        }
    }

    fn update_explosion(&mut self) {
        *self.explode_timer.as_mut().unwrap() -= 1;
        if self.explode_timer.unwrap() == 0 {
            self.is_alive = false;
        }
    }

    pub fn does_hit(&self, p: &Entity) -> bool {
        if !self.is_alive {
            return false;
        }
        let mut check_pos = self.pos;
        if check_pos.does_hit(p.pos) {
            return true;
        }
        // if we moved more than one square, check the other squares
        let opposite = self.dir.opposite();
        for _ in 1..self.speed {
            check_pos = check_pos.moved(opposite);
            if check_pos.does_hit(p.pos) {
                return true;
            }
        }
        if self.is_exploding() {
            for pos in self.explosion() {
                if pos.does_hit(p.pos) {
                    return true;
                }
            }
        }
        false
    }

    pub fn hit(&mut self) {
        if self.lives.is_none() {
            return;
        }
        let l = self.lives.as_mut().unwrap();
        *l -= 1;
        if *l == 0 {
            self.is_alive = false;
        }
        self.is_hit = true;
    }

    pub fn is_exploding(&self) -> bool {
        self.explode_timer.is_some()
    }

    // The positions affected by an explosion of this missile
    pub fn explosion(&self) -> Vec<crate::Pos> {
        let mut v = Vec::new(); // todo cache it
        if !self.is_alive {
            return v;
        }
        let left = if self.pos.x >= 2 { self.pos.x - 2 } else { 0 };
        let top = if self.pos.y >= 2 { self.pos.y - 2 } else { 0 };
        for x in left..=self.pos.x + 2 {
            for y in top..=self.pos.y + 2 {
                if crate::is_on_board(x, y, self.w, self.h) {
                    v.push(crate::Pos { x, y });
                }
            }
        }
        v
    }
}
