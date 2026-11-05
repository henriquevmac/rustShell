use std::{iter::Peekable, str::Chars};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Pipe,
    RedirOut,
    RedirAppend,
    RedirIn,
}

enum State {
    Normal,
    SingleQuote,
    DoubleQuote,
}

fn flush_word(word: &mut String, tokens: &mut Vec<Token>) {
    if !word.is_empty() {
        tokens.push(Token::Word(std::mem::take(word)));
    }
}

fn expand(chars: &mut Peekable<Chars>, last_status: i32) -> String {
    if chars.next_if_eq(&'?').is_some() {
        return last_status.to_string();
    }
    let mut name = String::new();
    while let Some(c) = chars.next_if(|c| c.is_ascii_alphanumeric() || *c == '_') {
        name.push(c);
    }
    if name.is_empty() {
        return "$".into();
    }
    std::env::var(&name).unwrap_or_default()
}

pub fn tokenize(line: &str, last_status: i32) -> Result<Vec<Token>, String> {
    let mut state = State::Normal;
    let mut tokens: Vec<Token> = Vec::new();
    let mut word = String::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match state {
            State::Normal => match ch {
                '\'' => state = State::SingleQuote,
                '"' => state = State::DoubleQuote,
                '\\' => {
                    if let Some(next) = chars.next() {
                        word.push(next);
                    }
                }
                '>' => {
                    flush_word(&mut word, &mut tokens);
                    if chars.next_if_eq(&'>').is_some() {
                        tokens.push(Token::RedirAppend);
                    } else {
                        tokens.push(Token::RedirOut);
                    }
                }
                '<' => {
                    flush_word(&mut word, &mut tokens);
                    tokens.push(Token::RedirIn);
                }
                '|' => {
                    flush_word(&mut word, &mut tokens);
                    tokens.push(Token::Pipe);
                }
                '$' => word.push_str(&expand(&mut chars, last_status)),
                _ => {
                    if ch.is_whitespace() {
                        flush_word(&mut word, &mut tokens);
                    } else {
                        word.push(ch)
                    }
                }
            },
            State::SingleQuote => match ch {
                '\'' => state = State::Normal,
                _ => word.push(ch),
            },
            State::DoubleQuote => match ch {
                '"' => state = State::Normal,
                '\\' => match chars.peek() {
                    Some('"') | Some('\\') | Some('$') => {
                        let symbol = chars.next().unwrap();
                        word.push(symbol);
                    }
                    Some(_) | None => word.push(ch),
                },
                '$' => word.push_str(&expand(&mut chars, last_status)),
                _ => word.push(ch),
            },
        }
    }
    if !matches!(state, State::Normal) {
        return Err("rsh: syntax error: unterminated quote".into());
    }
    flush_word(&mut word, &mut tokens);
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn w(s: &str) -> Token {
        Token::Word(s.into())
    }

    #[test]
    fn operators_without_spaces() {
        assert_eq!(
            tokenize("ls>out.txt|wc", 0).unwrap(),
            vec![w("ls"), Token::RedirOut, w("out.txt"), Token::Pipe, w("wc")]
        );
    }
    #[test]
    fn append() {
        assert_eq!(
            tokenize("a >> b", 0).unwrap(),
            vec![w("a"), Token::RedirAppend, w("b")]
        );
    }
    #[test]
    fn quotes_join_into_one_word() {
        assert_eq!(
            tokenize(r#"echo "a b"'c d'e"#, 0).unwrap(),
            vec![w("echo"), w("a bc de")]
        );
    }
    #[test]
    fn backslash_kept_in_double_quotes() {
        assert_eq!(tokenize(r#""a\n""#, 0).unwrap(), vec![w(r"a\n")]);
    }
    #[test]
    fn unterminated_quote_is_error() {
        assert!(tokenize("echo 'hi", 0).is_err());
    }
    #[test]
    fn status_expands() {
        assert_eq!(
            tokenize("echo $? \"x$?y\" '$?'", 3).unwrap(),
            vec![w("echo"), w("3"), w("x3y"), w("$?")]
        );
    }
    #[test]
    fn env_var_stops_at_slash() {
        unsafe {
            std::env::set_var("RSH_TEST", "/tmp");
        }
        assert_eq!(
            tokenize("ls $RSH_TEST/x", 0).unwrap(),
            vec![w("ls"), w("/tmp/x")]
        );
    }
    #[test]
    fn lone_dollar_is_literal() {
        assert_eq!(
            tokenize("echo $ \"a$\"", 0).unwrap(),
            vec![w("echo"), w("$"), w("a$")]
        );
    }
}
