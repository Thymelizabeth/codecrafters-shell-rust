use std::io::{self, Write};

enum Command<'a> {
    Builtin(Builtin<'a>),
    Unknown(&'a str),
}

enum Builtin<'a> {
    Echo(&'a str),
    Exit,
    Type(&'a str),
}

fn main() -> Result<(), io::Error> {
    let mut input = String::new();
    loop {
        prompt()?;

        io::stdin().read_line(&mut input)?;
        match Command::from(input.as_str()) {
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

impl<'a> From<&'a str> for Command<'a> {
    fn from(input: &'a str) -> Self {
        let mut split_input = input.splitn(2, " ");
        let cmd = split_input.next().map(str::trim);
        let args = split_input.next().unwrap_or("");
        match cmd {
            Some("exit") => Command::Builtin(Builtin::Exit),
            Some("echo") => Command::Builtin(Builtin::Echo(args)),
            Some("type") => Command::Builtin(Builtin::Type(args)),
            _ => Command::Unknown(input),
        }
    }
}

impl<'a> Builtin<'a> {
    fn eval(self) -> Result<(), io::Error> {
        Ok(match self {
            Builtin::Exit => {}
            Builtin::Echo(args) => writeln!(io::stdout(), "{}", args.trim())?,
            Builtin::Type(arg) => match Command::from(arg) {
                Command::Builtin(_) => writeln!(io::stdout(), "{} is a shell builtin", arg.trim())?,
                Command::Unknown(_) => writeln!(io::stdout(), "{}: not found", arg.trim())?,
            },
        })
    }
}
