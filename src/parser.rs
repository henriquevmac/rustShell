use crate::{parser::StdoutType::{Append, Truncate}, tokenizer::Token::{self, Pipe, RedirAppend, RedirIn, RedirOut, Word}};

pub enum StdoutType {
    Truncate,
    Append,
}

pub struct Command {
    pub args: Vec<String>,
    pub stdin: Option<String>,
    pub stdout: Option<(StdoutType, String)>,
}

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Command>, String> {
    let mut commands: Vec<Command> = Vec::new();
    let mut iterator = tokens.into_iter();
    let mut args: Vec<String> = Vec::new();
    let mut stdin: Option<String> = None;
    let mut stdout: Option<(StdoutType, String)> = None;
    while let Some(token) = iterator.next() {
        match token {
            Word(word) => args.push(word),
            Pipe => {
                if args.is_empty() { return Err("rsh: syntax error near |".into()); }
                commands.push(Command {
                    args: std::mem::take(&mut args),
                    stdin: stdin.take(),
                    stdout: stdout.take(),
                });
            }
            RedirIn => match iterator.next() {
                Some(Word(file)) => stdin = Some(file),
                _ => return Err("rsh: syntax error near <".into())
            }
            RedirOut => match iterator.next() {
                Some(Word(file)) => stdout = Some((Truncate, file)),
                _ => return Err("rsh: syntax error near >".into())
            }
            RedirAppend => match iterator.next() {
                Some(Word(file)) => stdout = Some((Append, file)),
                _ => return Err("rsh: syntax error near >>".into())
            }
        }
    }
    if args.is_empty() {
        if !commands.is_empty() { return Err("rsh: syntax error near |".into()); }
        if stdin.is_some() { return Err("rsh: syntax error near <".into()) }
        if let Some((stdout_type, _)) = stdout {
            match stdout_type {
                Truncate => return Err("rsh: syntax error near >".into()),
                Append => return Err("rsh: syntax error near >>".into()),
            }
        }
    } else {
        commands.push(Command { args, stdin, stdout });
    }
    Ok(commands)
}
