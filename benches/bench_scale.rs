//! High-density architectural benchmark for linex widgets and formatting engine.

#![allow(
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::uninlined_format_args,
    reason = "benchmark harness stdout reporting and timing calculation"
)]

use std::hint::black_box;
use std::time::Instant;

fn bench<F: FnMut()>(name: &str, iters: u64, mut f: F) {
    // Warmup
    for _ in 0..iters.min(100) {
        f();
    }
    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    let elapsed = start.elapsed();
    let per_iter_ns = (elapsed.as_nanos() as f64) / (iters as f64);
    println!(
        "{:<28} {:>10} iters {:>10.2} ns/iter",
        name, iters, per_iter_ns
    );
}

fn main() {
    if std::env::args().any(|arg| arg == "--list") {
        return;
    }
    println!("=== linex architectural benchmark suite ===");

    // 1. Benchmark Out stack buffer operations
    bench("fmt::Out push & itoa", 100_000, || {
        let mut out = linex::fmt::Out::new();
        out.push(b"prefix: ");
        linex::fmt::u64(&mut out, 123_456);
        out.byte(b'%');
        black_box(out.as_bytes());
    });

    // 2. Benchmark format dialect styling
    bench("fmt::style (tmux matrix)", 100_000, || {
        let mut out = linex::fmt::Out::new();
        linex::fmt::style(
            &mut out,
            linex::fmt::Dialect::Tmux,
            linex::fmt::Color::Green,
        );
        out.push(b"test");
        linex::fmt::style(
            &mut out,
            linex::fmt::Dialect::Tmux,
            linex::fmt::Color::Reset,
        );
        black_box(out.as_bytes());
    });

    // 3. Benchmark CLI argument parsing
    bench("args::parse", 50_000, || {
        let env = linex::args::Env {
            no_color: false,
            state_dir: None,
        };
        let args = [
            std::ffi::OsString::from("cpu"),
            std::ffi::OsString::from("mem"),
            std::ffi::OsString::from("-s"),
            std::ffi::OsString::from("tmux"),
        ];
        let res = linex::args::parse(&args, &env);
        let _ = black_box(res);
    });

    // 4. Benchmark widget rendering
    let temp = std::env::temp_dir();
    let cfg = linex::Config {
        widgets: linex::Widgets::ALL,
        dialect: linex::fmt::Dialect::Plain,
        icons: false,
        bytes: false,
        git_detail: false,
        sep: b" | ",
        state_dir: Some(temp.as_os_str()),
    };
    bench("render (all widgets)", 5_000, || {
        let mut out = linex::fmt::Out::new();
        let _ = linex::render(&cfg, &mut out);
        black_box(out.as_bytes());
    });
}
