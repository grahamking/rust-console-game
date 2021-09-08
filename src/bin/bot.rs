
use std::env;
use std::os::unix::net;
use std::io::{Read, Write};
use std::thread;
use std::time;
use std::convert::TryInto;
use std::io::ErrorKind;

use anyhow;

// TODO: import from server.rs
const SOCK_NAME_1: &str = "/tmp/rust-console-game-p1.sock";
const SOCK_NAME_2: &str = "/tmp/rust-console-game-p2.sock";

const FIRE: u8 = 2;

const USAGE: &str = r#"Usage: bot 1|2
    1 to be player 1, 2 to be player 2. Defaults to player 1.
"#;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        anyhow::bail!("{}", USAGE);
    }

    let sock_path = match args[0].as_str() {
        "1" => SOCK_NAME_1,
        "2" =>  SOCK_NAME_2,
        _ =>  SOCK_NAME_1,
    };

    let mut sock_out = match net::UnixStream::connect(sock_path) {
        Ok(s) => s,
        Err(e) => anyhow::bail!("Couldn't connecto to {}. {}", sock_path, e),
    };
    let mut sock_in = sock_out.try_clone()?;

    let writer = thread::spawn(move || {
        let mut cmd = vec![FIRE, 99, 0, 0, 0, 0, 0, 0];
        loop {
            for dir in [1,2,3,4] {
                cmd[1] = dir;
                if let Err(e) = sock_out.write(&cmd) {
                    println!("bot sock write err: {}", e);
                    return;
                }
                thread::sleep(time::Duration::from_secs(1));
            }
        }
    });

    let reader = thread::spawn(move || {
        let mut buf = [0u8; 12]; // protocol is units of 12 bytes
        loop {
            if let Err(e) = sock_in.read_exact(&mut buf) {
                match e.kind() {
                    ErrorKind::UnexpectedEof => { // remote closed connection
                        return;
                    },
                    _ => {
                        println!("bot read_exact: {}", e);
                    },
                }
            }
            let es = EntityState::from_network(&buf);
            println!("Read: {:?}", es);
        }
    });

    writer.join().unwrap();
    reader.join().unwrap();
    Ok(())
}

#[derive(Debug)]
struct EntityState {
    id: u8,
    x: u32,
    y: u32,
    dir: u8, // todo should be dir::Dir
    velocity: u8,
    has_shield: bool,
}
impl EntityState {
    fn from_network(msg: &[u8; 12]) -> EntityState {
        //println!("GOT: {:?}", msg);
        let mut e = EntityState{
            id: msg[0],
            dir: msg[9],
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
}
