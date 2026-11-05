use std::ffi::CString;

pub enum StdoutType {
    Truncate,
    Append,
}

pub struct Command<'a> {
    pub cmd: &'a str,
    pub args: Vec<CString>,
    pub stdin: Option<&'a str>,
    pub stdout: Option<(StdoutType, &'a str)>,
}

pub fn parse_command(input: &str, last_status: i32) -> Result<Option<Command<'_>>, String> {
    let mut parts = input.split_whitespace();
    let Some(cmd) = parts.next() else { return Ok(None)};
    let mut args = vec![CString::new(cmd).unwrap()];
    let mut stdin: Option<&str> = None;
    let mut stdout: Option<(StdoutType, &str)> = None;
    while let Some(part) = parts.next() {
        match part {
            ">" | ">>" | "<" => {
                let Some(file) = parts.next() else {
                    return Err(format!("rsh: syntax error: expected filename after {part}"));
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
    Ok(Some(Command {
        cmd,
        args,
        stdin,
        stdout,
    }))
}
