use crate::repl;
use hara_wasm::native_cli::RuntimeBroker;
use hara_wasm::package;
use hara_wasm::resp::{RespConnection, RespServer, RespValue};
use hara_wasm::Runtime;
use std::env;
use std::fs;
use std::io::{self, BufRead, Read};
use std::net::TcpStream;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct Options {
    pub(crate) root: Option<PathBuf>,
    pub(crate) native_sockets: bool,
    pub(crate) offline: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
    command: Vec<String>,
    pub(crate) history_file: Option<PathBuf>,
    pub(crate) no_history: bool,
    pub(crate) no_splash: bool,
    pub(crate) no_color: bool,
}

pub(crate) fn parse_options() -> Result<Options, String> {
    let mut options = Options { host: "127.0.0.1".into(), port: 1311, ..Options::default() };
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => { usage(); std::process::exit(0); }
            "--version" | "-V" => { println!("hara native {}", env!("CARGO_PKG_VERSION")); std::process::exit(0); }
            "--root" => options.root = Some(PathBuf::from(required(&mut args, "--root")?)),
            "--native-sockets" | "--allow-net" => options.native_sockets = true,
            "--offline" => options.offline = true,
            "--no-history" => options.no_history = true,
            "--no-splash" => options.no_splash = true,
            "--no-color" => options.no_color = true,
            "--history" => options.history_file = Some(PathBuf::from(required(&mut args, "--history")?)),
            "--host" => options.host = required(&mut args, "--host")?,
            "--port" => options.port = required(&mut args, "--port")?.parse().map_err(|_| "--port must be between 0 and 65535".to_owned())?,
            value if value.starts_with("--history=") => options.history_file = Some(PathBuf::from(&value[10..])),
            value if value.starts_with('-') => return Err(format!("unknown option: {value}")),
            value => { options.command.push(value.into()); options.command.extend(args); break; }
        }
    }
    Ok(options)
}

fn required(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{option} requires a value"))
}

pub(crate) fn run(options: Options) -> Result<(), String> {
    match options.command.first().map(String::as_str) {
        Some("package") => package::run(&options.command[1..]),
        Some("eval") => direct_eval(&options, &options.command[1..].join(" ")),
        Some("run") | Some("--file") => run_file(&options, options.command.get(1).ok_or_else(|| "run requires a file path".to_owned())?),
        Some("stdin") => { let mut source = String::new(); io::stdin().read_to_string(&mut source).map_err(|error| format!("stdin: {error}"))?; direct_eval(&options, &source) }
        Some("headless" | "server") => run_headless(&options),
        Some("remote") => run_remote(options.command.get(1).ok_or_else(|| "remote requires HOST:PORT".to_owned())?),
        Some("standalone") => repl::run_repl(&options, true),
        Some("repl") | None => repl::run_repl(&options, options.offline),
        Some(command) => Err(format!("unknown command: {command}")),
    }
}

fn direct_eval(options: &Options, source: &str) -> Result<(), String> {
    if source.is_empty() { return Err("eval requires a Hara expression".into()); }
    let mut runtime = Runtime::new();
    if let Some(root) = &options.root { runtime.install_native_file_provider(root.to_string_lossy().as_ref()); }
    if options.native_sockets { runtime.install_native_socket_provider(); }
    println!("{}", runtime.eval_native(source)?);
    Ok(())
}

fn run_file(options: &Options, path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let is_hir = path.ends_with(".hir") || bytes.starts_with(b"HIR\0");
    let mut runtime = Runtime::new();
    if let Some(root) = &options.root { runtime.install_native_file_provider(root.to_string_lossy().as_ref()); }
    if options.native_sockets { runtime.install_native_socket_provider(); }
    if is_hir { println!("{}", runtime.eval_hir(&bytes)?); } else { println!("{}", runtime.eval_native(&String::from_utf8(bytes).map_err(|error| format!("{path} is not valid UTF-8: {error}"))?)?); }
    Ok(())
}

fn run_headless(options: &Options) -> Result<(), String> {
    if options.offline { return Err("--offline cannot be used with headless".into()); }
    let broker = RuntimeBroker::start_with(options.root.clone(), options.native_sockets)?;
    let server = RespServer::start(&options.host, options.port, broker)?;
    println!("HARA RESP {} · session ROOT", server.endpoint());
    loop { std::thread::park(); }
}

fn run_remote(endpoint: &str) -> Result<(), String> {
    let (host, port) = repl::parse_endpoint(endpoint, "127.0.0.1")?;
    let stream = TcpStream::connect((host.as_str(), port)).map_err(|error| format!("remote connect failed: {error}"))?;
    let mut connection = RespConnection::new(stream)?;
    connection.write(&RespValue::array(["HELLO", "4", "CLIENT", "HARA-REMOTE"]))?;
    println!("{}", response_text(connection.read()?.ok_or("remote closed")?));
    let mut request = 0_u64;
    for line in io::stdin().lock().lines() {
        let source = line.map_err(|error| format!("stdin: {error}"))?;
        if matches!(source.trim(), "/quit" | ":quit") { connection.write(&RespValue::array(["QUIT"]))?; break; }
        request += 1;
        let id = format!("REMOTE-{request}");
        connection.write(&RespValue::array(["EVAL", &id, source.trim()]))?;
        if let Some(value) = connection.read()? { println!("{}", response_text(value)); }
        let _ = connection.read()?;
    }
    Ok(())
}

fn response_text(value: RespValue) -> String {
    match value {
        RespValue::Array(Some(values)) => values.into_iter().map(response_text).collect::<Vec<_>>().join(" "),
        RespValue::Bulk(Some(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
        RespValue::Simple(value) | RespValue::Error(value) => value,
        RespValue::Integer(value) => value.to_string(),
        RespValue::Bulk(None) | RespValue::Array(None) => "nil".into(),
    }
}

fn usage() {
    println!("hara [OPTIONS] [repl|standalone|headless|server|remote HOST:PORT|eval SOURCE|run FILE|stdin]");
    println!("  --offline --host HOST --port PORT --root PATH --allow-net");
    println!("  --history PATH --no-history --no-splash --no-color");
}

pub(crate) fn exit_error(message: &str, status: i32) -> ! {
    eprintln!("hara: {message}");
    std::process::exit(status)
}
