use std::ffi::CString;
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::path::Path;

use nix::fcntl::{OFlag, open};
use nix::sys::stat::Mode;
use nix::sys::wait::{WaitStatus, waitpid};
use nix::unistd::{ForkResult, dup2_stdin, dup2_stdout, execvp, fork};

enum StdoutType {
    Truncate,
    Append,
}

fn open_or_exit(filename: &str, flags: OFlag) -> OwnedFd {
    match open(filename, flags, Mode::from_bits_truncate(0o644)) {
        Ok(fd) => fd,
        Err(e) => {
            eprintln!("rsh: {filename}: {e}");
            std::process::exit(1);
        }
    }
}

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
        let word = line.trim();
        let mut parts = word.split_whitespace();
        let Some(cmd) = parts.next() else { continue };
        let mut args = vec![CString::new(cmd).unwrap()];
        let mut stdin: Option<&str> = None;
        let mut stdout: Option<(StdoutType, &str)> = None;
        while let Some(part) = parts.next() {
            match part {
                ">" | ">>" | "<" => {
                    let Some(file) = parts.next() else {
                        eprintln!("rsh: syntax error: expected filename after {part}");
                        last_status = 2;
                        continue 'repl;
                    };
                    match part {
                        ">" => stdout = Some((StdoutType::Truncate, file)),
                        ">>" => stdout = Some((StdoutType::Append, file)),
                        _ => stdin = Some(file),
                    }
                }
                "$?" => args.push(CString::new(last_status.to_string()).unwrap()),
                _ => args.push(CString::new(part).unwrap()),
            }
        }
        match cmd {
            "exit" => {
                let code = args
                    .get(1)
                    .and_then(|a| a.to_str().ok()?.parse().ok())
                    .unwrap_or(last_status);
                std::process::exit(code);
            }
            "cd" => {
                if args.len() == 1 {
                    if let Some(home_dir) = std::env::home_dir() {
                        if let Err(e) = std::env::set_current_dir(home_dir) {
                            eprintln!("rsh: cd: {e}");
                            last_status = 1;
                            continue;
                        }
                    }
                } else {
                    let path = args.get(1).unwrap().to_str().unwrap();
                    if let Err(e) = std::env::set_current_dir(Path::new(path)) {
                        eprintln!("rsh: cd: {path}: {e}");
                        last_status = 1;
                        continue;
                    }
                }
                last_status = 0;
            }
            _ => match unsafe { fork() }? {
                ForkResult::Parent { child } => {
                    if let WaitStatus::Exited(_, code) = waitpid(child, None)? {
                        last_status = code;
                    }
                }
                ForkResult::Child => {
                    match stdout {
                        Some((direction, filename)) => match direction {
                            StdoutType::Truncate => {
                                let fd = open_or_exit(
                                    filename,
                                    OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC,
                                );
                                dup2_stdout(&fd)?;
                            }
                            StdoutType::Append => {
                                let fd = open_or_exit(
                                    filename,
                                    OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_APPEND,
                                );
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
                    let Err(e) = execvp(&args[0], &args);
                    eprintln!("rsh: {cmd}: {e}");
                    std::process::exit(127);
                }
            },
        }
    }
    Ok(())
}
