use hara_wasm::kernel::{parse, parse_forms, Form};
use hara_wasm::project::{self, Project};
use std::env;
use std::fs;
#[cfg(all(unix, feature = "hoplite-embedded-nginx"))]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::thread;
use std::time::Duration;

#[path = "hara/cli.rs"]
mod cli;
#[path = "hara/repl.rs"]
mod repl;
#[path = "hara/terminal.rs"]
mod terminal;

const NGINX_VERSION: &str = "1.30.4";

#[cfg(feature = "hoplite-embedded-nginx")]
const EMBEDDED_NGINX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/hoplite/nginx/sbin/nginx"
));

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
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if matches!(
        arguments.first().map(String::as_str),
        Some("serve" | "version" | "--version" | "-V")
    ) {
        if let Err(error) = run_hoplite(arguments) {
            eprintln!("hoplite: {error}");
            process::exit(1);
        }
        return;
    }
    let options = match cli::parse_options() {
        Ok(options) => options,
        Err(error) => cli::exit_error(&error, 2),
    };
    if let Err(error) = cli::run(options) {
        cli::exit_error(&error, cli::error_exit_code(&error));
    }
}

fn run_hoplite(arguments: Vec<String>) -> Result<(), String> {
    match arguments.first().map(String::as_str) {
        Some("version" | "--version" | "-V") => {
            println!("Hoplite {}", env!("CARGO_PKG_VERSION"));
            println!("Hara {}", env!("CARGO_PKG_VERSION"));
            println!("Nginx {} ({})", NGINX_VERSION, nginx_distribution());
        }
        Some("serve") => run_serve_command(&arguments[1..])?,
        _ => return Err("unknown Hoplite command".into()),
    }
    Ok(())
}

fn run_serve_command(arguments: &[String]) -> Result<(), String> {
    let (action, project) = match arguments.first().map(String::as_str) {
        None => ("start", None),
        Some("--help" | "-h" | "help") => {
            serve_usage();
            return Ok(());
        }
        Some(
            action @ ("start" | "foreground" | "install" | "uninstall" | "status" | "reload"
            | "stop" | "build" | "check"),
        ) => (action, arguments.get(1)),
        Some(_) => ("start", arguments.first()),
    };
    let root = project
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(io)?);
    match action {
        "start" => serve(&root),
        "foreground" => run_foreground(&root),
        "install" => launchd_install(&root),
        "uninstall" => launchd_uninstall(&root),
        "status" => status(&root),
        "reload" => signal(&root, "reload"),
        "stop" => signal(&root, "quit"),
        "build" => build(&root).map(|output| println!("built {}", output.display())),
        "check" => check(&root).map(|project| println!("{} is ready for Hoplite", project.id)),
        _ => unreachable!(),
    }
}

fn serve_usage() {
    println!(
        "Hoplite {} — Hara on Nginx {}",
        env!("CARGO_PKG_VERSION"),
        NGINX_VERSION
    );
    println!("usage: hoplite serve [PROJECT]");
    println!("       hoplite serve <foreground|install|uninstall|status|reload|stop|build|check> [PROJECT]");
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
    let exit = Command::new(nginx_binary()?)
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
    wait_for_status(project_root)
}

fn run_foreground(root: &Path) -> Result<(), String> {
    let output = build(root)?;
    let project_root = output.parent().ok_or("invalid Hoplite output path")?;
    let mut command = Command::new(nginx_binary()?);
    command
        .arg("-p")
        .arg(project_root)
        .arg("-c")
        .arg(".hoplite/conf/nginx.conf")
        .arg("-e")
        .arg(".hoplite/error.log")
        .args(["-g", "daemon off;"]);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        Err(format!("cannot exec Hoplite Nginx: {error}"))
    }
    #[cfg(not(unix))]
    {
        let exit = command
            .status()
            .map_err(|error| format!("cannot run Hoplite Nginx: {error}"))?;
        if exit.success() {
            Ok(())
        } else {
            Err(format!("Nginx exited with {exit}"))
        }
    }
}

#[cfg(target_os = "macos")]
fn launchd_install(root: &Path) -> Result<(), String> {
    let output = build(root)?;
    let project_root = output.parent().ok_or("invalid Hoplite output path")?;
    let project = project::discover(project_root)?;
    let executable = env::current_exe().map_err(io)?.canonicalize().map_err(io)?;
    let label = launchd_label(&project.id);
    let home = env::var_os("HOME").ok_or("HOME is not set")?;
    let agents = PathBuf::from(home).join("Library/LaunchAgents");
    fs::create_dir_all(&agents).map_err(io)?;
    let plist = agents.join(format!("{label}.plist"));
    fs::write(&plist, launchd_plist(&label, &executable, project_root)).map_err(io)?;

    let uid = command_output("id", &["-u"])?;
    let domain = format!("gui/{uid}");
    let service = format!("{domain}/{label}");
    if Command::new("launchctl")
        .args(["print", &service])
        .output()
        .map_err(io)?
        .status
        .success()
    {
        let _ = Command::new("launchctl")
            .args(["bootout", &service])
            .status();
    }
    let status = Command::new("launchctl")
        .arg("bootstrap")
        .arg(&domain)
        .arg(&plist)
        .status()
        .map_err(io)?;
    if !status.success() {
        return Err(format!("launchctl bootstrap failed with {status}"));
    }
    println!("installed {label}");
    println!("launchd plist: {}", plist.display());
    wait_for_status(project_root)
}

