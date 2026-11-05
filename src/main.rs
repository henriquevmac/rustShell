mod executor;
mod parser;
mod tokenizer;

use parser::parse;

use std::io::{self, Write};

use crate::{
    executor::{execute_command, execute_pipeline},
    tokenizer::tokenize,
};

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
        let tokens = match tokenize(&line, last_status) {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("{e}");
                last_status = 2;
                continue;
            }
        };
        let mut commands = match parse(tokens) {
            Ok(commands) => commands,
            Err(e) => {
                eprintln!("{e}");
                last_status = 2;
                continue;
            }
        };
        match commands.len() {
            0 => continue,
            1 => {
                let command = commands.pop().unwrap();
                last_status = execute_command(command, last_status)?;
            }
            _ => last_status = execute_pipeline(commands)?,
        }
    }
    Ok(())
}
