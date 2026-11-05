use crate::parser::{Command, StdoutType};

use std::ffi::CString;
use std::io;
use std::os::fd::OwnedFd;
use std::path::Path;

use nix::fcntl::{OFlag, open};
use nix::sys::signal::{SigHandler, Signal, signal};
use nix::sys::stat::Mode;
use nix::sys::wait::{WaitStatus, waitpid};
use nix::unistd::{ForkResult, dup2_stdin, dup2_stdout, execvp, fork, pipe2};

fn open_or_exit(filename: &str, flags: OFlag) -> OwnedFd {
    match open(filename, flags, Mode::from_bits_truncate(0o644)) {
        Ok(fd) => fd,
        Err(e) => {
            eprintln!("rsh: {filename}: {e}");
            std::process::exit(1);
        }
    }
}

fn handle_output(stdout: Option<(StdoutType, String)>, stdin: Option<String>) -> io::Result<()> {
    match stdout {
        Some((direction, filename)) => match direction {
            StdoutType::Truncate => {
                let fd = open_or_exit(&filename, OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC);
                dup2_stdout(&fd)?;
            }
            StdoutType::Append => {
                let fd = open_or_exit(&filename, OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_APPEND);
                dup2_stdout(&fd)?;
            }
        },
        None => {}
    }
    match stdin {
        Some(filename) => {
            let fd = open_or_exit(&filename, OFlag::O_RDONLY);
            dup2_stdin(&fd)?;
        }
        None => {}
    }
    Ok(())
}

fn exec_child(command: Command) -> ! {
    unsafe { signal(Signal::SIGPIPE, SigHandler::SigDfl) }.ok();
    unsafe { signal(Signal::SIGINT, SigHandler::SigDfl) }.ok();
    unsafe { signal(Signal::SIGQUIT, SigHandler::SigDfl) }.ok();
    match handle_output(command.stdout, command.stdin) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("rsh: error handling redirection {e}");
            std::process::exit(1);
        }
    }
    let cargs: Vec<CString> = command.args.iter()
        .map(|a| CString::new(a.as_str()).unwrap())
        .collect();
    let Err(e) = execvp(&cargs[0], &cargs);
    eprintln!("rsh: {}: {e}", command.args[0]);
    std::process::exit(127);
}

fn status_code(status: WaitStatus) -> i32 {
    match status {
        WaitStatus::Exited(_, code) => code,
        WaitStatus::Signaled(_, sig, _) => {
            if sig == Signal::SIGINT {
                println!()
            }
            128 + sig as i32
        },
        _ => 0,
    }
}

pub fn execute_command(command: Command, last_status: i32) -> io::Result<i32> {
    match command.args[0].as_str() {
        "exit" => {
            let code = command
                .args
                .get(1)
                .and_then(|a| a.parse().ok())
                .unwrap_or(last_status);
            std::process::exit(code);
        }
        "cd" => {
            if command.args.len() == 1 {
                if let Some(home_dir) = std::env::home_dir() {
                    if let Err(e) = std::env::set_current_dir(home_dir) {
                        eprintln!("rsh: cd: {e}");
                        return Ok(1);
                    }
                }
            } else {
                let path = command.args.get(1).unwrap();
                if let Err(e) = std::env::set_current_dir(Path::new(path)) {
                    eprintln!("rsh: cd: {path}: {e}");
                    return Ok(1);
                }
            }
            return Ok(0);
        }
        _ => match unsafe { fork() }? {
            ForkResult::Parent { child } => {
                return Ok(status_code(waitpid(child, None)?))
            }
            ForkResult::Child => {
                exec_child(command);
            }
        },
    }
}

pub fn execute_pipeline(commands: Vec<Command>) -> io::Result<i32> {
    let n = commands.len();
    let mut prev_read: Option<OwnedFd> = None;
    let mut children = Vec::new();

    for (i, command) in commands.into_iter().enumerate() {
        let (next_read, write) = if i < n - 1 {
            let (r, w) = pipe2(OFlag::O_CLOEXEC)?;
            (Some(r), Some(w))
        } else {
            (None, None)
        };

        match unsafe { fork() }? {
            ForkResult::Child => {
                if prev_read.is_some() {
                    match dup2_stdin(prev_read.unwrap()) {
                        Ok(_) => {},
                        Err(_) => std::process::exit(1),
                    }
                }
                if write.is_some() {
                    match dup2_stdout(write.unwrap()) {
                        Ok(_) => {},
                        Err(_) => std::process::exit(1),
                    }
                }
                exec_child(command)
            }
            ForkResult::Parent { child } => children.push(child),
        }

        prev_read = next_read;
    }

    let mut last = None;
    for child in children {
        last = Some(waitpid(child, None)?);
    };
    Ok(last.map(status_code).unwrap_or(0))
}
