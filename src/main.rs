use std::{
    env,
    fs::{self, DirEntry, File},
    io::{self, Write},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::Path,
    process::{self, Stdio},
};
enum Overwrite {
    Overwrite,
    Append,
}

type OutputRedir<'a> = Option<(&'a Path, Overwrite)>;

enum Command<'a> {
    Builtin {
        command: Builtin<'a>,
        stdout: OutputRedir<'a>,
        stderr: OutputRedir<'a>,
    },
    Executable {
        command: Executable<'a>,
        stdout: OutputRedir<'a>,
        stderr: OutputRedir<'a>,
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
    fn eval(self, stdout: OutputRedir<'a>, stderr: OutputRedir<'a>) -> Result<(), io::Error>;
}

fn main() -> Result<(), io::Error> {
    let mut input = String::new();
    loop {
        prompt()?;
        io::stdin().read_line(&mut input)?;
        match Command::from(input.as_str()) {
            Command::Exit => break,
            Command::Builtin {
                command,
                stdout,
                stderr,
            } => command.eval(stdout, stderr)?,
            Command::Executable {
                command,
                stdout,
                stderr,
            } => command.eval(stdout, stderr)?,
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
        let (cmd, args) = input.split_once(" ").unwrap_or((input, ""));
        let (args, stderr) = match args.trim().split_once("2>") {
            Some((args, stderr)) => (args, Some((Path::new(stderr.trim()), Overwrite::Overwrite))),
            None => (args, None),
        };
        let (args, stdout) = if let Some((args, stdout)) = args.trim().split_once("1>>") {
            (args, Some((Path::new(stdout.trim()), Overwrite::Append)))
        } else if let Some((args, stdout)) = args.trim().split_once(">>") {
            (args, Some((Path::new(stdout.trim()), Overwrite::Append)))
        } else if let Some((args, stdout)) = args.trim().split_once("1>") {
            (args, Some((Path::new(stdout.trim()), Overwrite::Overwrite)))
        } else if let Some((args, stdout)) = args.trim().split_once(">") {
            (args, Some((Path::new(stdout.trim()), Overwrite::Overwrite)))
        } else {
            (args, None)
        };
        match cmd.trim() {
            "exit" => Command::Exit,
            "echo" => Command::Builtin {
                command: Builtin::Echo(args),
                stdout,
                stderr,
            },
            "type" => Command::Builtin {
                command: Builtin::Type(args),
                stdout,
                stderr,
            },
            "pwd" => Command::Builtin {
                command: Builtin::Pwd,
                stdout,
                stderr,
            },
            "cd" => Command::Builtin {
                command: Builtin::Cd(args),
                stdout,
                stderr,
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
                            stderr,
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
    fn type_(arg: &str) -> Result<String, String> {
        match Command::from(arg) {
            Command::Builtin { .. } | Command::Exit => {
                Ok(format!("{} is a shell builtin", arg.trim()))
            }
            Command::Executable {
                command: Executable(cmd, _),
                ..
            } => Ok(format!(
                "{} is {}",
                cmd.file_name().display(),
                cmd.path().display()
            )),
            Command::Unknown(arg) => Err(format!("{}: not found", arg.trim())),
        }
    }
}

impl<'a> Eval<'a> for Builtin<'a> {
    fn eval(self, stdout: OutputRedir<'a>, stderr: OutputRedir<'a>) -> Result<(), io::Error> {
        let mut output_stdout;
        let mut output_stderr;
        let mut output_file;
        let mut error_file;
        let stdout: &mut dyn Write = match stdout {
            Some((path, overwrite)) => {
                if let Some(dir_path) = path.parent() {
                    fs::create_dir_all(dir_path)?;
                }
                output_file = match overwrite {
                    Overwrite::Overwrite => File::create(path)?,
                    Overwrite::Append => File::options().append(true).create(true).open(path)?,
                };
                &mut output_file
            }
            None => {
                output_stdout = io::stdout();
                &mut output_stdout
            }
        };
        let stderr: &mut dyn Write = match stderr {
            Some((path, overwrite)) => {
                if let Some(dir_path) = path.parent() {
                    fs::create_dir_all(dir_path)?;
                }
                error_file = match overwrite {
                    Overwrite::Overwrite => File::create(path)?,
                    Overwrite::Append => File::options().append(true).create(true).open(path)?,
                };
                &mut error_file
            }
            None => {
                output_stderr = io::stderr();
                &mut output_stderr
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
                        writeln!(stderr, "cd: {}: No such file or directory", path.display())?
                    }
                }
            }
            Builtin::Echo(args) => writeln!(stdout, "{}", args.trim())?,
            Builtin::Pwd => writeln!(stdout, "{}", env::current_dir()?.display())?,
            Builtin::Type(arg) => match Builtin::type_(arg) {
                Ok(type_of_cmd) => writeln!(stdout, "{}", type_of_cmd)?,
                Err(type_of_cmd) => writeln!(stderr, "{}", type_of_cmd)?,
            },
        }
        Ok(())
    }
}

impl<'a> Eval<'a> for Executable<'a> {
    fn eval(self, stdout: OutputRedir<'a>, stderr: OutputRedir<'a>) -> Result<(), io::Error> {
        let Executable(cmd, args) = self;
        let stdout: Stdio = match stdout {
            Some((path, overwrite)) => {
                if let Some(dir_path) = path.parent() {
                    fs::create_dir_all(dir_path)?;
                }
                Stdio::from(match overwrite {
                    Overwrite::Overwrite => File::create(path)?,
                    Overwrite::Append => File::options().append(true).create(true).open(path)?,
                })
            }
            None => Stdio::inherit(),
        };
        let stderr: Stdio = match stderr {
            Some((path, overwrite)) => {
                if let Some(dir_path) = path.parent() {
                    fs::create_dir_all(dir_path)?;
                }
                Stdio::from(match overwrite {
                    Overwrite::Overwrite => File::create(path)?,
                    Overwrite::Append => File::options().append(true).create(true).open(path)?,
                })
            }
            None => Stdio::inherit(),
        };
        process::Command::new(cmd.path())
            .arg0(cmd.file_name())
            .args(args.trim().split(' ').filter(|arg| !arg.is_empty()))
            .stdout(stdout)
            .stderr(stderr)
            .status()?;
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
