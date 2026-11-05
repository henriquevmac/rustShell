mod parser;
mod executor;

use parser::{parse_command, Command};

use std::io::{self, Write};

use crate::executor::{execute_command, execute_pipeline};

fn main() -> io::Result<()> {
    let mut line = String::new();
    let mut last_status = 0;

    'repl: loop {
        print!("rsh> ");
        io::stdout().flush()?;
        line.clear();
        let n = io::stdin().read_line(&mut line)?;
        if n == 0 {
            println!();
            break;
        }
        let commands_txt = line.trim().split('|');
        let mut commands: Vec<Command> = Vec::new();
        for command_txt in commands_txt {
            let command = match parse_command(command_txt, last_status) {
                Ok(Some(command)) => command,
                Ok(None) => {
                    if line.trim().is_empty() {continue 'repl; }
                    eprintln!("rsh: syntax error near |");
                    last_status = 2;
                    continue 'repl;
                },
                Err(e) => {
                    eprintln!("{e}");
                    last_status = 2;
                    continue 'repl;
                }
            };
            commands.push(command);
        }
        if commands.len() == 1 {
            let command = commands.pop().unwrap();
            last_status = execute_command(command, last_status)?;
        } else if commands.len() > 1 {
            last_status = execute_pipeline(commands)?;
        }
    }
    Ok(())
}
