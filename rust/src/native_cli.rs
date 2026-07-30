#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use crate::Runtime;

enum Request {
    Eval {
        session: String,
        source: String,
        reply: mpsc::Sender<Result<String, String>>,
    },
    Namespace {
        session: String,
        reply: mpsc::Sender<Result<String, String>>,
    },
    Complete {
        session: String,
        prefix: String,
        reply: mpsc::Sender<Result<Vec<String>, String>>,
    },
    Create {
        session: String,
        reply: mpsc::Sender<Result<String, String>>,
    },
    Close {
        session: String,
        reply: mpsc::Sender<Result<String, String>>,
    },
    List {
        reply: mpsc::Sender<Result<Vec<String>, String>>,
    },
    Info {
        session: String,
        reply: mpsc::Sender<Result<String, String>>,
    },
    InstallModule {
        session: String,
        manifest: String,
        module: crate::wasmtime_provider::CompiledWasmModule,
        reply: mpsc::Sender<Result<String, String>>,
    },
    InvokeModule {
        session: String,
        namespace: String,
        export: String,
        arguments: Vec<u8>,
        reply: mpsc::Sender<Result<Vec<u8>, String>>,
    },
    Shutdown,
}

struct BrokerHandle {
    sender: mpsc::Sender<Request>,
}

impl Drop for BrokerHandle {
    fn drop(&mut self) {
        let _ = self.sender.send(Request::Shutdown);
    }
}

#[derive(Clone)]
pub struct RuntimeBroker {
    handle: Arc<BrokerHandle>,
}

impl RuntimeBroker {
    pub fn start() -> Result<Self, String> {
        Self::start_with(None, false)
    }

    pub fn start_with(root: Option<PathBuf>, native_sockets: bool) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("hara-runtime-broker".into())
            .spawn(move || run(receiver, root, native_sockets))
            .map_err(|error| format!("runtime broker failed: {error}"))?;
        Ok(Self {
            handle: Arc::new(BrokerHandle { sender }),
        })
    }

    pub fn eval(&self, session: &str, source: &str) -> Result<String, String> {
        self.call(|reply| Request::Eval {
            session: session.into(),
            source: source.into(),
            reply,
        })
    }

    pub fn namespace(&self, session: &str) -> Result<String, String> {
        self.call(|reply| Request::Namespace {
            session: session.into(),
            reply,
        })
    }

    pub fn complete(&self, session: &str, prefix: &str) -> Result<Vec<String>, String> {
        self.call(|reply| Request::Complete {
            session: session.into(),
            prefix: prefix.into(),
            reply,
        })
    }

    pub fn create(&self, session: &str) -> Result<String, String> {
        self.call(|reply| Request::Create {
            session: session.into(),
            reply,
        })
    }

    pub fn close(&self, session: &str) -> Result<String, String> {
        self.call(|reply| Request::Close {
            session: session.into(),
            reply,
        })
    }

    pub fn list(&self) -> Result<Vec<String>, String> {
        self.call(|reply| Request::List { reply })
    }

    pub fn info(&self, session: &str) -> Result<String, String> {
        self.call(|reply| Request::Info {
            session: session.into(),
            reply,
        })
    }

    pub fn install_module(
        &self,
        session: &str,
        manifest: &str,
        module: &crate::wasmtime_provider::CompiledWasmModule,
    ) -> Result<String, String> {
        self.call(|reply| Request::InstallModule {
            session: session.into(),
            manifest: manifest.into(),
            module: module.clone(),
            reply,
        })
    }

    pub fn invoke_module(
        &self,
        session: &str,
        namespace: &str,
        export: &str,
        arguments: &[u8],
    ) -> Result<Vec<u8>, String> {
        self.call(|reply| Request::InvokeModule {
            session: session.into(),
            namespace: namespace.into(),
            export: export.into(),
            arguments: arguments.into(),
            reply,
        })
    }

    fn call<T>(
        &self,
        request: impl FnOnce(mpsc::Sender<Result<T, String>>) -> Request,
    ) -> Result<T, String> {
        let (reply, response) = mpsc::channel();
        self.handle
            .sender
            .send(request(reply))
            .map_err(|_| "runtime broker is closed".to_owned())?;
        response
            .recv()
            .map_err(|_| "runtime broker stopped without a response".to_owned())?
    }
}

fn runtime(root: Option<&PathBuf>, native_sockets: bool) -> Runtime {
    let mut runtime = Runtime::new();
    if let Some(root) = root {
        runtime.install_native_file_provider(root.to_string_lossy().as_ref());
    }
    if native_sockets {
        runtime.install_native_socket_provider();
    }
    runtime
}

