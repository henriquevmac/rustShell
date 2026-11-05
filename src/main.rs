mod parser;
mod executor;

use parser::parse_command;

use std::io::{self, Write};

use crate::executor::executor;

fn main() -> io::Result<()> {
    let mut line = String::new();
    let mut last_status = 0;

    loop {
        print!("rsh> ");
        io::stdout().flush()?;
        line.clear();
        let n = io::stdin().read_line(&mut line)?;
        if n == 0 {
            println!();
            break;
        }
        let word = line.trim();
        let command = match parse_command(word, last_status) {
            Ok(Some(command)) => command,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("{e}");
                last_status = 2;
                continue;
            }
        };
        last_status = executor(command, last_status)?;
    }
    Ok(())
}
