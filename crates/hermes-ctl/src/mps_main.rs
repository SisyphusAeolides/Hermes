//! `nvidia-cuda-mps-control` drop-in control plane.
//!
//! The control protocol is implemented as a small Unix-socket broker so the
//! command behaves like the standard MPS client: `-d` starts a persistent
//! control endpoint and subsequent invocations send line commands to it.
//! A server is started only after the real Hermes `/dev/nvidiactl` status says
//! that the GSP session is Online. No process or GPU capacity is fabricated
//! when that prerequisite is absent.

use hermes_core::chaos::ChaosScheduler;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{self, Command};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STATUS_IOCTL: u64 = (2u64 << 30) | (0x48u64 << 8) | 0x10 | (16u64 << 16);
const ONLINE_PHASE: u32 = 5;

unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, argp: *mut u8) -> i32;
}

#[derive(Debug)]
struct MpsServer {
    pid: u32,
    started_at: u64,
    active_thread_percentage: u32,
    default_pinned_mem_limit: u64,
}

struct MpsState {
    server: Option<MpsServer>,
    sequence: u64,
    /// Chaos-derived service quantum (microseconds) for fair command turns.
    service_quantum_us: u32,
    chaos: ChaosScheduler,
}

impl MpsState {
    fn new() -> Self {
        Self {
            server: None,
            sequence: 0,
            service_quantum_us: 1,
            chaos: ChaosScheduler::new(),
        }
    }

    fn tick(&mut self) {
        self.sequence = self.sequence.wrapping_add(1);
        self.service_quantum_us = self.chaos.next_interval(0.01).max(1);
    }
}

fn pipe_dir() -> PathBuf {
    env::var_os("CUDA_MPS_PIPE_DIRECTORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/nvidia-mps"))
}

fn socket_path() -> PathBuf {
    pipe_dir().join("control")
}

fn socket_ready() -> bool {
    fs::symlink_metadata(socket_path())
        .map(|metadata| metadata.file_type().is_socket())
        .unwrap_or(false)
}

fn gsp_online() -> bool {
    let file = match File::open("/dev/nvidiactl") {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut status = [0u32; 4];
    // SAFETY: `status` is a writable buffer of the exact UAPI size and the
    // file descriptor is owned by this function for the ioctl duration.
    let rc = unsafe {
        ioctl(
            file.as_raw_fd(),
            STATUS_IOCTL,
            status.as_mut_ptr().cast::<u8>(),
        )
    };
    rc == 0 && status[0] == 1 && status[1] == ONLINE_PHASE
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn parse_percent(value: Option<&str>) -> Result<u32, String> {
    let value = value.ok_or_else(|| "missing percentage".to_string())?;
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("invalid percentage '{value}'"))?;
    if parsed > 100 {
        return Err("percentage must be between 0 and 100".into());
    }
    Ok(parsed)
}

fn parse_bytes(value: Option<&str>) -> Result<u64, String> {
    let value = value.ok_or_else(|| "missing byte limit".to_string())?;
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid byte limit '{value}'"))
}

fn handle_command(state: &mut MpsState, command: &str) -> (String, bool) {
    let mut words = command.split_whitespace();
    let verb = match words.next() {
        Some(verb) => verb,
        None => return (String::new(), false),
    };
    state.tick();

    match verb {
        "help" | "?" => (
            "Hermes MPS commands: get_server_list, get_server_status, start_server, \
             stop_server, set_default_active_thread_percentage <0..100>, \
             get_default_active_thread_percentage, set_default_device_pinned_mem_limit <bytes>, \
             get_default_device_pinned_mem_limit, quit"
                .into(),
            false,
        ),
        "get_server_list" => match &state.server {
            Some(server) => (format!("Server PID: {}", server.pid), false),
            None => ("No MPS servers running".into(), false),
        },
        "get_server_status" => match &state.server {
            Some(server) => (
                format!(
                    "MPS server pid={} started={} active_thread_percentage={} default_pinned_mem_limit={} service_quantum_us={}",
                    server.pid,
                    server.started_at,
                    server.active_thread_percentage,
                    server.default_pinned_mem_limit,
                    state.service_quantum_us
                ),
                false,
            ),
            None => ("MPS server is not running".into(), false),
        },
        "start_server" | "start" => {
            if state.server.is_some() {
                return ("MPS server is already running".into(), false);
            }
            if !gsp_online() {
                return (
                    "error: Hermes GSP status is not Online; start_server requires a live GPU session"
                        .into(),
                    false,
                );
            }
            state.server = Some(MpsServer {
                pid: process::id(),
                started_at: now_seconds(),
                active_thread_percentage: 100,
                default_pinned_mem_limit: 0,
            });
            (
                format!(
                    "MPS server started pid={} service_quantum_us={}",
                    process::id(),
                    state.service_quantum_us
                ),
                false,
            )
        }
        "stop_server" | "stop" => {
            if state.server.take().is_some() {
                ("MPS server stopped".into(), false)
            } else {
                ("MPS server is not running".into(), false)
            }
        }
        "set_default_active_thread_percentage" => match parse_percent(words.next()) {
            Ok(value) => match state.server.as_mut() {
                Some(server) => {
                    server.active_thread_percentage = value;
                    (format!("OK {value}"), false)
                }
                None => ("error: MPS server is not running".into(), false),
            },
            Err(error) => (format!("error: {error}"), false),
        },
        "get_default_active_thread_percentage" => match &state.server {
            Some(server) => (server.active_thread_percentage.to_string(), false),
            None => ("error: MPS server is not running".into(), false),
        },
        "set_default_device_pinned_mem_limit" => match parse_bytes(words.next()) {
            Ok(value) => match state.server.as_mut() {
                Some(server) => {
                    server.default_pinned_mem_limit = value;
                    (format!("OK {value}"), false)
                }
                None => ("error: MPS server is not running".into(), false),
            },
            Err(error) => (format!("error: {error}"), false),
        },
        "get_default_device_pinned_mem_limit" => match &state.server {
            Some(server) => (server.default_pinned_mem_limit.to_string(), false),
            None => ("error: MPS server is not running".into(), false),
        },
        "quit" | "exit" => ("Bye".into(), true),
        other => (format!("error: unknown command '{other}'"), false),
    }
}

fn prepare_socket() -> io::Result<(PathBuf, UnixListener)> {
    let directory = pipe_dir();
    fs::create_dir_all(&directory)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    let path = socket_path();
    if path.exists() {
        // Only remove a filesystem socket at this exact operator-selected
        // path; never unlink an arbitrary file from the pipe directory.
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_socket() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} exists and is not a Unix socket", path.display()),
            ));
        }
        fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    Ok((path, listener))
}

