use crate::parser::{Command, StdoutType};

use std::io;
use std::os::fd::OwnedFd;
use std::path::Path;

use nix::fcntl::{OFlag, open};
use nix::sys::stat::Mode;
use nix::sys::wait::{WaitStatus, waitpid};
use nix::unistd::{ForkResult, dup2_stdin, dup2_stdout, execvp, fork};

fn open_or_exit(filename: &str, flags: OFlag) -> OwnedFd {
    match open(filename, flags, Mode::from_bits_truncate(0o644)) {
        Ok(fd) => fd,
        Err(e) => {
            eprintln!("rsh: {filename}: {e}");
            std::process::exit(1);
        }
    }
}

fn handle_output(stdout: Option<(StdoutType, &str)>, stdin: Option<&str>) -> io::Result<()> {
    match stdout {
        Some((direction, filename)) => match direction {
            StdoutType::Truncate => {
                let fd = open_or_exit(filename, OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC);
                dup2_stdout(&fd)?;
            }
            StdoutType::Append => {
                let fd = open_or_exit(filename, OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_APPEND);
                dup2_stdout(&fd)?;
            }
        },
        None => {}
    }
    match stdin {
        Some(filename) => {
            let fd = open_or_exit(filename, OFlag::O_RDONLY);
            dup2_stdin(&fd)?;
        }
        None => {}
    }
    Ok(())
}

pub fn executor(command: Command, last_status: i32) -> io::Result<i32> {
    match command.cmd {
        "exit" => {
            let code = command
                .args
                .get(1)
                .and_then(|a| a.to_str().ok()?.parse().ok())
                .unwrap_or(last_status);
            std::process::exit(code);
        }
        "cd" => {
            if command.args.len() == 1 {
                if let Some(home_dir) = std::env::home_dir() {
                    if let Err(e) = std::env::set_current_dir(home_dir) {
                        eprintln!("rsh: cd: {e}");
                        return Ok(1)
                    }
                }
            } else {
                let path = command.args.get(1).unwrap().to_str().unwrap();
                if let Err(e) = std::env::set_current_dir(Path::new(path)) {
                    eprintln!("rsh: cd: {path}: {e}");
                    return Ok(1);
                }
            }
            return Ok(0)
        }
        _ => match unsafe { fork() }? {
            ForkResult::Parent { child } => {
                if let WaitStatus::Exited(_, code) = waitpid(child, None)? {
                    return Ok(code);
                }
            }
            ForkResult::Child => {
                handle_output(command.stdout, command.stdin)?;
                let Err(e) = execvp(&command.args[0], &command.args);
                let cmd = command.cmd;
                eprintln!("rsh: {cmd}: {e}");
                std::process::exit(127);
            }
        },
    }
    Ok(last_status)
}
