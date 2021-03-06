use std::thread;
use std::path;
use std::fs;
use std::os::unix::net;
use std::net::Shutdown;
use std::error;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::sync::{self, Arc, Mutex};

use log::{debug, error};

use crate::InputEvent;
use rs_sdk::{Dir, SOCK_NAME_1, SOCK_NAME_2};

pub struct Server {
    entity_id: u8,
    conn: Mutex<Option<net::UnixStream>>,
}

impl Server {

    // Start a server for given player (1 or 2)
    pub fn new(player: u8, ch: sync::mpsc::Sender<InputEvent>) -> Arc<Server> {
        let sock_name = match player {
            1 => SOCK_NAME_1,
            2 => SOCK_NAME_2,
            _ => panic!("invalid player number"),
        };
        let s = Arc::new(Server{
            entity_id: player,
            conn: Mutex::new(Option::None),
        });

        let inner_s = s.clone();
        let _ = thread::spawn(move || inner_s.run(sock_name, ch));

        s
    }

    // accept a connection and call handler
    fn run(&self, sock_name: &str, ch: sync::mpsc::Sender<InputEvent>) {
        let sock_path: path::PathBuf = sock_name.into();
        if sock_path.exists() {
            fs::remove_file(&sock_path).unwrap();
        }
        debug!("Player {} server listening on {}", self.entity_id, sock_name);

        let l = net::UnixListener::bind(&sock_path).expect("local socket bind error");
        loop {
            match l.accept() {
                Ok((conn, addr)) => {
                    debug!("Connection from {:?}", addr);
                    let out_conn = match conn.try_clone() {
                        Ok(c) => c,
                        Err(e) => {
                            error!("try_clone: {}", e);
                            return;
                        }
                    };
                    self.conn.lock().unwrap().replace(out_conn);

                    handler(conn, self.entity_id, ch.clone()).unwrap();
                },
                Err(e) => error!("accept on {}: {}", sock_path.display(), e),
            }
        }
    }

    // send all our connections the latest world state. called every tick
    pub fn send_state(&self, state: &[u8]) {
        let mut l = self.conn.lock().unwrap();
        if l.is_none() {
            return;
        }
        if let Err(e) = l.as_mut().unwrap().write_all(state) {
            error!("server.send_state err: {}", e);
            let c = l.take().unwrap();
            // should happen at most once
            c.shutdown(Shutdown::Both).expect("server: conn shutdown err");
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
        ch.send(iv)?;
    }
}

fn into_input_event(b: &[u8; 8], entity_id: u8) -> InputEvent {
    match b[0] {
        0 => InputEvent::Quit,
        1 => {
            InputEvent::Move {
                entity_id,
                dir: Dir::from_num(b[1]),
            }
        },
        2 => {
            InputEvent::Fire {
                entity_id,
                dir: Dir::from_num(b[1]),
            }
        },
        3 => InputEvent::ToggleShield { entity_id },
        4 => InputEvent::ChangeWeapon { entity_id },
        _ => panic!("Undefined command: {}", b[0]),
    }
}
