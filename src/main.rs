use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let env = linex::args::Env {
        no_color: std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()),
        state_dir: linex::sys::state_dir(),
    };
    let cfg = match linex::args::parse(&argv, &env) {
        Ok(linex::args::Cmd::Run(cfg)) => cfg,
        Ok(linex::args::Cmd::Help) => return emit(linex::args::HELP),
        Ok(linex::args::Cmd::Version) => return emit(linex::args::VERSION),
        Err(msg) => {
            linex::sys::write_stderr(msg);
            return ExitCode::from(2);
        }
    };
    let mut out = linex::fmt::Out::new();
    let healthy = linex::render(&cfg, &mut out);
    if !linex::sys::write_stdout(out.as_bytes()) {
        return ExitCode::FAILURE;
    }
    if healthy {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn emit(msg: &[u8]) -> ExitCode {
    if linex::sys::write_stdout(msg) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