#[cfg(not(target_os = "macos"))]
fn launchd_install(_root: &Path) -> Result<(), String> {
    Err("hoplite install requires macOS launchd".into())
}

#[cfg(target_os = "macos")]
fn launchd_uninstall(root: &Path) -> Result<(), String> {
    let project = project::discover(root)?;
    let label = launchd_label(&project.id);
    let uid = command_output("id", &["-u"])?;
    let service = format!("gui/{uid}/{label}");
    let status = Command::new("launchctl")
        .args(["bootout", &service])
        .status()
        .map_err(io)?;
    if !status.success() {
        return Err(format!("launchctl bootout failed with {status}"));
    }
    if let Some(home) = env::var_os("HOME") {
        let plist = PathBuf::from(home)
            .join("Library/LaunchAgents")
            .join(format!("{label}.plist"));
        if plist.exists() {
            fs::remove_file(&plist).map_err(io)?;
        }
    }
    println!("uninstalled {label}");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn launchd_uninstall(_root: &Path) -> Result<(), String> {
    Err("hoplite uninstall requires macOS launchd".into())
}

fn launchd_label(project_id: &str) -> String {
    let suffix = project_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("org.hara.hoplite.{suffix}")
}

fn launchd_plist(label: &str, executable: &Path, project_root: &Path) -> String {
    let output = project_root.join(".hoplite/launchd.stdout.log");
    let error = project_root.join(".hoplite/launchd.stderr.log");
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
  <key>Label</key><string>{}</string>\n\
  <key>ProgramArguments</key>\n\
  <array>\n\
    <string>{}</string>\n\
    <string>serve</string>\n\
    <string>foreground</string>\n\
    <string>{}</string>\n\
  </array>\n\
  <key>WorkingDirectory</key><string>{}</string>\n\
  <key>RunAtLoad</key><true/>\n\
  <key>KeepAlive</key><true/>\n\
  <key>ProcessType</key><string>Background</string>\n\
  <key>StandardOutPath</key><string>{}</string>\n\
  <key>StandardErrorPath</key><string>{}</string>\n\
</dict>\n\
</plist>\n",
        xml_escape(label),
        xml_escape(&executable.display().to_string()),
        xml_escape(&project_root.display().to_string()),
        xml_escape(&project_root.display().to_string()),
        xml_escape(&output.display().to_string()),
        xml_escape(&error.display().to_string())
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "macos")]
fn command_output(command: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(command).args(arguments).output().map_err(io)?;
    if !output.status.success() {
        return Err(format!("{command} exited with {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| error.to_string())
}

fn wait_for_status(root: &Path) -> Result<(), String> {
    let mut last_error = None;
    for _ in 0..200 {
        match status(root) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(last_error.unwrap_or_else(|| "Hoplite failed to start".into()))
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
    let status = Command::new(nginx_binary()?)
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

fn nginx_distribution() -> &'static str {
    if cfg!(feature = "hoplite-embedded-nginx") {
        "embedded"
    } else {
        "external"
    }
}

fn nginx_binary() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("HOPLITE_NGINX") {
        return Ok(path.into());
    }
    #[cfg(feature = "hoplite-embedded-nginx")]
    return materialize_embedded_nginx();

    #[cfg(not(feature = "hoplite-embedded-nginx"))]
    {
        if let Ok(executable) = env::current_exe() {
            if let Some(directory) = executable.parent() {
                let sibling = directory.join("nginx");
                if sibling.is_file() {
                    return Ok(sibling);
                }
            }
        }
        Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join("target/hoplite/nginx/sbin/nginx"))
    }
}

#[cfg(feature = "hoplite-embedded-nginx")]
fn materialize_embedded_nginx() -> Result<PathBuf, String> {
    let cache = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Caches")))
        .unwrap_or_else(env::temp_dir)
        .join("hoplite")
        .join(env!("CARGO_PKG_VERSION"));
    let path = cache.join(format!("nginx-{NGINX_VERSION}"));
    if fs::read(&path)
        .map(|contents| contents == EMBEDDED_NGINX)
        .unwrap_or(false)
    {
        return Ok(path);
    }
    fs::create_dir_all(&cache).map_err(io)?;
    let temporary = cache.join(format!(".nginx-{}.tmp", process::id()));
    fs::write(&temporary, EMBEDDED_NGINX).map_err(io)?;
    #[cfg(unix)]
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o700)).map_err(io)?;
    fs::rename(&temporary, &path).map_err(io)?;
    Ok(path)
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

    #[test]
    fn creates_safe_launchd_definition() {
        let label = launchd_label("example/web api");
        assert_eq!(label, "org.hara.hoplite.example-web-api");
        let plist = launchd_plist(
            &label,
            Path::new("/Applications/Hara & Hoplite/hoplite"),
            Path::new("/tmp/example <web>"),
        );
        assert!(plist.contains("<string>serve</string>"));
        assert!(plist.contains("<string>foreground</string>"));
        assert!(plist.contains("Hara &amp; Hoplite"));
        assert!(plist.contains("example &lt;web&gt;"));
        assert!(plist.contains("<key>KeepAlive</key><true/>"));
    }
}
