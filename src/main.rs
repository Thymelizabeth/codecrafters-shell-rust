use std::io::{self, Write};

enum Command<'a> {
    Builtin(Builtin<'a>),
    Unknown(&'a str),
}

enum Builtin<'a> {
    Echo(&'a str),
    Exit,
}

fn main() -> Result<(), io::Error> {
    let mut input = String::new();
    loop {
        prompt()?;

        io::stdin().read_line(&mut input)?;
        match Command::parse_command(&input) {
            Command::Builtin(Builtin::Exit) => break,
            Command::Builtin(cmd) => cmd.eval()?,
            Command::Unknown(cmd) => writeln!(io::stdout(), "{}: command not found", cmd.trim())?,
        }
        input.clear();
    }
    Ok(())
}

fn prompt() -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    write!(&mut stdout, "$ ")?;
    stdout.flush()
}

impl<'a> Command<'a> {
    fn parse_command(input: &'a str) -> Self {
        let mut split_input = input.trim().splitn(2, " ");
        match split_input.next() {
            Some("exit") => Command::Builtin(Builtin::Exit),
            Some("echo") => Command::Builtin(Builtin::Echo(split_input.next().unwrap_or(""))),
            _ => Command::Unknown(input),
        }
    }
}

impl<'a> Builtin<'a> {
    fn eval(self) -> Result<(), io::Error> {
        Ok(match self {
            Builtin::Exit => {}
            Builtin::Echo(args) => writeln!(io::stdout(), "{}", args)?,
        })
    }
}
