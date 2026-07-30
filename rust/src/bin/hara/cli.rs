use crate::repl;
use hara_wasm::native_cli::RuntimeBroker;
use hara_wasm::package;
use hara_wasm::project;
use hara_wasm::resp::{FabricRespServer, RespConnection, RespServer, RespValue};
use hara_wasm::service::{FabricService, ServiceConfig};
use hara_wasm::kernel::{parse, Form};
#[cfg(feature = "hir-encoder")]
use hara_wasm::kernel::{hir::encode_hir_module, parse_forms};
use hara_wasm::Runtime;
use std::env;
use std::fs;
use std::io::{self, BufRead, Read};
use std::net::TcpStream;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct Options {
    pub(crate) root: Option<PathBuf>,
    pub(crate) project: Option<PathBuf>,
    pub(crate) native_sockets: bool,
    pub(crate) offline: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
    command: Vec<String>,
    pub(crate) history_file: Option<PathBuf>,
    pub(crate) no_history: bool,
    pub(crate) no_splash: bool,
    pub(crate) no_color: bool,
    data_dir: Option<PathBuf>,
    shards: Option<usize>,
    node_id: String,
    peers: Vec<String>,
    cluster_epoch: String,
}

pub(crate) fn parse_options() -> Result<Options, String> {
    let mut options = Options {
        host: "127.0.0.1".into(),
        port: 1311,
        node_id: "local".into(),
        cluster_epoch: "local.v1".into(),
        ..Options::default()
    };
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                usage();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("hara native {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--root" => options.root = Some(PathBuf::from(required(&mut args, "--root")?)),
            "--project" => options.project = Some(PathBuf::from(required(&mut args, "--project")?)),
            "--native-sockets" | "--allow-net" => options.native_sockets = true,
            "--offline" => options.offline = true,
            "--no-history" => options.no_history = true,
            "--no-splash" => options.no_splash = true,
            "--no-color" => options.no_color = true,
            "--history" => {
                options.history_file = Some(PathBuf::from(required(&mut args, "--history")?))
            }
            "--host" => options.host = required(&mut args, "--host")?,
            "--port" => {
                options.port = required(&mut args, "--port")?
                    .parse()
                    .map_err(|_| "--port must be between 0 and 65535".to_owned())?
            }
            "--data" => options.data_dir = Some(PathBuf::from(required(&mut args, "--data")?)),
            "--shards" => {
                options.shards = Some(
                    required(&mut args, "--shards")?
                        .parse()
                        .map_err(|_| "--shards must be a positive integer".to_owned())?,
                )
            }
            "--node-id" => options.node_id = required(&mut args, "--node-id")?,
            "--peer" => options.peers.push(required(&mut args, "--peer")?),
            "--cluster-epoch" => {
                options.cluster_epoch = required(&mut args, "--cluster-epoch")?
            }
            value if value.starts_with("--history=") => {
                options.history_file = Some(PathBuf::from(&value[10..]))
            }
            value if value.starts_with('-') => return Err(format!("unknown option: {value}")),
            value => {
                options.command.push(value.into());
                options.command.extend(args);
                break;
            }
        }
    }
    Ok(options)
}

fn required(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

pub(crate) fn run(options: Options) -> Result<(), String> {
    match options.command.first().map(String::as_str) {
        Some("package") => package::run(&options.command[1..]),
        #[cfg(feature = "hir-encoder")]
        Some("compile-hir") => compile_hir(&options.command[1..]),
        Some("new") => new_project(&options.command[1..]),
        Some("check") => check_project(&options, &options.command[1..]),
        Some("add") => edit_dependency(&options, &options.command[1..], true),
        Some("remove") => edit_dependency(&options, &options.command[1..], false),
        Some("sync") => sync_project(&options, &options.command),
        Some("update") => Err("project update requires the reviewed registry client".into()),
        Some("test") => test_project(&options, &options.command[1..]),
        Some("eval") => direct_eval(&options, &options.command[1..].join(" ")),
        Some("run") if options.command.len() == 1 => run_project(&options),
        Some("run") | Some("--file") => run_file(&options, options.command.get(1).ok_or_else(|| "run requires a file path".to_owned())?),
        Some("stdin") => { let mut source = String::new(); io::stdin().read_to_string(&mut source).map_err(|error| format!("stdin: {error}"))?; direct_eval(&options, &source) }
        Some("headless" | "server") => run_headless(&options),
        Some("fabric" | "service") => run_fabric(&options),
        Some("remote") => run_remote(
            options
                .command
                .get(1)
                .ok_or_else(|| "remote requires HOST:PORT".to_owned())?,
        ),
        Some("standalone") => repl::run_repl(&options, true),
        Some("repl") | None => repl::run_repl(&options, options.offline),
        Some(command) => Err(format!("unknown command: {command}")),
    }
}

#[cfg(feature = "hir-encoder")]
fn compile_hir(args: &[String]) -> Result<(), String> {
    let source_path = args
        .first()
        .ok_or_else(|| "compile-hir requires SOURCE.hal --output OUTPUT.hir".to_owned())?;
    let output_index = args
        .iter()
        .position(|argument| argument == "--output")
        .ok_or_else(|| "compile-hir requires --output OUTPUT.hir".to_owned())?;
    let output_path = args
        .get(output_index + 1)
        .ok_or_else(|| "compile-hir requires --output OUTPUT.hir".to_owned())?;
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("cannot read {source_path}: {error}"))?;
    let forms = parse_forms(&source)?;
    let namespace = forms
        .iter()
        .find_map(|form| match form {
            Form::List(values)
                if matches!(values.first(), Some(Form::Symbol(head)) if head == "ns" || head == "ns+") =>
            {
                match values.get(1) {
                    Some(Form::Symbol(namespace)) => Some(namespace.clone()),
                    _ => None,
                }
            }
            _ => None,
        })
        .ok_or_else(|| format!("{source_path} does not declare an ns or ns+ namespace"))?;
    let artifact = encode_hir_module(&namespace, source_path, &source, forms);
    if let Some(parent) = std::path::Path::new(output_path).parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(output_path, artifact)
        .map_err(|error| format!("cannot write {output_path}: {error}"))
}

