use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let env = statty::args::Env {
        no_color: std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()),
        state_dir: statty::sys::state_dir(),
    };
    let cfg = match statty::args::parse(&argv, &env) {
        Ok(statty::args::Cmd::Run(cfg)) => cfg,
        Ok(statty::args::Cmd::Help) => return emit(statty::args::HELP),
        Ok(statty::args::Cmd::Version) => return emit(statty::args::VERSION),
        Err(msg) => {
            statty::sys::write_stderr(msg);
            return ExitCode::from(2);
        }
    };
    let mut out = statty::fmt::Out::new();
    let healthy = statty::render(&cfg, &mut out);
    if !statty::sys::write_stdout(out.as_bytes()) {
        return ExitCode::FAILURE;
    }
    if healthy {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn emit(msg: &[u8]) -> ExitCode {
    if statty::sys::write_stdout(msg) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