fn run_daemon() -> io::Result<()> {
    let (path, listener) = prepare_socket()?;
    let mut state = MpsState::new();
    for incoming in listener.incoming() {
        let mut stream = match incoming {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("nvidia-cuda-mps-control: accept failed: {error}");
                continue;
            }
        };
        let mut input = String::new();
        stream.read_to_string(&mut input)?;
        let mut quit = false;
        let mut output = String::new();
        for line in input.lines() {
            let (response, should_quit) = handle_command(&mut state, line.trim());
            if !response.is_empty() {
                output.push_str(&response);
                output.push('\n');
            }
            quit |= should_quit;
            if quit {
                break;
            }
        }
        stream.write_all(output.as_bytes())?;
        stream.flush()?;
        if quit {
            break;
        }
    }
    let _ = fs::remove_file(path);
    Ok(())
}

fn send_command(command: &str) -> io::Result<String> {
    let mut stream = UnixStream::connect(socket_path())?;
    stream.write_all(command.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

fn print_help() {
    println!(
        "nvidia-cuda-mps-control (Hermes GSP)\n\
         Usage: nvidia-cuda-mps-control -d | [command]\n\
         Commands: get_server_list, get_server_status, start_server, stop_server,\n\
         set_default_active_thread_percentage N, get_default_active_thread_percentage,\n\
         set_default_device_pinned_mem_limit BYTES, get_default_device_pinned_mem_limit, quit\n\
         Pipe directory: $CUDA_MPS_PIPE_DIRECTORY (default /tmp/nvidia-mps)\n\
         start_server requires the primary Hermes GSP status to be ONLINE."
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return;
    }
    if args.iter().any(|arg| arg == "-v" || arg == "--version") {
        println!("CUDA MPS control (Hermes) 0.2.0");
        return;
    }
    if args.iter().any(|arg| arg == "--daemon-foreground") {
        if let Err(error) = run_daemon() {
            eprintln!("nvidia-cuda-mps-control: daemon failed: {error}");
            process::exit(1);
        }
        return;
    }
    if args.iter().any(|arg| arg == "-d" || arg == "--daemon") {
        let exe = match env::current_exe() {
            Ok(exe) => exe,
            Err(error) => {
                eprintln!("nvidia-cuda-mps-control: cannot locate executable: {error}");
                process::exit(1);
            }
        };
        match Command::new(exe).arg("--daemon-foreground").spawn() {
            Ok(mut child) => {
                // Do not report a running endpoint before bind succeeds.  The
                // bounded wait also removes the startup race between `-d` and
                // the first control command.
                for _ in 0..200 {
                    if socket_ready() {
                        println!("nvidia-cuda-mps-control daemon started pid={}", child.id());
                        return;
                    }
                    if let Ok(Some(status)) = child.try_wait() {
                        eprintln!(
                            "nvidia-cuda-mps-control: daemon exited during startup ({status})"
                        );
                        process::exit(1);
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                let _ = child.kill();
                let _ = child.wait();
                eprintln!(
                    "nvidia-cuda-mps-control: daemon did not create {}",
                    socket_path().display()
                );
                process::exit(1);
            }
            Err(error) => {
                eprintln!("nvidia-cuda-mps-control: cannot start daemon: {error}");
                process::exit(1);
            }
        }
    }

    let commands: Vec<String> = if args.is_empty() {
        let stdin = io::stdin();
        stdin.lock().lines().map_while(Result::ok).collect()
    } else {
        vec![args.join(" ")]
    };
    if commands.is_empty() {
        return;
    }
    for command in commands {
        match send_command(command.trim()) {
            Ok(response) => print!("{response}"),
            Err(error) => {
                eprintln!(
                    "nvidia-cuda-mps-control: control endpoint unavailable ({}): {error}",
                    socket_path().display()
                );
                process::exit(1);
            }
        }
    }
}