fn project_for(options: &Options, args: &[String]) -> Result<project::Project, String> {
    let path = args.first().map(PathBuf::from).or_else(|| options.project.clone()).unwrap_or_else(|| PathBuf::from("."));
    project::discover(&path)
}

fn new_project(args: &[String]) -> Result<(), String> {
    let name = args.first().ok_or_else(|| "new requires a project name".to_owned())?;
    if args.len() > 1 { return Err("new accepts exactly one project name".into()); }
    let project = project::new_app(&PathBuf::from(name), name)?;
    println!("created {}", project.root.display());
    Ok(())
}

fn check_project(options: &Options, args: &[String]) -> Result<(), String> {
    let project = project_for(options, args)?;
    println!("project check: {} {}", project.id, project.version);
    Ok(())
}

fn edit_dependency(options: &Options, args: &[String], add: bool) -> Result<(), String> {
    let coordinate = args.first().ok_or_else(|| if add { "add requires COORDINATE@RANGE".to_owned() } else { "remove requires COORDINATE".to_owned() })?;
    if args.len() > 1 { return Err("dependency commands accept one coordinate".into()); }
    let (coordinate, version) = if add {
        coordinate.rsplit_once('@').ok_or_else(|| "add requires COORDINATE@RANGE".to_owned())?
    } else { (coordinate.as_str(), "") };
    let project = project_for(options, &[])?;
    project::set_dependency(&project, coordinate, if add { Some(version) } else { None })?;
    println!("{} {}", if add { "added" } else { "removed" }, coordinate);
    Ok(())
}

fn sync_project(options: &Options, args: &[String]) -> Result<(), String> {
    let project = project_for(options, &[])?;
    let flags: Vec<_> = args.iter().skip(1).collect();
    let mode = match flags.as_slice() {
        [] if options.offline => project::LockMode::Offline,
        [] => project::LockMode::Default,
        [flag] if (*flag).as_str() == "--offline" => project::LockMode::Offline,
        [flag] if (*flag).as_str() == "--locked" => project::LockMode::Locked,
        [flag] if (*flag).as_str() == "--frozen" => project::LockMode::Frozen,
        _ => return Err("sync accepts at most one of --offline, --locked, or --frozen".into()),
    };
    let lock = project::sync_lock(&project, mode)?;
    println!("project sync: {}", lock.display());
    Ok(())
}

