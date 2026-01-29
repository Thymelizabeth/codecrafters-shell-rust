use std::io::{self, Write};

enum Command<'a> {
    Unknown(&'a str),
}

fn main() -> Result<(), io::Error> {
    let mut input = String::new();
    loop {
        prompt()?;

        io::stdin().read_line(&mut input)?;
        match Command::parse_command(&input) {
            Command::Unknown(cmd) => println!("{}: command not found", cmd.trim()),
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
            _ => Command::Unknown(input),
        }
    }
}
