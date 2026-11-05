mod executor;
mod parser;
mod tokenizer;

use parser::parse;

use std::io::{self, Write};

use crate::executor::{execute_command, execute_pipeline};
use crate::tokenizer::tokenize;

use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction, signal};
use std::os::fd::BorrowedFd;
use std::sync::atomic::{AtomicBool, Ordering};

static AT_PROMPT: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigint(_: nix::libc::c_int) {
    if AT_PROMPT.load(Ordering::Relaxed) {
        INTERRUPTED.store(true, Ordering::Relaxed);
        let stdout = unsafe { BorrowedFd::borrow_raw(1) }; // fd 1, no std machinery
        let _ = nix::unistd::write(stdout, b"\nrsh> ");
    }
}

fn main() -> io::Result<()> {
    let action = SigAction::new(
        SigHandler::Handler(on_sigint),
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );
    unsafe { sigaction(Signal::SIGINT, &action) }.ok();
    unsafe { signal(Signal::SIGQUIT, SigHandler::SigIgn) }.ok();
    let mut line = String::new();
    let mut last_status = 0;

    loop {
        print!("rsh> ");
        io::stdout().flush()?;
        line.clear();
        AT_PROMPT.store(true, Ordering::Relaxed);
        let n = io::stdin().read_line(&mut line)?;
        AT_PROMPT.store(false, Ordering::Relaxed);
        if INTERRUPTED.swap(false, Ordering::Relaxed) {
            last_status = 130;
        }
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
