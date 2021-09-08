
use std::env;
use std::os::unix::net;
use std::io::Write;
use std::thread;
use std::time;

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

    let mut sock = match net::UnixStream::connect(sock_path) {
        Ok(s) => s,
        Err(e) => anyhow::bail!("Couldn't connecto to {}. {}", sock_path, e),
    };

    let mut dir = 0;
    let mut cmd = vec![FIRE, dir, 0, 0, 0, 0, 0, 0];
    loop {
        sock.write(&cmd)?;
        thread::sleep(time::Duration::from_secs(1));
        dir += 1;
        dir %= 4;
        cmd[1] = dir;
    }

    //Ok(())
}
