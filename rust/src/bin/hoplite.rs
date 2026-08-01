use hara_wasm::kernel::{parse, parse_forms, Form};
use hara_wasm::project::{self, Project};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

const NGINX_VERSION: &str = "1.30.4";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Route {
    path: String,
    handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Server {
    listen: u16,
    workers: usize,
}

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("hoplite: {error}");
        process::exit(1);
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    let command = arguments.first().map(String::as_str).unwrap_or("help");
    let root = arguments
        .get(1)
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(io)?);
    match command {
        "check" => {
            let project = check(&root)?;
            println!("{} is ready for Hoplite", project.id);
        }
        "build" => {
            let output = build(&root)?;
            println!("built {}", output.display());
        }
        "serve" => serve(&root)?,
        "status" => status(&root)?,
        "reload" => signal(&root, "reload")?,
        "stop" => signal(&root, "quit")?,
        "version" | "--version" | "-V" => {
            println!(
                "hoplite {} (Hara {}, Nginx {})",
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_VERSION"),
                NGINX_VERSION
            );
        }
        "help" | "--help" | "-h" => usage(),
        unknown => return Err(format!("unknown command {unknown:?}; run `hoplite help`")),
    }
    Ok(())
}

fn usage() {
    println!(
        "Hoplite {} — Hara on Nginx {}",
        env!("CARGO_PKG_VERSION"),
        NGINX_VERSION
    );
    println!("usage: hoplite <check|build|serve|status|reload|stop|version> [PROJECT]");
}

fn check(root: &Path) -> Result<Project, String> {
    let project = project::discover(root)?;
    let sources = source_files(&project)?;
    if sources.is_empty() {
        return Err("project has no .hal source files".into());
    }
    let source = bundle_sources(&sources)?;
    compile_application(&source)
        .map_err(|error| format!("Hoplite bytecode compilation failed: {error}"))?;
    load_configuration(&project)?;
    Ok(project)
}

fn build(root: &Path) -> Result<PathBuf, String> {
    let project = check(root)?;
    let sources = source_files(&project)?;
    let source = bundle_sources(&sources)?;
    let bytecode = compile_application(&source)
        .map_err(|error| format!("Hoplite bytecode compilation failed: {error}"))?;
    let (server, routes) = load_configuration(&project)?;
    let output = project.root.join(".hoplite");
    let configuration = output.join("conf");
    fs::create_dir_all(&configuration).map_err(io)?;
    fs::write(output.join("app.hal"), &source).map_err(io)?;
    fs::write(output.join("app.hbc"), bytecode).map_err(io)?;
    fs::write(
        configuration.join("nginx.conf"),
        nginx_configuration(&project, &server, &routes)?,
    )
    .map_err(io)?;
    Ok(output)
}

fn serve(root: &Path) -> Result<(), String> {
    let output = build(root)?;
    let project_root = output.parent().ok_or("invalid Hoplite output path")?;
    let exit = Command::new(nginx_binary())
        .arg("-p")
        .arg(project_root)
        .arg("-c")
        .arg(".hoplite/conf/nginx.conf")
        .arg("-e")
        .arg(".hoplite/error.log")
        .status()
        .map_err(|error| format!("cannot start Hoplite Nginx: {error}"))?;
    if !exit.success() {
        return Err(format!("Nginx exited with {exit}"));
    }
    status(project_root)
}

fn status(root: &Path) -> Result<(), String> {
    let project = project::discover(root)?;
    let pid_path = project.root.join(".hoplite/nginx.pid");
    let pid = fs::read_to_string(&pid_path)
        .map_err(|_| format!("Hoplite is stopped (no {})", pid_path.display()))?;
    let pid = pid.trim();
    let running = Command::new("kill")
        .args(["-0", pid])
        .status()
        .map_err(|error| format!("cannot inspect Hoplite process {pid}: {error}"))?
        .success();
    if !running {
        return Err(format!("Hoplite is stopped (stale pid {pid})"));
    }
    println!("Hoplite is running (pid {pid})");
    Ok(())
}

