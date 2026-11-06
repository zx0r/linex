//! Zero-alloc CLI argument parsing (< 90 lines).

use crate::fmt::Dialect;
use crate::{Widget, Widgets};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Env {
    pub no_color: bool,
    pub state_dir: Option<std::ffi::OsString>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd<'a> {
    Run(crate::Config<'a>),
    Help,
    Version,
}

pub const HELP: &[u8] = b"\
statty [WIDGET...] [OPTIONS]

WIDGETS: cpu mem ssd git all (default: all)
OPTIONS:
  -s, --style <ansi|tmux|zsh|bash|none>
      --no-color
      --separator <S>
  -h, --help
  -V, --version
";

pub const VERSION: &[u8] = concat!("statty ", env!("CARGO_PKG_VERSION"), "\n").as_bytes();

/// Parses command-line arguments and environment into a command.
///
/// # Errors
///
/// Returns an error message if an unknown flag or missing argument is encountered.
pub fn parse<'a>(argv: &'a [std::ffi::OsString], env: &'a Env) -> Result<Cmd<'a>, &'static [u8]> {
    let mut widgets = Widgets::empty();
    let mut dialect = None;
    let mut no_color = false;
    let mut sep = None;

    let mut i = 0;
    while i < argv.len() {
        let arg = argv
            .get(i)
            .map(|s| s.as_encoded_bytes())
            .unwrap_or_default();
        i += 1;
        match arg {
            b"-h" | b"--help" => return Ok(Cmd::Help),
            b"-V" | b"--version" => return Ok(Cmd::Version),
            b"--no-color" => no_color = true,
            b"-s" | b"--style" => {
                let val = argv
                    .get(i)
                    .map(|s| s.as_encoded_bytes())
                    .ok_or(&b"missing --style value"[..])?;
                i += 1;
                dialect = Some(parse_dialect(val)?);
            }
            b"--separator" => {
                let val = argv
                    .get(i)
                    .map(|s| s.as_encoded_bytes())
                    .ok_or(&b"missing --separator value"[..])?;
                i += 1;
                sep = Some(val);
            }
            _ if arg.starts_with(b"--style=") => dialect = Some(parse_dialect(&arg[8..])?),
            _ if arg.starts_with(b"--separator=") => sep = Some(&arg[12..]),
            b"cpu" => widgets.push(Widget::Cpu),
            b"mem" => widgets.push(Widget::Mem),
            b"ssd" => widgets.push(Widget::Ssd),
            b"git" => widgets.push(Widget::Git),
            b"all" => {
                widgets.push(Widget::Cpu);
                widgets.push(Widget::Mem);
                widgets.push(Widget::Ssd);
                widgets.push(Widget::Git);
            }
            _ => return Err(&b"unknown argument"[..]),
        }
    }

    if widgets.is_empty() {
        widgets = Widgets::ALL;
    }
    let d = dialect.unwrap_or(if no_color || env.no_color {
        Dialect::Plain
    } else {
        Dialect::Ansi
    });

    Ok(Cmd::Run(crate::Config {
        widgets,
        dialect: d,
        sep: sep.unwrap_or(" │ ".as_bytes()),
        state_dir: env.state_dir.as_deref(),
    }))
}

fn parse_dialect(s: &[u8]) -> Result<Dialect, &'static [u8]> {
    match s {
        b"ansi" => Ok(Dialect::Ansi),
        b"tmux" => Ok(Dialect::Tmux),
        b"zsh" => Ok(Dialect::Zsh),
        b"bash" => Ok(Dialect::Bash),
        b"none" => Ok(Dialect::Plain),
        _ => Err(&b"invalid style"[..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn parse_defaults() {
        let env = Env {
            no_color: false,
            state_dir: None,
        };
        let cmd = parse(&[], &env).unwrap();
        match cmd {
            Cmd::Run(cfg) => {
                assert_eq!(cfg.widgets, Widgets::ALL);
                assert_eq!(cfg.dialect, Dialect::Ansi);
            }
            _ => unreachable!("expected run"),
        }
    }

    #[test]
    fn parse_flags() {
        let env = Env {
            no_color: false,
            state_dir: None,
        };
        let args = [
            OsString::from("cpu"),
            OsString::from("-s"),
            OsString::from("tmux"),
        ];
        let cmd = parse(&args, &env).unwrap();
        match cmd {
            Cmd::Run(cfg) => {
                assert_eq!(cfg.widgets.as_slice(), &[Widget::Cpu]);
                assert_eq!(cfg.dialect, Dialect::Tmux);
            }
            _ => unreachable!("expected run"),
        }
    }
}
