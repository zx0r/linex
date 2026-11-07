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

pub const HELP: &[u8] = b"linex [WIDGET...] [OPTIONS]\n\nWIDGETS: cpu mem ssd git uptime\nOPTIONS:\n  -s, --style <ansi|tmux|plain>\n  -G, --git-status\n  -b, --bytes\n      --no-icons\n      --no-color\n      --separator <S>\n  -h, --help\n  -V, --version\n";
pub const VERSION: &[u8] = concat!("linex ", env!("CARGO_PKG_VERSION"), "\n").as_bytes();

/// Parse CLI arguments.
///
/// # Errors
///
/// Returns an error message if an unknown argument or missing value is encountered.
#[rustfmt::skip]
pub fn parse<'a>(argv: &'a [std::ffi::OsString], env: &'a Env) -> Result<Cmd<'a>, &'static [u8]> {
    let mut widgets = Widgets::empty();
    let (mut dialect, mut icons, mut bytes, mut git_detail, mut no_color, mut sep, mut path) = (None, true, false, false, false, None, None);
    let mut i = 0;
    while i < argv.len() {
        let arg = argv.get(i).map(|s| s.as_encoded_bytes()).unwrap_or_default();
        i += 1;
        match arg {
            b"-h" | b"--help" => return Ok(Cmd::Help),
            b"-V" | b"--version" => return Ok(Cmd::Version),
            b"-G" | b"--git-status" => git_detail = true,
            b"-b" | b"--bytes" => bytes = true,
            b"-i" | b"--icons" => icons = true,
            b"--no-icons" => icons = false,
            b"--no-color" => no_color = true,
            b"-s" | b"--style" => {
                let val = argv.get(i).map(|s| s.as_encoded_bytes()).ok_or(&b"missing --style value"[..])?;
                i += 1;
                dialect = Some(parse_dialect(val)?);
            }
            b"--separator" => {
                let val = argv.get(i).map(|s| s.as_encoded_bytes()).ok_or(&b"missing --separator value"[..])?;
                i += 1;
                sep = Some(val);
            }
            b"-C" => {
                let val = argv.get(i).ok_or(&b"missing -C value"[..])?;
                i += 1;
                path = Some(std::path::Path::new(val));
            }
            _ if arg.starts_with(b"--style=") => dialect = Some(parse_dialect(&arg[8..])?),
            _ if arg.starts_with(b"--separator=") => sep = Some(&arg[12..]),
            _ if arg.starts_with(b"-C") && arg.len() > 2 => {
                if let Some(os_arg) = argv.get(i - 1) {
                    let bytes = os_arg.as_encoded_bytes();
                    // SAFETY: slicing valid encoded bytes from index 2 onwards
                    let sub = unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(&bytes[2..]) };
                    path = Some(std::path::Path::new(sub));
                }
            }
            b"cpu" => widgets.push(Widget::Cpu),
            b"mem" => widgets.push(Widget::Mem),
            b"ssd" => widgets.push(Widget::Ssd),
            b"git" => widgets.push(Widget::Git),
            b"uptime" => widgets.push(Widget::Uptime),
            _ if !arg.starts_with(b"-") => {
                if path.is_none() && !arg.is_empty() {
                    path = argv.get(i - 1).map(std::path::Path::new);
                }
            }
            _ => return Err(&b"unknown argument"[..]),
        }
    }
    if widgets.is_empty() { widgets = Widgets::ALL; }
    let d = dialect.unwrap_or(if no_color || env.no_color { Dialect::Plain } else { Dialect::Tmux });
    let icons = icons && d != Dialect::Plain;
    Ok(Cmd::Run(crate::Config {
        widgets, dialect: d, icons, bytes, git_detail,
        sep: sep.unwrap_or(b" "),
        path,
        state_dir: env.state_dir.as_deref(),
    }))
}

fn parse_dialect(s: &[u8]) -> Result<Dialect, &'static [u8]> {
    match s {
        b"ansi" => Ok(Dialect::Ansi),
        b"tmux" => Ok(Dialect::Tmux),
        b"plain" | b"none" => Ok(Dialect::Plain),
        _ => Err(&b"invalid style"[..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    #[rustfmt::skip]
    fn parse_tests() {
        let env = Env { no_color: false, state_dir: None };
        let Cmd::Run(c1) = parse(&[], &env).unwrap() else { unreachable!() };
        assert_eq!((c1.widgets, c1.dialect, c1.icons, c1.bytes, c1.git_detail), (Widgets::ALL, Dialect::Tmux, true, false, false));

        let args = [OsString::from("cpu"), OsString::from("-s"), OsString::from("tmux"), OsString::from("-G"), OsString::from("--bytes"), OsString::from("--no-icons")];
        let Cmd::Run(c2) = parse(&args, &env).unwrap() else { unreachable!() };
        assert_eq!((c2.widgets.as_slice(), c2.dialect, c2.bytes, c2.git_detail, c2.icons), (&[Widget::Cpu][..], Dialect::Tmux, true, true, false));
    }
}
