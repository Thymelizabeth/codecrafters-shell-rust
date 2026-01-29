use std::{
    env,
    fs::{self, DirEntry},
    io::{self, Write},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::Path,
    process,
};

enum Command<'a> {
    Builtin(Builtin<'a>),
    Executable(DirEntry, &'a str),
    Exit,
    Unknown(&'a str),
}

enum Builtin<'a> {
    Cd(&'a str),
    Echo(&'a str),
    Pwd,
    Type(&'a str),
}

fn main() -> Result<(), io::Error> {
    let mut input = String::new();
    loop {
        prompt()?;
        io::stdin().read_line(&mut input)?;
        match Command::from(input.as_str()) {
            Command::Exit => break,
            Command::Builtin(cmd) => cmd.eval()?,
            Command::Executable(cmd, args) => {
                process::Command::new(cmd.path())
                    .arg0(cmd.file_name())
                    .args(args.trim().split(' ').filter(|arg| !arg.is_empty()))
                    .spawn()?
                    .wait()?;
            }
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
            Some("exit") => Command::Exit,
            Some("echo") => Command::Builtin(Builtin::Echo(args)),
            Some("type") => Command::Builtin(Builtin::Type(args)),
            Some("pwd") => Command::Builtin(Builtin::Pwd),
            Some("cd") => Command::Builtin(Builtin::Cd(args)),
            Some(cmd) => {
                let path = env::var_os("PATH").unwrap_or_default();
                let path = env::split_paths(&path);
                for dir in path {
                    if let Ok(mut dir_contents) = fs::read_dir(dir)
                        && let Some(cmd) = dir_contents.find_map(|file| {
                            let file = file.ok()?;
                            file.file_name()
                                .to_str()
                                .is_some_and(|name| name == cmd)
                                .then_some(file)
                        })
                        && is_executable(&cmd.path())
                    {
                        return Command::Executable(cmd, args);
                    }
                }
                Command::Unknown(input)
            }
            None => Command::Unknown(input),
        }
    }
}

impl<'a> Builtin<'a> {
    fn eval(self) -> Result<(), io::Error> {
        match self {
            Builtin::Cd(arg) => {
                let home;
                let path = Path::new(match arg.trim() {
                    "~" => {
                        home = env::var("HOME").map_err(|_| io::ErrorKind::NotFound)?;
                        home.as_str()
                    }
                    arg => arg,
                });
                match env::set_current_dir(path) {
                    Ok(()) => (),
                    Err(_) => writeln!(
                        io::stdout(),
                        "cd: {}: No such file or directory",
                        path.display()
                    )?,
                }
            }
            Builtin::Echo(args) => writeln!(io::stdout(), "{}", args.trim())?,
            Builtin::Pwd => writeln!(io::stdout(), "{}", env::current_dir()?.display())?,
            Builtin::Type(arg) => match Command::from(arg) {
                Command::Builtin(_) | Command::Exit => {
                    writeln!(io::stdout(), "{} is a shell builtin", arg.trim())?
                }
                Command::Executable(cmd, _) => writeln!(
                    io::stdout(),
                    "{} is {}",
                    cmd.file_name().display(),
                    cmd.path().display()
                )?,
                Command::Unknown(_) => writeln!(io::stdout(), "{}: not found", arg.trim())?,
            },
        }
        Ok(())
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    let metadata = match path.metadata() {
        Ok(metadata) => metadata,
        Err(_) => return false,
    };
    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}
