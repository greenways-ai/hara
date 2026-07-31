//! Data-defined command routing shared by native Hara entrypoints.
//!
//! Route metadata is embedded from the normative Hara CLI EDN document.  The
//! document contains stable handler IDs only; executable handlers remain in a
//! closed registry owned by each runtime.

use crate::kernel::{parse, Form};
use std::sync::OnceLock;

pub const MANIFEST_SOURCE: &str =
    include_str!("../../specs/02-platform/000001-cli/draft/hara-cli.edn");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Execution {
    Finite,
    Interactive,
    Stream,
    Daemon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tier {
    Public,
    Developer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDescriptor {
    pub id: String,
    pub path: Vec<String>,
    pub aliases: Vec<Vec<String>>,
    pub handler: String,
    pub execution: Execution,
    pub tier: Tier,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoute {
    pub route: RouteDescriptor,
    pub arguments: Vec<String>,
    pub alias: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliOutcome {
    Success,
    Failed,
    UsageError,
    ReadError,
    ResolutionError,
    Unavailable,
    InternalError,
    Interrupted,
}

impl CliOutcome {
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Failed => 1,
            Self::UsageError
            | Self::ReadError
            | Self::ResolutionError
            | Self::Unavailable
            | Self::InternalError => 2,
            Self::Interrupted => 130,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliRequest {
    pub route: String,
    pub arguments: Vec<String>,
    pub runtime: String,
    pub cwd: String,
    pub project: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliResult {
    pub outcome: CliOutcome,
    pub data: Option<String>,
    pub messages: Vec<String>,
}

impl CliResult {
    pub fn success(data: Option<String>) -> Self {
        Self {
            outcome: CliOutcome::Success,
            data,
            messages: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct CliRouter {
    routes: Vec<RouteDescriptor>,
}

impl CliRouter {
    pub fn from_edn(source: &str) -> Result<Self, String> {
        let document = parse(source)?;
        let routes = map_get(&document, "cli/routes")
            .and_then(vector)
            .ok_or("CLI manifest requires :cli/routes")?
            .iter()
            .map(parse_route)
            .collect::<Result<Vec<_>, _>>()?;
        let router = Self { routes };
        router.verify()?;
        Ok(router)
    }

    pub fn routes(&self) -> &[RouteDescriptor] {
        &self.routes
    }

    pub fn public_routes(&self) -> impl Iterator<Item = &RouteDescriptor> {
        self.routes
            .iter()
            .filter(|route| route.tier == Tier::Public)
    }

    pub fn resolve(&self, argv: &[String]) -> Option<ResolvedRoute> {
        if argv.is_empty() {
            return self.resolve_id("hara.cli.route/repl", Vec::new(), false);
        }
        if argv == ["run"] {
            return self.resolve_id("hara.cli.route/project-run", Vec::new(), true);
        }
        let mut matches = self
            .routes
            .iter()
            .flat_map(|route| {
                std::iter::once((&route.path, route, false))
                    .chain(route.aliases.iter().map(move |alias| (alias, route, true)))
            })
            .filter(|(path, _, _)| argv.starts_with(path))
            .collect::<Vec<_>>();
        matches.sort_by_key(|(path, _, alias)| (std::cmp::Reverse(path.len()), *alias));
        let (path, route, alias) = matches.first()?;
        let mut arguments = argv[path.len()..].to_vec();
        if arguments.first().is_some_and(|argument| argument == "--") {
            arguments.remove(0);
        }
        Some(ResolvedRoute {
            route: (*route).clone(),
            arguments,
            alias: *alias,
        })
    }

    fn resolve_id(&self, id: &str, arguments: Vec<String>, alias: bool) -> Option<ResolvedRoute> {
        self.routes
            .iter()
            .find(|route| route.id == id)
            .cloned()
            .map(|route| ResolvedRoute {
                route,
                arguments,
                alias,
            })
    }

    fn verify(&self) -> Result<(), String> {
        let mut paths = std::collections::BTreeMap::<Vec<String>, String>::new();
        for route in &self.routes {
            if route.id.is_empty() || !route.id.contains('/') {
                return Err(format!("CLI route has invalid stable ID: {}", route.id));
            }
            for path in std::iter::once(&route.path).chain(route.aliases.iter()) {
                if path.is_empty() || path.iter().any(String::is_empty) {
                    return Err(format!("CLI route {} has an empty path", route.id));
                }
                if let Some(existing) = paths.insert(path.clone(), route.id.clone()) {
                    return Err(format!(
                        "ambiguous CLI route {}: {} and {}",
                        path.join(" "),
                        existing,
                        route.id
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn router() -> &'static CliRouter {
    static ROUTER: OnceLock<CliRouter> = OnceLock::new();
    ROUTER.get_or_init(|| {
        CliRouter::from_edn(MANIFEST_SOURCE).expect("embedded Hara CLI manifest must be valid")
    })
}

fn parse_route(form: &Form) -> Result<RouteDescriptor, String> {
    Ok(RouteDescriptor {
        id: keyword_field(form, "route/id")?,
        path: string_vector_field(form, "route/path")?,
        aliases: map_get(form, "route/aliases")
            .and_then(vector)
            .ok_or("CLI route requires :route/aliases")?
            .iter()
            .map(|alias| {
                vector(alias)
                    .ok_or_else(|| "CLI alias must be a vector".to_owned())?
                    .iter()
                    .map(|segment| match segment {
                        Form::String(value) => Ok(value.clone()),
                        _ => Err("CLI alias segments must be strings".to_owned()),
                    })
                    .collect()
            })
            .collect::<Result<Vec<_>, _>>()?,
        handler: keyword_field(form, "route/handler")?,
        execution: match keyword_field(form, "route/execution")?.as_str() {
            "finite" => Execution::Finite,
            "interactive" => Execution::Interactive,
            "stream" => Execution::Stream,
            "daemon" => Execution::Daemon,
            value => return Err(format!("unknown CLI execution mode: {value}")),
        },
        tier: match keyword_field(form, "route/tier")?.as_str() {
            "public" => Tier::Public,
            "developer" => Tier::Developer,
            value => return Err(format!("unknown CLI route tier: {value}")),
        },
        summary: string_field(form, "route/summary")?,
    })
}

fn map_get<'a>(form: &'a Form, key: &str) -> Option<&'a Form> {
    let Form::Map(entries) = form else {
        return None;
    };
    entries.iter().find_map(|(candidate, value)| {
        matches!(candidate, Form::Keyword(name) if name == key).then_some(value)
    })
}

fn vector(form: &Form) -> Option<&[Form]> {
    match form {
        Form::Vector(values) => Some(values),
        _ => None,
    }
}

fn keyword_field(form: &Form, key: &str) -> Result<String, String> {
    match map_get(form, key) {
        Some(Form::Keyword(value)) => Ok(value.clone()),
        _ => Err(format!("CLI route requires keyword :{key}")),
    }
}

fn string_field(form: &Form, key: &str) -> Result<String, String> {
    match map_get(form, key) {
        Some(Form::String(value)) => Ok(value.clone()),
        _ => Err(format!("CLI route requires string :{key}")),
    }
}

fn string_vector_field(form: &Form, key: &str) -> Result<Vec<String>, String> {
    vector(map_get(form, key).ok_or_else(|| format!("CLI route requires :{key}"))?)
        .ok_or_else(|| format!("CLI route :{key} must be a vector"))?
        .iter()
        .map(|value| match value {
            Form::String(value) => Ok(value.clone()),
            _ => Err(format!("CLI route :{key} must contain strings")),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{map_get, parse, router, vector, CliOutcome, Form, MANIFEST_SOURCE};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn repo_text(relative: &str) -> Option<String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(relative);
        match std::fs::read_to_string(&path) {
            Ok(content) => Some(content),
            Err(_) => {
                eprintln!(
                    "skipping: {} is unavailable (specs submodule not initialized)",
                    path.display()
                );
                None
            }
        }
    }

    #[test]
    fn vendored_manifest_matches_specs_submodule_when_present() {
        let Some(submodule) = repo_text("specs/00-unsorted/cli/draft/hara-cli.edn") else {
            return;
        };
        assert_eq!(
            submodule, MANIFEST_SOURCE,
            "rust/resources/hara-cli.edn is stale; refresh it from specs/00-unsorted/cli/draft/hara-cli.edn"
        );
    }

    #[test]
    fn embedded_manifest_routes_canonical_and_alias_paths() {
        let canonical = router()
            .resolve(&args(&["project", "check", "demo"]))
            .unwrap();
        let alias = router().resolve(&args(&["check", "demo"])).unwrap();
        assert_eq!(canonical.route.id, "hara.cli.route/project-check");
        assert_eq!(canonical.arguments, ["demo"]);
        assert!(!canonical.alias);
        assert_eq!(alias.route.id, canonical.route.id);
        assert!(alias.alias);
    }

    #[test]
    fn every_public_manifest_route_resolves_to_its_stable_id() {
        for route in router().public_routes() {
            let mut argv = route.path.clone();
            if route.id == "hara.cli.route/run-file" {
                argv.push("fixture.hal".into());
            }
            let resolved = router()
                .resolve(&argv)
                .unwrap_or_else(|| panic!("unresolved public route {}", route.id));
            assert_eq!(route.id, resolved.route.id);
        }
    }

    #[test]
    fn run_compatibility_is_unambiguous() {
        assert_eq!(
            router().resolve(&args(&["run"])).unwrap().route.id,
            "hara.cli.route/project-run"
        );
        assert_eq!(
            router()
                .resolve(&args(&["run", "main.hal"]))
                .unwrap()
                .route
                .id,
            "hara.cli.route/run-file"
        );
    }

    #[test]
    fn option_terminator_is_not_a_route_argument() {
        assert_eq!(
            router()
                .resolve(&args(&["eval", "--", "(- 2 1)"]))
                .unwrap()
                .arguments,
            ["(- 2 1)"]
        );
    }

    #[test]
    fn public_outcomes_have_stable_exit_codes() {
        assert_eq!(CliOutcome::Success.exit_code(), 0);
        assert_eq!(CliOutcome::Failed.exit_code(), 1);
        assert_eq!(CliOutcome::ReadError.exit_code(), 2);
        assert_eq!(CliOutcome::Interrupted.exit_code(), 130);
    }

    #[test]
    fn shared_outcome_conformance_cases_pass() {
        let document = parse(include_str!(
            "../../specs/02-platform/000001-cli/draft/conformance/outcomes.edn"
        ))
        .unwrap();
        for case in vector(map_get(&document, "conformance/cases").unwrap()).unwrap() {
            let Some(Form::Keyword(input)) = map_get(case, "case/input") else {
                continue;
            };
            let outcome = match input.as_str() {
                "hara.cli.outcome/success" => CliOutcome::Success,
                "hara.cli.outcome/failed" => CliOutcome::Failed,
                "hara.cli.outcome/read-error" => CliOutcome::ReadError,
                "hara.cli.outcome/usage-error" => CliOutcome::UsageError,
                "hara.cli.outcome/interrupted" => CliOutcome::Interrupted,
                other => panic!("unknown conformance outcome: {other}"),
            };
            let Form::Number(expected) =
                map_get(case, "case/expected-exit").expect("expected exit")
            else {
                panic!("expected exit must be an integer");
            };
            assert_eq!(outcome.exit_code(), *expected as i32);
        }
    }

    #[test]
    fn shared_route_conformance_cases_pass() {
        let document = parse(include_str!(
            "../../specs/02-platform/000001-cli/draft/conformance/routes.edn"
        ))
        .unwrap();
        for case in vector(map_get(&document, "conformance/cases").unwrap()).unwrap() {
            let id = match map_get(case, "case/id").unwrap() {
                Form::Keyword(value) => value,
                _ => panic!("case ID must be a keyword"),
            };
            let argv = vector(map_get(case, "case/argv").unwrap())
                .unwrap()
                .iter()
                .map(|value| match value {
                    Form::String(value) => value.clone(),
                    _ => panic!("{id}: argv must contain strings"),
                })
                .collect::<Vec<_>>();
            let expected = map_get(case, "case/expected").unwrap();
            if let Some(Form::Keyword(route_id)) = map_get(expected, "route/id") {
                let resolved = router().resolve(&argv).unwrap_or_else(|| panic!("{id}"));
                assert_eq!(&resolved.route.id, route_id, "{id}");
                if let Some(Form::Vector(arguments)) = map_get(expected, "route/arguments") {
                    let arguments = arguments
                        .iter()
                        .map(|value| match value {
                            Form::String(value) => value.clone(),
                            _ => panic!("{id}: expected arguments must be strings"),
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(resolved.arguments, arguments, "{id}");
                }
            } else {
                assert!(router().resolve(&argv).is_none(), "{id}");
            }
        }
    }
}
