# rsh — a small Unix shell in Rust

A learning project: a Unix shell written from scratch in Rust, built directly on the system calls (`fork`, `execvp`, `pipe2`, `dup2`, `waitpid`, `sigaction`) through the [`nix`](https://crates.io/crates/nix) crate, without `std::process::Command`.

```
rsh> printf "c\nb\na\n" | sort | head -2 > first.txt
rsh> cat < first.txt
a
b
rsh> echo "home is $HOME" '$HOME is literal'
home is /home/user $HOME is literal
rsh> foo
rsh: foo: ENOENT: No such file or directory
rsh> echo $?
127
```

## Features

- **Running programs** from `PATH` with fork/exec, and exit codes kept for `$?`
- **Builtins:** `cd` (no argument → `$HOME`) and `exit [code]`
- **Redirection:** `>`, `>>` and `<`, with or without spaces (`ls>out.txt`)
- **Pipelines** of any length (`a | b | c`), with the status of the last command
- **Quoting:** double quotes (expansion still happens), single quotes (everything literal), backslash escapes
- **Expansion:** `$?` and environment variables (`$HOME`, `$HOME/docs`)
- **Signals:**
  - Ctrl-C and Ctrl-\ stop the running command, not the shell
  - Ctrl-C at the prompt gives a fresh prompt
  - a command killed by a signal sets `$?` to 128 + the signal number (130 for Ctrl-C)
- **Syntax errors** (`ls |`, `echo hi >`, `echo "unterminated`) print a message and set `$?` to 2

## Build and run

Linux only (it uses POSIX APIs through `nix`).

```sh
cargo run            # start the shell
cargo test           # tokenizer unit tests
```

Exit with `exit` or Ctrl-D.

## How it works

Each input line goes through three stages:

```
line ──tokenize──▶ Vec<Token> ──parse──▶ Vec<Command> ──execute──▶ exit status
```

| File | Role |
|---|---|
| `src/tokenizer.rs` | A character-by-character state machine (normal / single-quoted / double-quoted). Handles quotes, escapes and `$` expansion, and produces `Word`, `Pipe`, `RedirIn`, `RedirOut` and `RedirAppend` tokens. |
| `src/parser.rs` | Groups tokens into `Command`s (arguments plus optional stdin and stdout redirects), split at each `Pipe`, and reports syntax errors. |
| `src/executor.rs` | Runs builtins in the shell process; otherwise forks, applies redirects with `dup2`, and execs. Pipelines connect children with `pipe2(O_CLOEXEC)` so no pipe end leaks into a program. |
| `src/main.rs` | The read–eval loop, `$?` tracking and the `SIGINT` handler. |

Some details worth knowing:
- The shell ignores `SIGQUIT` and handles `SIGINT`, and children reset both to the default before `exec`. An *ignored* signal would otherwise stay ignored in every program the shell starts.
- Children also reset `SIGPIPE` to the default. The Rust runtime ignores it at startup, which would make `yes | head -1` print "Broken pipe" errors.
- The `SIGINT` handler only touches atomics and calls raw `write(2)`, since those are async-signal-safe. It's installed with `SA_RESTART`, so a Ctrl-C can't interrupt `waitpid`.

## Not supported (yet)

- Job control: `&`, `jobs`, `fg`, `bg`, Ctrl-Z
- `&&`, `||`, `;`, globbing (`*.txt`), `2>`, here-docs
- Line editing and history (input uses the terminal's cooked mode)
- Word splitting of unquoted variables (every expansion behaves as if quoted)
- Empty quoted arguments (`echo ""` passes no argument)
