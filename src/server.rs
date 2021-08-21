use std::thread;
use std::path;
use std::fs;
use std::os::unix::net;
use std::error;
use std::io::ErrorKind;
use std::io::Read;
use std::sync;

use log::{debug, error};

use crate::InputEvent;
use crate::dir::Dir;

pub const SOCK_NAME_1: &str = "/tmp/rust-console-game-p1.sock";
pub const SOCK_NAME_2: &str = "/tmp/rust-console-game-p2.sock";

const DIRS: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

pub fn start(ch: sync::mpsc::Sender<InputEvent>) -> (thread::JoinHandle<()>, thread::JoinHandle<()>) {
    let ch_clone = ch.clone();
    let p1 = thread::spawn(move || server_main(SOCK_NAME_1, 1, ch_clone));
    let p2 = thread::spawn(move || server_main(SOCK_NAME_2, 2, ch));
    (p1, p2)
}

fn server_main(sock_name: &str, entity_id: u8, ch: sync::mpsc::Sender<InputEvent>) {
    let sock_path: path::PathBuf = sock_name.into();
    if sock_path.exists() {
        fs::remove_file(&sock_path).unwrap();
    }
    debug!("Player {} server listening on {}", entity_id, sock_name);

    let l = net::UnixListener::bind(&sock_path).expect("local socket bind error");
    loop {
        match l.accept() {
            Ok((conn, addr)) => {
                debug!("Connection from {:?}", addr);
                handler(conn, entity_id, ch.clone()).unwrap();
            },
            Err(e) => error!("accept on {}: {}", sock_path.display(), e),
        }
    }
}

fn handler(mut conn: net::UnixStream, entity_id: u8, ch: sync::mpsc::Sender<InputEvent>) -> Result<(), Box<dyn error::Error>> {
    let mut buf = [0u8; 8]; // protocol is u64 messages
    loop {
        if let Err(e) = conn.read_exact(&mut buf) {
            match e.kind() {
                ErrorKind::UnexpectedEof => return Ok(()), // remote closed connection
                _ => {
                    error!("read_exact: {}", e);
                    return Err(Box::new(e));
                },
            }
        }
        let iv = into_input_event(&buf, entity_id);
        debug!("got: {:?}", iv);
        ch.send(iv)?;
    }
}

fn into_input_event(b: &[u8; 8], entity_id: u8) -> InputEvent {
    debug!("{:x?}", b);
    match b[0] {
        0 => InputEvent::Quit,
        1 => {
            InputEvent::Move {
                entity_id,
                dir: DIRS[b[1] as usize],
            }
        },
        2 => {
            InputEvent::Fire {
                entity_id,
                dir: DIRS[b[1] as usize],
            }
        },
        3 => InputEvent::ToggleShield { entity_id },
        4 => InputEvent::ChangeWeapon { entity_id },
        _ => panic!("Undefined command: {}", b[0]),
    }
}
