use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HTML: &str = include_str!("../../../../assets/hermes-settings/index.html");
const MAX_REQUEST: usize = 64 * 1024;
const IDLE_TIMEOUT_SECONDS: u64 = 30;

#[derive(Clone, Copy)]
struct HermesProfile {
    id: &'static str,
    name: &'static str,
    tuned_profile: &'static str,
    description: &'static str,
}

const PROFILES: &[HermesProfile] = &[
    HermesProfile {
        id: "performance",
        name: "Performance",
        tuned_profile: "throughput-performance",
        description: "Prioritize sustained CPU and system throughput.",
    },
    HermesProfile {
        id: "balanced",
        name: "Balanced",
        tuned_profile: "balanced",
        description: "General purpose performance and power balance.",
    },
    HermesProfile {
        id: "power-saver",
        name: "Power Saver",
        tuned_profile: "powersave",
        description: "Reduce power use and extend battery life.",
    },
    HermesProfile {
        id: "battery-saver",
        name: "Battery Saver",
        tuned_profile: "laptop-battery-powersave",
        description: "Use the laptop profile with more aggressive savings.",
    },
    HermesProfile {
        id: "low-latency",
        name: "Low Latency",
        tuned_profile: "latency-performance",
        description: "Favor deterministic response times over efficiency.",
    },
];

fn main() -> io::Result<()> {
    let token = session_token()?;
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let url = format!("http://{address}/?token={token}");
    open_browser(&url);
    eprintln!("Hermes GPU Settings is available at {url}");

    let last_activity = Arc::new(AtomicU64::new(unix_time()));
    loop {
        match listener.accept() {
            Ok((stream, peer)) => {
                if !peer.ip().is_loopback() {
                    continue;
                }
                let token = token.clone();
                let last_activity = Arc::clone(&last_activity);
                thread::spawn(move || {
                    if let Err(error) = serve(stream, &token, &last_activity) {
                        eprintln!("Hermes GPU Settings request failed: {error}");
                    }
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if unix_time().saturating_sub(last_activity.load(Ordering::Relaxed))
                    >= IDLE_TIMEOUT_SECONDS
                {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn session_token() -> io::Result<String> {
    let mut bytes = [0_u8; 24];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().fold(String::new(), |mut token, byte| {
        let _ = write!(token, "{byte:02x}");
        token
    }))
}

fn open_browser(url: &str) {
    if let Err(error) = Command::new("xdg-open").arg(url).spawn() {
        eprintln!("Could not open the desktop browser: {error}");
    }
}

fn serve(mut stream: TcpStream, token: &str, last_activity: &AtomicU64) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let (method, target, body) = read_request(&mut stream)?;
    let (path, query) = target
        .as_str()
        .split_once('?')
        .unwrap_or((target.as_str(), ""));
    let query_token = query
        .split('&')
        .find_map(|part| part.strip_prefix("token="));

    if path == "/" && method == "GET" {
        last_activity.store(unix_time(), Ordering::Relaxed);
        return respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            HTML.as_bytes(),
        );
    }

    if path == "/api/status"
        && method == "GET"
        && query_token.is_some_and(|candidate| candidate == token)
    {
        last_activity.store(unix_time(), Ordering::Relaxed);
        let body = status_json();
        return respond(
            &mut stream,
            "200 OK",
            "application/json; charset=utf-8",
            body.as_bytes(),
        );
    }

    if path == "/api/profile"
        && method == "POST"
        && query_token.is_some_and(|candidate| candidate == token)
    {
        last_activity.store(unix_time(), Ordering::Relaxed);
        let response = apply_profile(&body);
        return respond(
            &mut stream,
            "200 OK",
            "application/json; charset=utf-8",
            response.as_bytes(),
        );
    }

    respond(
        &mut stream,
        "404 Not Found",
        "text/plain; charset=utf-8",
        b"Not found",
    )
}

fn read_request(stream: &mut TcpStream) -> io::Result<(String, String, Vec<u8>)> {
    let mut buffer = Vec::new();
    let header_end = loop {
        if buffer.len() >= MAX_REQUEST {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP request is too large",
            ));
        }
        let mut chunk = [0_u8; 4096];
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete HTTP request",
            ));
        }
        buffer.extend_from_slice(&chunk[..count]);
        if let Some(end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break end + 4;
        }
    };

    let header = String::from_utf8_lossy(&buffer[..header_end]);
    let mut request_line = header.lines().next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_string();
    let target = request_line.next().unwrap_or_default().to_string();
    let content_length = header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    if header_end + content_length > MAX_REQUEST {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "HTTP request body is too large",
        ));
    }
    while buffer.len() < header_end + content_length {
        let mut chunk = [0_u8; 4096];
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete HTTP request body",
            ));
        }
        buffer.extend_from_slice(&chunk[..count]);
    }
    Ok((
        method,
        target,
        buffer[header_end..header_end + content_length].to_vec(),
    ))
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self'\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)
}

