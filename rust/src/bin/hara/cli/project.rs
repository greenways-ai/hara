use super::Options;
use crate::repl;
#[cfg(feature = "hir-encoder")]
use hara_wasm::kernel::{hir::encode_hir_module, parse_forms};
use hara_wasm::kernel::{parse, Form};
use hara_wasm::native_cli::RuntimeBroker;
use hara_wasm::project;
use hara_wasm::resp::{RespConnection, RespServer, RespValue};
use hara_wasm::Runtime;
use std::fs;
use std::io::{self, BufRead};
use std::net::TcpStream;
use std::path::PathBuf;

#[cfg(feature = "hir-encoder")]
pub(crate) fn compile_hir(args: &[String]) -> Result<(), String> {
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
    fs::write(output_path, artifact).map_err(|error| format!("cannot write {output_path}: {error}"))
}

fn project_for(options: &Options, args: &[String]) -> Result<project::Project, String> {
    let path = args
        .first()
        .map(PathBuf::from)
        .or_else(|| options.project.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    project::discover(&path)
}

pub(crate) fn new_project(args: &[String]) -> Result<(), String> {
    let name = args
        .first()
        .ok_or_else(|| "new requires a project name".to_owned())?;
    if args.len() > 1 {
        return Err("new accepts exactly one project name".into());
    }
    let project = project::new_app(&PathBuf::from(name), name)?;
    println!("created {}", project.root.display());
    Ok(())
}

pub(crate) fn check_project(options: &Options, args: &[String]) -> Result<(), String> {
    let project = project_for(options, args)?;
    println!("project check: {} {}", project.id, project.version);
    Ok(())
}

pub(crate) fn edit_dependency(options: &Options, args: &[String], add: bool) -> Result<(), String> {
    let coordinate = args.first().ok_or_else(|| {
        if add {
            "add requires COORDINATE@RANGE".to_owned()
        } else {
            "remove requires COORDINATE".to_owned()
        }
    })?;
    if args.len() > 1 {
        return Err("dependency commands accept one coordinate".into());
    }
    let (coordinate, version) = if add {
        coordinate
            .rsplit_once('@')
            .ok_or_else(|| "add requires COORDINATE@RANGE".to_owned())?
    } else {
        (coordinate.as_str(), "")
    };
    let project = project_for(options, &[])?;
    project::set_dependency(&project, coordinate, if add { Some(version) } else { None })?;
    println!("{} {}", if add { "added" } else { "removed" }, coordinate);
    Ok(())
}

pub(crate) fn sync_project(options: &Options, args: &[String]) -> Result<(), String> {
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

pub(crate) fn run_project(options: &Options) -> Result<(), String> {
    let project = project_for(options, &[])?;
    let path = project::main_file(&project)?;
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut runtime = Runtime::new();
    runtime.install_native_file_provider(project.root.to_string_lossy().as_ref());
    project::register_sources(&project, &mut runtime)?;
    if options.native_sockets {
        runtime.install_native_socket_provider();
    }
    println!("{}", runtime.eval_native(&source)?);
    Ok(())
}

pub(crate) fn test_project(options: &Options, args: &[String]) -> Result<(), String> {
    if args.len() > 1 {
        return Err("test accepts at most one path".into());
    }
    let project = project_for(options, args)?;
    let files = match args.first().map(PathBuf::from) {
        Some(path)
            if path.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("hal") =>
        {
            vec![path]
        }
        Some(path) if path.is_file() => return Err("test file must use the .hal extension".into()),
        Some(path) if path.is_dir() => project::files_in(&path, &[PathBuf::new()])?,
        Some(path) => return Err(format!("test path does not exist: {}", path.display())),
        None => project::files_in(&project.root, &project.test_paths)?,
    };
    if files.is_empty() {
        return Err("project has no .hal files under :project/test-paths".into());
    }
    let mut passed = 0usize;
    let mut failed = 0usize;
    for path in files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut runtime = Runtime::new();
        runtime.install_native_file_provider(project.root.to_string_lossy().as_ref());
        project::register_sources(&project, &mut runtime)?;
        runtime.eval_native(include_str!("../../../../../lib/src/std/lib/test.hal"))?;
        let evaluated = runtime.eval_native(&source)?;
        match test_results(&evaluated) {
            Ok((file_passed, file_failed)) => {
                passed += file_passed;
                failed += file_failed;
                println!(
                    "test {}: {} passed, {} failed",
                    path.display(),
                    file_passed,
                    file_failed
                );
                if file_failed > 0 {
                    eprintln!("{evaluated}");
                }
            }
            Err(error) => {
                failed += 1;
                eprintln!("test {}: {error}", path.display());
            }
        }
    }
    println!("test result: {passed} passed, {failed} failed");
    if failed == 0 {
        Ok(())
    } else {
        Err("test failures".into())
    }
}

fn test_results(value: &str) -> Result<(usize, usize), String> {
    let Form::String(source) = parse(value)? else {
        return Err("test file must finish with test/print-results".into());
    };
    let Form::Vector(results) = parse(&source)? else {
        return Err("test/print-results must return a vector".into());
    };
    let mut passed = 0;
    let mut failed = 0;
    for result in results {
        let Form::Map(entries) = result else {
            return Err("test result must be a map".into());
        };
        let pass = entries
            .iter()
            .find(|(key, _)| matches!(key, Form::Keyword(name) if name == "pass"))
            .map(|(_, value)| value);
        match pass {
            Some(Form::Bool(true)) => passed += 1,
            Some(Form::Bool(false)) => failed += 1,
            _ => return Err("test result is missing boolean :pass".into()),
        }
    }
    Ok((passed, failed))
}

pub(crate) fn direct_eval(options: &Options, source: &str) -> Result<(), String> {
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

pub(crate) fn run_file(options: &Options, path: &str) -> Result<(), String> {
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

pub(crate) fn run_headless(options: &Options) -> Result<(), String> {
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

pub(crate) fn run_remote(endpoint: &str) -> Result<(), String> {
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
