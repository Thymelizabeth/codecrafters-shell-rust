use std::io::{self, Write};

fn main() -> Result<(), io::Error> {
    let mut input = String::new();
    loop {
        prompt()?;

        io::stdin().read_line(&mut input)?;
        println!("{}: command not found", input.trim());
        input.clear();
    }
    Ok(())
}

fn prompt() -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    write!(&mut stdout, "$ ")?;
    stdout.flush()
}