fn status_json() -> String {
    let graphics = command_output("hermes-ctl", &["graphics-status"]);
    let settings = command_output("nvidia-settings", &["--status"]);
    let power = command_output("tuned-adm", &["active"]);
    let active_profile = active_profile_name(&power);
    format!(
        "{{\"graphics\":{},\"settings\":{},\"power\":{},\"active_profile\":{},\"profiles\":{}}}",
        json_string(&graphics),
        json_string(&settings),
        json_string(&power),
        json_string(&active_profile),
        profile_catalog_json()
    )
}

fn command_output(program: &str, arguments: &[&str]) -> String {
    command_result(program, arguments).1
}

fn command_result(program: &str, arguments: &[&str]) -> (bool, String) {
    let candidates = [format!("/usr/local/bin/{program}"), program.to_string()];
    let mut last_error = None;
    for candidate in candidates {
        let output = Command::new(&candidate).args(arguments).output();
        let Ok(output) = output else {
            last_error = output.err();
            continue;
        };
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if text.trim().is_empty() {
            text = String::from_utf8_lossy(&output.stderr).into_owned();
        }
        return (output.status.success(), text.trim().to_string());
    }
    (
        false,
        format!(
            "{program} is unavailable: {}",
            last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "unknown error".to_string())
        ),
    )
}

fn active_profile_name(power_output: &str) -> String {
    power_output
        .lines()
        .find_map(|line| line.strip_prefix("Current active profile:"))
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn profile_catalog_json() -> String {
    let profiles = PROFILES
        .iter()
        .map(|profile| {
            format!(
                "{{\"id\":{},\"name\":{},\"description\":{},\"tuned_profile\":{}}}",
                json_string(profile.id),
                json_string(profile.name),
                json_string(profile.description),
                json_string(profile.tuned_profile),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{profiles}]")
}

fn apply_profile(body: &[u8]) -> String {
    let profile_id = String::from_utf8_lossy(body);
    let Some(profile_id) = profile_id
        .split_once("\"profile\"")
        .and_then(|(_, value)| value.split_once(':'))
        .and_then(|(_, value)| value.trim().strip_prefix('"'))
        .and_then(|value| value.split_once('"').map(|(id, _)| id))
    else {
        return profile_response(false, "Invalid profile request");
    };
    let Some(profile) = PROFILES.iter().find(|profile| profile.id == profile_id) else {
        return profile_response(false, "Unknown Hermes profile");
    };
    let (success, output) = command_result("tuned-adm", &["profile", profile.tuned_profile]);
    if success {
        profile_response(true, &format!("{} profile applied", profile.name))
    } else {
        profile_response(false, &output)
    }
}

fn profile_response(ok: bool, message: &str) -> String {
    format!("{{\"ok\":{ok},\"message\":{}}}", json_string(message))
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                let _ = write!(escaped, "\\u{:04x}", character as u32);
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}
