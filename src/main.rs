use std::io::{self, Write};

fn main() -> Result<(), io::Error> {
    prompt()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    println!("{}: command not found", input.trim());
    Ok(())
}

fn prompt() -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    write!(&mut stdout, "$ ")?;
    stdout.flush();
}