fn run(receiver: mpsc::Receiver<Request>, root: Option<PathBuf>, native_sockets: bool) {
    let mut sessions = HashMap::from([("ROOT".to_owned(), runtime(root.as_ref(), native_sockets))]);
    while let Ok(request) = receiver.recv() {
        match request {
            Request::Eval {
                session,
                source,
                reply,
            } => {
                let result = sessions
                    .get_mut(&session)
                    .ok_or_else(|| format!("No session: {session}"))
                    .and_then(|runtime| runtime.eval_native_traced(&source));
                let _ = reply.send(result);
            }
            Request::Namespace { session, reply } => {
                let result = sessions
                    .get(&session)
                    .map(Runtime::current_namespace)
                    .ok_or_else(|| format!("No session: {session}"));
                let _ = reply.send(result);
            }
            Request::Complete {
                session,
                prefix,
                reply,
            } => {
                let result = sessions
                    .get(&session)
                    .map(|runtime| {
                        let mut symbols = runtime
                            .visible_symbols()
                            .into_iter()
                            .filter(|symbol| symbol.starts_with(&prefix))
                            .collect::<Vec<_>>();
                        symbols.sort();
                        symbols.dedup();
                        symbols
                    })
                    .ok_or_else(|| format!("No session: {session}"));
                let _ = reply.send(result);
            }
            Request::Create { session, reply } => {
                let result = if session.is_empty() || sessions.contains_key(&session) {
                    Err(format!("Session already exists or is invalid: {session}"))
                } else {
                    sessions.insert(session.clone(), runtime(root.as_ref(), native_sockets));
                    Ok(session)
                };
                let _ = reply.send(result);
            }
            Request::Close { session, reply } => {
                let result = if session == "ROOT" {
                    Err("ROOT cannot be closed".into())
                } else if sessions.remove(&session).is_some() {
                    Ok(session)
                } else {
                    Err(format!("No session: {session}"))
                };
                let _ = reply.send(result);
            }
            Request::List { reply } => {
                let mut names = sessions.keys().cloned().collect::<Vec<_>>();
                names.sort();
                let _ = reply.send(Ok(names));
            }
            Request::Info { session, reply } => {
                let result = sessions
                    .get(&session)
                    .map(|runtime| format!("{session} {}", runtime.current_namespace()))
                    .ok_or_else(|| format!("No session: {session}"));
                let _ = reply.send(result);
            }
            Request::InstallModule {
                session,
                manifest,
                module,
                reply,
            } => {
                let result = sessions
                    .get_mut(&session)
                    .ok_or_else(|| format!("No session: {session}"))
                    .and_then(|runtime| {
                        let provider = module.provider();
                        let parsed =
                            crate::extension::ExtensionManifest::parse(&manifest, "MODULE PUT")?;
                        let namespace = parsed.namespace.clone();
                        runtime.install_wasm_extension(&manifest, "MODULE PUT", provider)?;
                        Ok(namespace)
                    });
                let _ = reply.send(result);
            }
            Request::InvokeModule {
                session,
                namespace,
                export,
                arguments,
                reply,
            } => {
                let result = sessions
                    .get_mut(&session)
                    .ok_or_else(|| format!("No session: {session}"))
                    .and_then(|runtime| {
                        let arguments = crate::hta::decode(&arguments)?;
                        let arguments: Vec<crate::extension::Value> = match arguments {
                            crate::extension::Value::Vector(values) => {
                                values.iter().cloned().collect()
                            }
                            crate::extension::Value::Tuple(values) => {
                                values.iter().cloned().collect()
                            }
                            other => {
                                return Err(format!(
                                    "hta/arguments: expected vector, got {}",
                                    other.display()
                                ))
                            }
                        };
                        let result =
                            runtime.invoke_wasm_extension(&namespace, &export, &arguments)?;
                        crate::hta::encode(&result)
                    });
                let _ = reply.send(result);
            }
            Request::Shutdown => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeBroker;

    #[test]
    fn sessions_are_isolated_and_root_is_persistent() {
        let broker = RuntimeBroker::start().unwrap();
        assert_eq!(broker.eval("ROOT", "(def answer 42)").unwrap(), "42");
        broker.create("APP").unwrap();
        assert!(broker
            .eval("APP", "answer")
            .unwrap_err()
            .contains("unbound"));
        assert_eq!(broker.eval("ROOT", "answer").unwrap(), "42");
        assert_eq!(broker.list().unwrap(), vec!["APP", "ROOT"]);
        broker.close("APP").unwrap();
        assert!(broker.close("ROOT").is_err());
    }
}
