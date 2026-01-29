use std::io::{self, Write};

enum Command<'a> {
    Builtin(Builtin),
    Unknown(&'a str),
}

enum Builtin {
    Exit,
}

fn main() -> Result<(), io::Error> {
    let mut input = String::new();
    loop {
        prompt()?;

        io::stdin().read_line(&mut input)?;
        match Command::parse_command(&input) {
            Command::Builtin(Builtin::Exit) => break,
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
        match input.trim() {
            "exit" => Command::Builtin(Builtin::Exit),
            _ => Command::Unknown(input),
        }
    }
}
