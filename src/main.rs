use std::{
    env,
    fs::{self, DirEntry, File},
    io::{self, Write},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::Path,
    process::{self, Stdio},
};

type OutputRedir<'a> = Option<&'a Path>;

enum Command<'a> {
    Builtin {
        command: Builtin<'a>,
        stdout: OutputRedir<'a>,
    },
    Executable {
        command: Executable<'a>,
        stdout: OutputRedir<'a>,
    },
    Exit,
    Unknown(&'a str),
}

enum Builtin<'a> {
    Cd(&'a str),
    Echo(&'a str),
    Pwd,
    Type(&'a str),
}

struct Executable<'a>(DirEntry, &'a str);

trait Eval<'a> {
    fn eval(self, stdout: OutputRedir<'a>) -> Result<(), io::Error>;
}

fn main() -> Result<(), io::Error> {
    let mut input = String::new();
    loop {
        prompt()?;
        io::stdin().read_line(&mut input)?;
        match Command::from(input.as_str()) {
            Command::Exit => break,
            Command::Builtin { command, stdout } => command.eval(stdout)?,
            Command::Executable { command, stdout } => command.eval(stdout)?,
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
        let (cmd, args) = match input.split_once(" ") {
            Some((cmd, args)) => (cmd.trim(), args),
            None => (input, ""),
        };
        let (args, stdout) = match args.split_once("1>") {
            Some((args, stdout)) => (args, Some(Path::new(stdout))),
            None => match args.split_once(">") {
                Some((args, stdout)) => (args, Some(Path::new(stdout))),
                None => (args, None),
            },
        };
        match cmd {
            "exit" => Command::Exit,
            "echo" => Command::Builtin {
                command: Builtin::Echo(args),
                stdout,
            },
            "type" => Command::Builtin {
                command: Builtin::Type(args),
                stdout,
            },
            "pwd" => Command::Builtin {
                command: Builtin::Pwd,
                stdout,
            },
            "cd" => Command::Builtin {
                command: Builtin::Cd(args),
                stdout,
            },
            cmd => {
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
                        return Command::Executable {
                            command: Executable(cmd, args),
                            stdout,
                        };
                    }
                }
                Command::Unknown(input)
            }
        }
    }
}

impl<'a> Builtin<'a> {
    #[inline]
    fn type_(arg: &str) -> String {
        match Command::from(arg) {
            Command::Builtin { .. } | Command::Exit => {
                format!("{} is a shell builtin", arg.trim())
            }
            Command::Executable {
                command: Executable(cmd, _),
                ..
            } => {
                format!("{} is {}", cmd.file_name().display(), cmd.path().display())
            }
            Command::Unknown(arg) => format!("{}: not found", arg.trim()),
        }
    }
}

impl<'a> Eval<'a> for Builtin<'a> {
    fn eval(self, stdout: OutputRedir<'a>) -> Result<(), io::Error> {
        let mut output_stdout;
        let mut output_file;
        let stdout: &mut dyn Write = match stdout {
            Some(path) => {
                fs::create_dir_all(path)?;
                output_file = File::create(path)?;
                &mut output_file
            }
            None => {
                output_stdout = io::stdout();
                &mut output_stdout
            }
        };
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
                    Err(_) => {
                        writeln!(stdout, "cd: {}: No such file or directory", path.display())?
                    }
                }
            }
            Builtin::Echo(args) => writeln!(stdout, "{}", args.trim())?,
            Builtin::Pwd => writeln!(stdout, "{}", env::current_dir()?.display())?,
            Builtin::Type(arg) => writeln!(stdout, "{}", Builtin::type_(arg))?,
        }
        Ok(())
    }
}

impl<'a> Eval<'a> for Executable<'a> {
    fn eval(self, stdout: OutputRedir<'a>) -> Result<(), io::Error> {
        let Executable(cmd, args) = self;
        let output_stdout;
        let stdout: Stdio = match stdout {
            Some(path) => {
                fs::create_dir_all(path)?;
                Stdio::from(File::create(path)?)
            }
            None => {
                output_stdout = io::stdout();
                Stdio::from(output_stdout)
            }
        };
        process::Command::new(cmd.path())
            .arg0(cmd.file_name())
            .args(args.trim().split(' ').filter(|arg| !arg.is_empty()))
            .stdout(stdout)
            .spawn()?
            .wait()?;
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