fn signal(root: &Path, signal: &str) -> Result<(), String> {
    let project = project::discover(root)?;
    let status = Command::new(nginx_binary())
        .arg("-p")
        .arg(&project.root)
        .arg("-c")
        .arg(".hoplite/conf/nginx.conf")
        .arg("-e")
        .arg(".hoplite/error.log")
        .arg("-s")
        .arg(signal)
        .status()
        .map_err(|error| format!("cannot signal Hoplite Nginx: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Nginx {signal} failed with {status}"))
    }
}

fn nginx_binary() -> PathBuf {
    if let Some(path) = env::var_os("HOPLITE_NGINX") {
        return path.into();
    }
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            let sibling = directory.join("nginx");
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/hoplite/nginx/sbin/nginx")
}

fn source_files(project: &Project) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for path in &project.source_paths {
        collect_hal(&project.root.join(path), &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_hal(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|value| value.to_str()) == Some("hal") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    let entries = fs::read_dir(path).map_err(io)?;
    for entry in entries {
        let entry = entry.map_err(io)?;
        let path = entry.path();
        if path.is_dir() {
            collect_hal(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("hal") {
            files.push(path);
        }
    }
    Ok(())
}

fn bundle_sources(files: &[PathBuf]) -> Result<String, String> {
    let mut source = String::new();
    for path in files {
        source.push_str(&format!(";; {}\n", path.display()));
        source.push_str(&fs::read_to_string(path).map_err(io)?);
        source.push_str("\n\n");
    }
    Ok(source)
}

fn compile_application(source: &str) -> Result<Vec<u8>, String> {
    let forms = parse_forms(source)?;
    let compilable = forms
        .into_iter()
        .filter(|form| {
            !matches!(form, Form::List(items) if matches!(items.first(), Some(Form::Symbol(operator)) if operator == "ns"))
        })
        .map(|form| render_form(&form))
        .collect::<Vec<_>>();
    let program = format!("(do {})", compilable.join("\n"));
    hara_wasm::compile_bytecode_artifact(&program)
}

fn render_form(form: &Form) -> String {
    match form {
        Form::Metadata(metadata, value) => {
            format!("^{} {}", render_form(metadata), render_form(value))
        }
        Form::Tagged(tag, value) => format!("#{tag}{}", render_form(value)),
        Form::List(values) => render_sequence(values, "(", ")"),
        Form::Vector(values) => render_sequence(values, "[", "]"),
        Form::Set(values) => render_sequence(values, "#{", "}"),
        Form::Map(entries) => {
            let values = entries
                .iter()
                .flat_map(|(key, value)| [render_form(key), render_form(value)])
                .collect::<Vec<_>>();
            format!("{{{}}}", values.join(" "))
        }
        _ => form.to_string(),
    }
}

fn render_sequence(values: &[Form], prefix: &str, suffix: &str) -> String {
    format!(
        "{prefix}{}{suffix}",
        values.iter().map(render_form).collect::<Vec<_>>().join(" ")
    )
}

fn load_configuration(project: &Project) -> Result<(Server, Vec<Route>), String> {
    let server_path = project.root.join("server.edn");
    let routes_path = project.root.join("routes.edn");
    let server = if server_path.is_file() {
        parse_server(&read_edn(&server_path)?)?
    } else {
        Server {
            listen: 8080,
            workers: 1,
        }
    };
    let routes = if routes_path.is_file() {
        parse_routes(&read_edn(&routes_path)?)?
    } else {
        let namespace = project
            .main
            .as_deref()
            .ok_or("Hoplite requires routes.edn or :project/main for its default / route")?;
        vec![Route {
            path: "/".into(),
            handler: format!("{namespace}/handler"),
        }]
    };
    if routes.is_empty() {
        return Err("routes.edn must declare at least one route".into());
    }
    Ok((server, routes))
}

fn read_edn(path: &Path) -> Result<Form, String> {
    let source = fs::read_to_string(path).map_err(io)?;
    parse(&source).map_err(|error| format!("{}: {error}", path.display()))
}

fn parse_server(form: &Form) -> Result<Server, String> {
    let entries = as_map(form, "server.edn")?;
    let listen = match lookup(entries, "hoplite/listen") {
        Some(Form::Number(value)) if (1..=65535).contains(value) => *value as u16,
        None => 8080,
        _ => return Err("server.edn :hoplite/listen must be a TCP port".into()),
    };
    let workers = match lookup(entries, "hoplite/workers") {
        Some(Form::Number(value)) if *value > 0 => *value as usize,
        None => 1,
        _ => return Err("server.edn :hoplite/workers must be a positive integer".into()),
    };
    Ok(Server { listen, workers })
}

fn parse_routes(form: &Form) -> Result<Vec<Route>, String> {
    let entries = as_map(form, "routes.edn")?;
    let forms = match lookup(entries, "hoplite/routes") {
        Some(Form::Vector(forms)) => forms,
        _ => return Err("routes.edn requires :hoplite/routes vector".into()),
    };
    forms
        .iter()
        .map(|form| {
            let route = as_map(form, "route")?;
            let path = text(lookup(route, "path"), "route :path")?;
            let handler = text(lookup(route, "handler"), "route :handler")?;
            if !path.starts_with('/') || unsafe_nginx(&path) {
                return Err(format!("invalid route path {path:?}"));
            }
            if unsafe_nginx(&handler) || handler.contains(char::is_whitespace) {
                return Err(format!("invalid route handler {handler:?}"));
            }
            Ok(Route { path, handler })
        })
        .collect()
}

fn nginx_configuration(
    project: &Project,
    server: &Server,
    routes: &[Route],
) -> Result<String, String> {
    let bootstrap = project
        .root
        .join(".hoplite/app.hal")
        .canonicalize()
        .unwrap_or_else(|_| project.root.join(".hoplite/app.hal"));
    let mut locations = String::new();
    for route in routes {
        locations.push_str(&format!(
            "        location {} {{\n            hoplite_content {};\n        }}\n",
            route.path, route.handler
        ));
    }
    Ok(format!(
        "worker_processes {};\npid .hoplite/nginx.pid;\nerror_log .hoplite/error.log;\nevents {{}}\nhttp {{\n    access_log .hoplite/access.log;\n    hoplite_bootstrap {};\n    server {{\n        listen {};\n{}    }}\n}}\n",
        server.workers,
        bootstrap.display(),
        server.listen,
        locations
    ))
}

fn as_map<'a>(form: &'a Form, label: &str) -> Result<&'a [(Form, Form)], String> {
    match form {
        Form::Map(entries) => Ok(entries),
        _ => Err(format!("{label} must be an EDN map")),
    }
}

fn lookup<'a>(entries: &'a [(Form, Form)], key: &str) -> Option<&'a Form> {
    entries.iter().find_map(|(candidate, value)| {
        matches!(candidate, Form::Keyword(name) if name == key).then_some(value)
    })
}

fn text(value: Option<&Form>, label: &str) -> Result<String, String> {
    match value {
        Some(Form::String(value) | Form::Symbol(value) | Form::Keyword(value)) => Ok(value.clone()),
        _ => Err(format!("{label} must be text or a symbol")),
    }
}

fn unsafe_nginx(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character, ';' | '{' | '}' | '\n' | '\r' | '\0'))
}

fn io(error: std::io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_server_and_routes() {
        let server =
            parse_server(&parse("{:hoplite/listen 9090 :hoplite/workers 3}").unwrap()).unwrap();
        assert_eq!(
            server,
            Server {
                listen: 9090,
                workers: 3
            }
        );
        let routes = parse_routes(
            &parse("{:hoplite/routes [{:path \"/hello\" :handler app/hello}]}").unwrap(),
        )
        .unwrap();
        assert_eq!(
            routes,
            vec![Route {
                path: "/hello".into(),
                handler: "app/hello".into()
            }]
        );
    }

    #[test]
    fn rejects_nginx_configuration_injection() {
        let error = parse_routes(
            &parse("{:hoplite/routes [{:path \"/; return 200\" :handler app/hello}]}").unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("invalid route path"));
    }
}
