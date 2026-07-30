use std::env;
use std::path::PathBuf;

pub(crate) fn history_file() -> PathBuf {
    env::var_os("HARA_HISTORY")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".hara_history")))
        .unwrap_or_else(|| PathBuf::from(".hara_history"))
}

pub(crate) const DEFAULT_SPLASH: &str = r#"


                               ░░░▒▒▓▒▒░░░
                          ░░░░░▒▒▒▒▒▓▒▒▒▒▒░░░░░
                     ░░░░░▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒░░░░░
                ░░░░░▒▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒▒░░░░░

          ██╗       ██╗   ███████╗    ███████╗     ███████╗
          ██║       ██║  ██╔═════██╗  ██╔════██╗   ██╔═════██╗
          ██║  ●────██║  ██║     ██║  ██║     ██║  ██║     ██║
          ████████████║  ███████████║  ████████╔╝   ███████████║
          ██╔═══════██║  ██╔══════██║  ██╔═══██╗    ██╔══════██║
          ██║ ──●── ██║  ██║  ●───██║  ██║    ██╗   ██║  ●───██║
          ██║       ██║  ██║      ██║  ██║     ██║  ██║      ██║
          ╚═╝       ╚═╝  ╚═╝      ╚═╝  ╚═╝     ╚═╝  ╚═╝      ╚═╝
                ·───────●───────────────●───────────────·
      "#;

pub(crate) fn print_header(resp: &str, include_splash: bool, color: bool) {
    if include_splash {
        println!("{}\n", rendered_splash(color));
    }
    println!("{:<52}SESSION ROOT", "HARA · RUST");
    println!("{}", tagline("JOURNEY WITHIN", color));
    println!("────────────────────────────────────────────────────────────────\n");
    println!("  /docs  Docs       /walkthrough  Tour");
    println!("  /help  Help       /history      History");
    println!("  /status Status    /resp         Listener");
    println!("  /clear Clear      /quit         Exit\n");
    println!("RESP  {resp}\n");
}

pub(crate) fn rendered_splash(color: bool) -> String {
    let value = DEFAULT_SPLASH.trim_end();
    if !color {
        return value.into();
    }
    let lines = value.lines().collect::<Vec<_>>();
    let triangle = &[
        (255, 246, 150),
        (235, 246, 185),
        (170, 226, 230),
        (85, 170, 255),
    ];
    let word = &[
        (105, 245, 255),
        (35, 185, 255),
        (45, 105, 255),
        (105, 65, 235),
        (185, 65, 220),
        (70, 20, 100),
        (5, 8, 20),
    ];
    let word_length = (lines.len().saturating_sub(8)).max(1);
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index < 2 || index == 6 {
                return (*line).to_owned();
            }
            let (position, stops) = if index < 7 {
                ((index - 2) as f64 / 3.0, triangle.as_slice())
            } else {
                ((index - 7) as f64 / word_length as f64, word.as_slice())
            };
            let (r, g, b) = gradient(position, stops);
            format!("\x1b[38;2;{r};{g};{b}m{line}\x1b[0m")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn gradient(position: f64, stops: &[(i32, i32, i32)]) -> (i32, i32, i32) {
    let scaled = position.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let from = (scaled as usize).min(stops.len() - 2);
    let phase = scaled - from as f64;
    let blend = |a: i32, b: i32| (a as f64 + (b - a) as f64 * phase).round() as i32;
    (
        blend(stops[from].0, stops[from + 1].0),
        blend(stops[from].1, stops[from + 1].1),
        blend(stops[from].2, stops[from + 1].2),
    )
}

fn tagline(text: &str, color: bool) -> String {
    if !color {
        return text.into();
    }
    let stops = [
        (100, 245, 255),
        (45, 145, 255),
        (125, 75, 235),
        (220, 90, 205),
    ];
    let length = text.chars().count().saturating_sub(1).max(1);
    let mut result = String::new();
    for (index, ch) in text.chars().enumerate() {
        if ch.is_whitespace() {
            result.push(ch);
        } else {
            let (r, g, b) = gradient(index as f64 / length as f64, &stops);
            result.push_str(&format!("\x1b[38;2;{r};{g};{b}m{ch}"));
        }
    }
    result.push_str("\x1b[0m");
    result
}

pub(crate) fn session_prompt(namespace: &str, color: bool) -> String {
    if color {
        format!("\x1b[2m[\x1b[0m\x1b[36;1m{namespace}\x1b[0m\x1b[2m] \x1b[0m")
    } else {
        format!("[{namespace}] ")
    }
}
pub(crate) fn clear_terminal() {
    print!("\x1b[2J\x1b[H");
}
pub(crate) fn is_terminal() -> bool {
    unsafe { libc_isatty(0) != 0 }
}
#[cfg(unix)]
unsafe fn libc_isatty(fd: i32) -> i32 {
    unsafe extern "C" {
        fn isatty(fd: i32) -> i32;
    }
    unsafe { isatty(fd) }
}
#[cfg(not(unix))]
unsafe fn libc_isatty(_fd: i32) -> i32 {
    0
}
