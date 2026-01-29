use std::io::{self, Write};

fn main() -> Result<(), Box<dyn io::Error>> {
    print!("$ ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    println!("{}: command not found", input.trim());
    Ok(())
}
