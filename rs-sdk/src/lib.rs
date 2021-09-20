use std::os::unix::net;
use std::io::{Read, Write};
use std::convert::TryInto;
use std::io::ErrorKind;
use std::error;
use std::fmt;

mod dir;
pub use dir::Dir;

pub const SOCK_NAME_1: &str = "/tmp/rust-console-game-p1.sock";
pub const SOCK_NAME_2: &str = "/tmp/rust-console-game-p2.sock";

// must match the order in which players are added in game/src/lib.rs
const PLAYER_1_ID: u8 = 0;
const PLAYER_2_ID: u8 = 1;

// Commands
// Must match game/src/server.rs into_input_event
const MOVE: u8 = 1;
const FIRE: u8 = 2;

#[derive(Clone, Copy, Debug)]
pub enum Player {
    One,
    Two,
}
impl Player {
    fn sock_path(&self) -> &'static str {
        match self {
            Player::One => SOCK_NAME_1,
            Player::Two => SOCK_NAME_2,
        }
    }
    fn id(&self) -> u8 {
        match self {
            Player::One => PLAYER_1_ID,
            Player::Two => PLAYER_2_ID,
        }
    }
}

pub struct BotIn {
    sock_in: net::UnixStream,
    buf: [u8; 12],  // protocol is units of 12 bytes
}

pub struct BotOut {
    move_cmd: Vec<u8>,
    fire_cmd: Vec<u8>,
    sock_out: net::UnixStream,
}

pub fn connect(p: Player) -> Result<(BotIn, BotOut), anyhow::Error> {
    let sp = p.sock_path();
    let sock_out = match net::UnixStream::connect(sp) {
        Ok(s) => s,
        Err(e) => anyhow::bail!("Couldn't connecto to {}. {}", sp, e),
    };
    let sock_in = sock_out.try_clone()?;
    let b_in = BotIn {
        sock_in,
        buf: [0u8; 12],
    };
    let b_out = BotOut {
        sock_out,
        move_cmd: vec![MOVE, 99, 0, 0, 0, 0, 0, 0],
        fire_cmd: vec![FIRE, 99, 0, 0, 0, 0, 0, 0],
    };
    Ok((b_in, b_out))
}

impl BotOut {
    // Set bot direction
    pub fn dir(&mut self, d: Dir) -> Result<(), anyhow::Error> {
        self.move_cmd[1] = d.as_num();
        //self.send_cmd(&self.move_cmd)
        match self.sock_out.write(&self.move_cmd) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("socket write err: {}", e)),
        }
    }

    // Fire in a direction
    pub fn fire(&mut self, d: Dir) -> Result<(), anyhow::Error> {
        self.fire_cmd[1] = d.as_num();
        match self.sock_out.write(&self.fire_cmd) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("socket write err: {}", e)),
        }
    }

    //fn send_cmd(&mut self, cmd: &[u8]) -> Result<(), anyhow::Error> {
    //    match self.sock_out.write(cmd) {
    //        Ok(_) => Ok(()),
    //        Err(e) => Err(anyhow::anyhow!("socket write err: {}", e)),
    //    }
    //}
}

impl BotIn {
    pub fn get_next_entity(&mut self) -> Result<EntityState, SDKError> {
        if let Err(e) = self.sock_in.read_exact(&mut self.buf) {
            match e.kind() {
                ErrorKind::UnexpectedEof => { // remote closed connection
                    return Err(SDKError::Stop);
                },
                _ => {
                    return Err(SDKError::Misc(format!("bot read_exact: {}", e)));
                },
            }
        }
        Ok(EntityState::from_network(&self.buf))
    }
}

#[derive(Debug)]
pub enum SDKError {
    Stop,
    Misc(String),
}

impl error::Error for SDKError {}
impl fmt::Display for SDKError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SDKError::Stop => write!(f, "stop"),
            SDKError::Misc(msg) => write!(f, "{}", msg)
        }
    }
}

#[derive(Debug)]
pub struct EntityState {
    id: u8,
    x: u32,
    y: u32,
    dir: Dir,
    velocity: u8,
    has_shield: bool,
}
impl EntityState {
    fn from_network(msg: &[u8]) -> EntityState {
        //println!("GOT: {:?}", msg);
        let mut e = EntityState{
            id: msg[0],
            dir: Dir::from_num(msg[9]),
            velocity: msg[10],
            has_shield: msg[11] == 1,
            x: 0,
            y: 0,
        };
        // bytes 1..5 (not inclusive) are x position as u32
        let (x_bytes, rest) = msg[1..].split_at(4);
        e.x = u32::from_be_bytes(x_bytes.try_into().unwrap());
        let (y_bytes, _) = rest.split_at(4); // next 4 bytes are y position as u32
        e.y = u32::from_be_bytes(y_bytes.try_into().unwrap());

        e
    }

    pub fn is_player(&self, p: Player) -> bool {
        self.id == p.id()
    }

    pub fn pos(&self) -> (u32, u32) {
        (self.x, self.y)
    }
}
