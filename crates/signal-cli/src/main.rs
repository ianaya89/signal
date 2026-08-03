//! `signal` — CLI companion for the Signal desktop app.
//!
//! Talks newline-delimited JSON to the app's Unix control socket.
//! Scriptable: every command supports `--json` for raw output.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

const HELP: &str = "signal — control the Signal player from the terminal

usage: signal <command> [args]

  status            now playing (add --json for machine output)
  toggle            play / pause
  next              skip forward (queue first, then context)
  prev              restart current track
  stop              stop playback
  play <query>      search and play the first match (context = results)
  seek <s|+s|-s>    seek to seconds, or relative
  vol <n|+n|-n>     volume 0-100, or relative
  add <query>       stage the first match onto the queue
  queue             list the queue
  search <query>    search the library (JSON)
  server <start|stop|status>
                    control the OpenSubsonic mobile server

socket: $SIGNAL_SOCKET or the app data dir. The app must be running.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json_flag = args.iter().any(|a| a == "--json");
    let args: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--json")
        .collect();

    let Some((&cmd, rest)) = args.split_first() else {
        eprintln!("{HELP}");
        return ExitCode::FAILURE;
    };

    let Some(request) = build_request(cmd, rest) else {
        eprintln!("{HELP}");
        return ExitCode::FAILURE;
    };

    let response = match send(&request) {
        Ok(response) => response,
        Err(err) => {
            eprintln!("signal: {err}");
            eprintln!("is the app running?");
            return ExitCode::FAILURE;
        }
    };

    render(cmd, &response, json_flag)
}

fn build_request(cmd: &str, rest: &[&str]) -> Option<String> {
    let joined = rest.join(" ");
    let value = match cmd {
        "status" => serde_json::json!({ "cmd": "status" }),
        "toggle" | "play-pause" => serde_json::json!({ "cmd": "toggle" }),
        "next" => serde_json::json!({ "cmd": "next" }),
        "prev" => serde_json::json!({ "cmd": "prev" }),
        "stop" => serde_json::json!({ "cmd": "stop" }),
        "play" if !joined.is_empty() => {
            serde_json::json!({ "cmd": "play", "query": joined })
        }
        "seek" if rest.len() == 1 => serde_json::json!({ "cmd": "seek", "to": rest[0] }),
        "vol" | "volume" if rest.len() == 1 => {
            serde_json::json!({ "cmd": "volume", "to": rest[0] })
        }
        "add" if !joined.is_empty() => {
            serde_json::json!({ "cmd": "queue-add", "query": joined })
        }
        "queue" => serde_json::json!({ "cmd": "queue-list" }),
        "search" if !joined.is_empty() => {
            serde_json::json!({ "cmd": "search", "query": joined })
        }
        "server" if rest.len() == 1 => match rest[0] {
            "start" => serde_json::json!({ "cmd": "server-start" }),
            "stop" => serde_json::json!({ "cmd": "server-stop" }),
            "status" => serde_json::json!({ "cmd": "server-status" }),
            _ => return None,
        },
        _ => return None,
    };
    Some(value.to_string())
}

fn socket_path() -> String {
    if let Ok(path) = std::env::var("SIGNAL_SOCKET") {
        return path;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if cfg!(target_os = "macos") {
        format!("{home}/Library/Application Support/app.signal.desktop/signal.sock")
    } else {
        let data = std::env::var("XDG_DATA_HOME").unwrap_or(format!("{home}/.local/share"));
        format!("{data}/app.signal.desktop/signal.sock")
    }
}

fn send(request: &str) -> std::io::Result<serde_json::Value> {
    let mut stream = UnixStream::connect(socket_path())?;
    stream.write_all(request.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    serde_json::from_str(&line).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn render(cmd: &str, response: &serde_json::Value, json_flag: bool) -> ExitCode {
    let ok = response.get("ok").and_then(serde_json::Value::as_bool) == Some(true);
    if !ok {
        let error = response
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown error");
        eprintln!("signal: {error}");
        return ExitCode::FAILURE;
    }
    let data = response
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    if json_flag {
        println!("{data}");
        return ExitCode::SUCCESS;
    }

    match cmd {
        "status" => {
            let state = data
                .get("state")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("?");
            match data.get("title").and_then(serde_json::Value::as_str) {
                Some(title) => {
                    let artist = data
                        .get("artist")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("?");
                    let glyph = match state {
                        "playing" => "▶",
                        "paused" => "⏸",
                        _ => "■",
                    };
                    let pos = fmt_ms(data.get("position_ms"));
                    let dur = fmt_ms(data.get("duration_ms"));
                    let bp = if data.get("bit_perfect").and_then(serde_json::Value::as_bool)
                        == Some(true)
                    {
                        " · bit-perfect"
                    } else {
                        ""
                    };
                    println!("{glyph} {artist} — {title}  {pos}/{dur}{bp}");
                }
                None => println!("■ stopped"),
            }
        }
        "queue" => match data.as_array() {
            Some(items) if !items.is_empty() => {
                for (i, item) in items.iter().enumerate() {
                    let title = item
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("?");
                    println!("{:2}  {title}", i + 1);
                }
            }
            _ => println!("queue empty"),
        },
        "search" => println!("{data}"),
        "server" => {
            let running =
                data.get("running").and_then(serde_json::Value::as_bool) == Some(true);
            if running {
                let port = data.get("port").and_then(serde_json::Value::as_u64).unwrap_or(0);
                match data.get("lanIp").and_then(serde_json::Value::as_str) {
                    Some(ip) => println!("● http://{ip}:{port}"),
                    None => println!("● serving on port {port}"),
                }
            } else {
                println!("○ off");
            }
        }
        _ => {
            if let Some(obj) = data.as_object() {
                for (key, value) in obj {
                    println!("{key}: {value}");
                }
            } else {
                println!("ok");
            }
        }
    }
    ExitCode::SUCCESS
}

fn fmt_ms(value: Option<&serde_json::Value>) -> String {
    let ms = value.and_then(serde_json::Value::as_u64).unwrap_or(0);
    let secs = ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
}