fn run_project(options: &Options) -> Result<(), String> {
    let project = project_for(options, &[])?;
    let path = project::main_file(&project)?;
    let source = fs::read_to_string(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut runtime = Runtime::new();
    runtime.install_native_file_provider(project.root.to_string_lossy().as_ref());
    project::register_sources(&project, &mut runtime)?;
    if options.native_sockets { runtime.install_native_socket_provider(); }
    println!("{}", runtime.eval_native(&source)?);
    Ok(())
}

fn test_project(options: &Options, args: &[String]) -> Result<(), String> {
    let project = project_for(options, args)?;
    let files = project::files_in(&project.root, &project.test_paths)?;
    if files.is_empty() { return Err("project has no .hal files under :project/test-paths".into()); }
    let mut passed = 0usize;
    let mut failed = 0usize;
    for path in files {
        let source = fs::read_to_string(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut runtime = Runtime::new();
        runtime.install_native_file_provider(project.root.to_string_lossy().as_ref());
        project::register_sources(&project, &mut runtime)?;
        runtime.eval_native(include_str!("../../../../lib/src/std/lib/test.hal"))?;
        match test_results(&runtime.eval_native(&source)?) {
            Ok((file_passed, file_failed)) => { passed += file_passed; failed += file_failed; println!("test {}: {} passed, {} failed", path.display(), file_passed, file_failed); }
            Err(error) => { failed += 1; eprintln!("test {}: {error}", path.display()); }
        }
    }
    println!("test result: {passed} passed, {failed} failed");
    if failed == 0 { Ok(()) } else { Err("test failures".into()) }
}

fn test_results(value: &str) -> Result<(usize, usize), String> {
    let Form::String(source) = parse(value)? else { return Err("test file must finish with test/print-results".into()); };
    let Form::Vector(results) = parse(&source)? else { return Err("test/print-results must return a vector".into()); };
    let mut passed = 0;
    let mut failed = 0;
    for result in results {
        let Form::Map(entries) = result else { return Err("test result must be a map".into()); };
        let pass = entries.iter().find(|(key, _)| matches!(key, Form::Keyword(name) if name == "pass")).map(|(_, value)| value);
        match pass { Some(Form::Bool(true)) => passed += 1, Some(Form::Bool(false)) => failed += 1, _ => return Err("test result is missing boolean :pass".into()) }
    }
    Ok((passed, failed))
}

fn direct_eval(options: &Options, source: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("eval requires a Hara expression".into());
    }
    let mut runtime = Runtime::new();
    if let Some(root) = &options.root {
        runtime.install_native_file_provider(root.to_string_lossy().as_ref());
    }
    if options.native_sockets {
        runtime.install_native_socket_provider();
    }
    println!("{}", runtime.eval_native(source)?);
    Ok(())
}

fn run_file(options: &Options, path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let is_hir = path.ends_with(".hir") || bytes.starts_with(b"HIR\0");
    let mut runtime = Runtime::new();
    if let Some(root) = &options.root {
        runtime.install_native_file_provider(root.to_string_lossy().as_ref());
    }
    if options.native_sockets {
        runtime.install_native_socket_provider();
    }
    if is_hir {
        println!("{}", runtime.eval_hir(&bytes)?);
    } else {
        println!(
            "{}",
            runtime.eval_native(
                &String::from_utf8(bytes)
                    .map_err(|error| format!("{path} is not valid UTF-8: {error}"))?
            )?
        );
    }
    Ok(())
}

fn run_headless(options: &Options) -> Result<(), String> {
    if options.offline {
        return Err("--offline cannot be used with headless".into());
    }
    let broker = RuntimeBroker::start_with(options.root.clone(), options.native_sockets)?;
    let server = RespServer::start(&options.host, options.port, broker)?;
    println!("HARA RESP {} · session ROOT", server.endpoint());
    loop {
        std::thread::park();
    }
}

fn run_fabric(options: &Options) -> Result<(), String> {
    if options.offline {
        return Err("--offline cannot be used with fabric".into());
    }
    let service = FabricService::start(ServiceConfig {
        shards: options.shards.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
        }),
        data_dir: options.data_dir.clone(),
        node_id: options.node_id.clone(),
        peers: options.peers.clone(),
        cluster_epoch: options.cluster_epoch.clone(),
        ..ServiceConfig::default()
    })?;
    let server = FabricRespServer::start(&options.host, options.port, service)?;
    println!(
        "HARA FABRIC {} · node {} · space default · session ROOT",
        server.endpoint(),
        options.node_id
    );
    loop {
        std::thread::park();
    }
}

fn run_remote(endpoint: &str) -> Result<(), String> {
    let (host, port) = repl::parse_endpoint(endpoint, "127.0.0.1")?;
    let stream = TcpStream::connect((host.as_str(), port))
        .map_err(|error| format!("remote connect failed: {error}"))?;
    let mut connection = RespConnection::new(stream)?;
    connection.write(&RespValue::array(["HELLO", "4", "CLIENT", "HARA-REMOTE"]))?;
    println!(
        "{}",
        response_text(connection.read()?.ok_or("remote closed")?)
    );
    let mut request = 0_u64;
    for line in io::stdin().lock().lines() {
        let source = line.map_err(|error| format!("stdin: {error}"))?;
        if matches!(source.trim(), "/quit" | ":quit") {
            connection.write(&RespValue::array(["QUIT"]))?;
            break;
        }
        request += 1;
        let id = format!("REMOTE-{request}");
        connection.write(&RespValue::array(["EVAL", &id, source.trim()]))?;
        if let Some(value) = connection.read()? {
            println!("{}", response_text(value));
        }
        let _ = connection.read()?;
    }
    Ok(())
}

fn response_text(value: RespValue) -> String {
    match value {
        RespValue::Array(Some(values)) => values
            .into_iter()
            .map(response_text)
            .collect::<Vec<_>>()
            .join(" "),
        RespValue::Bulk(Some(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
        RespValue::Simple(value) | RespValue::Error(value) => value,
        RespValue::Integer(value) => value.to_string(),
        RespValue::Bulk(None) | RespValue::Array(None) => "nil".into(),
    }
}

fn usage() {
    println!("hara [OPTIONS] [new NAME|check [PATH]|add COORDINATE@RANGE|remove COORDINATE|sync|test [PATH]|repl|standalone|headless|server|fabric|remote HOST:PORT|eval SOURCE|run [FILE]|stdin]");
    println!("  --offline --host HOST --port PORT --root PATH --project PATH --allow-net");
    println!("  --data PATH --shards N --node-id NAME --peer NAME --cluster-epoch NAME");
    println!("  --history PATH --no-history --no-splash --no-color");
}

pub(crate) fn exit_error(message: &str, status: i32) -> ! {
    eprintln!("hara: {message}");
    std::process::exit(status)
}
