//! `nvidia-cuda-mps-control` drop-in stub — MPS control surface.
//! Full multi-process service is not claimed; commands report honestly.

use std::env;
use std::io::{self, BufRead, Write};
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "nvidia-cuda-mps-control (Hermes GSP drop-in)\n\
             Commands (pipe or argv): get_server_list | quit | help\n\
             Full MPS server is not implemented — fail-closed stubs only."
        );
        return;
    }
    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("CUDA MPS control (Hermes) 0.1.0");
        return;
    }

    if !args.is_empty() {
        for a in &args {
            dispatch(a);
        }
        return;
    }

    // Interactive / pipe mode (classic daemon speaks line commands).
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }
        if cmd == "quit" || cmd == "exit" {
            break;
        }
        let resp = handle(cmd);
        let _ = writeln!(stdout, "{resp}");
        let _ = stdout.flush();
    }
}

fn dispatch(cmd: &str) {
    println!("{}", handle(cmd));
    if cmd == "quit" {
        process::exit(0);
    }
}

fn handle(cmd: &str) -> String {
    match cmd {
        "help" | "?" => {
            "Hermes MPS control stub commands: get_server_list, get_server_status, quit, help\n\
             Note: no MPS server process is started (fail-closed)."
                .into()
        }
        "get_server_list" => String::new(), // empty = no servers (honest)
        "get_server_status" => "MPS server not running (Hermes stub)".into(),
        "start_server" | "start" => {
            "error: Hermes does not invent an MPS server without GSP Online multi-tenant path"
                .into()
        }
        other => format!("error: unknown command '{other}'"),
    }
}
