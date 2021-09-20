A two-player scriptable console game in Rust. Only tested on Fedora Linux, but should in theory work on most text displays.

The two players use the same keyboard on a single machine.

Just `cargo run` and follow the on-screen instructions.

The game has a server which a bot can use to play, instead of a human player.
A basic bot is in progress: `cargo run -p bot -- 1` (or `-- 2` at the end for player 2).
