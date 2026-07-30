#![cfg(not(target_arch = "wasm32"))]

use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::native_cli::RuntimeBroker;
use crate::service::FabricService;

const MAX_LINE: usize = 64 * 1024;
const MAX_BULK: usize = 64 * 1024 * 1024;
const MAX_NESTING: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RespValue {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
    Array(Option<Vec<RespValue>>),
}

impl RespValue {
    pub fn text(&self) -> Option<String> {
        match self {
            Self::Simple(value) | Self::Error(value) => Some(value.clone()),
            Self::Integer(value) => Some(value.to_string()),
            Self::Bulk(Some(value)) => String::from_utf8(value.clone()).ok(),
            _ => None,
        }
    }

    pub fn bulk(value: impl Into<String>) -> Self {
        Self::Bulk(Some(value.into().into_bytes()))
    }

    pub fn array(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Array(Some(
            values
                .into_iter()
                .map(|value| Self::bulk(value.into()))
                .collect(),
        ))
    }
}

pub struct RespConnection {
    input: BufReader<TcpStream>,
    output: BufWriter<TcpStream>,
}

impl RespConnection {
    pub fn new(stream: TcpStream) -> Result<Self, String> {
        let output = stream
            .try_clone()
            .map(BufWriter::new)
            .map_err(|error| format!("RESP socket clone failed: {error}"))?;
        Ok(Self {
            input: BufReader::new(stream),
            output,
        })
    }

    pub fn read(&mut self) -> Result<Option<RespValue>, String> {
        let mut prefix = [0_u8; 1];
        match self.input.read_exact(&mut prefix) {
            Ok(()) => self.read_after_prefix(prefix[0], 0).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(error) => Err(format!("RESP read failed: {error}")),
        }
    }

    fn read_after_prefix(&mut self, prefix: u8, depth: usize) -> Result<RespValue, String> {
        if depth > MAX_NESTING {
            return Err("RESP nesting limit exceeded".into());
        }
        match prefix {
            b'+' => Ok(RespValue::Simple(self.line()?)),
            b'-' => Ok(RespValue::Error(self.line()?)),
            b':' => self
                .line()?
                .parse::<i64>()
                .map(RespValue::Integer)
                .map_err(|_| "Invalid RESP integer".into()),
            b'$' => {
                let length = self.length()?;
                if length < 0 {
                    return Ok(RespValue::Bulk(None));
                }
                let length = usize::try_from(length).map_err(|_| "Invalid RESP length")?;
                if length > MAX_BULK {
                    return Err("RESP bulk limit exceeded".into());
                }
                let mut bytes = vec![0; length];
                self.input
                    .read_exact(&mut bytes)
                    .map_err(|error| format!("RESP read failed: {error}"))?;
                self.crlf()?;
                Ok(RespValue::Bulk(Some(bytes)))
            }
            b'*' => {
                let length = self.length()?;
                if length < 0 {
                    return Ok(RespValue::Array(None));
                }
                let length = usize::try_from(length).map_err(|_| "Invalid RESP length")?;
                if length > MAX_LINE {
                    return Err("RESP array limit exceeded".into());
                }
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    let mut prefix = [0_u8; 1];
                    self.input
                        .read_exact(&mut prefix)
                        .map_err(|error| format!("RESP read failed: {error}"))?;
                    values.push(self.read_after_prefix(prefix[0], depth + 1)?);
                }
                Ok(RespValue::Array(Some(values)))
            }
            _ => Err("Unknown RESP type".into()),
        }
    }

    fn length(&mut self) -> Result<i64, String> {
        self.line()?
            .parse()
            .map_err(|_| "Invalid RESP length".into())
    }

    fn line(&mut self) -> Result<String, String> {
        let mut bytes = Vec::new();
        let read = self
            .input
            .read_until(b'\n', &mut bytes)
            .map_err(|error| format!("RESP read failed: {error}"))?;
        if read < 2 || bytes[read - 2..] != *b"\r\n" {
            return Err("Invalid RESP line ending".into());
        }
        if bytes.len() > MAX_LINE {
            return Err("RESP line limit exceeded".into());
        }
        bytes.truncate(read - 2);
        String::from_utf8(bytes).map_err(|_| "RESP line is not UTF-8".into())
    }

    fn crlf(&mut self) -> Result<(), String> {
        let mut ending = [0_u8; 2];
        self.input
            .read_exact(&mut ending)
            .map_err(|error| format!("RESP read failed: {error}"))?;
        if ending != *b"\r\n" {
            return Err("Invalid RESP bulk ending".into());
        }
        Ok(())
    }

    pub fn write(&mut self, value: &RespValue) -> Result<(), String> {
        write_value(&mut self.output, value)?;
        self.output
            .flush()
            .map_err(|error| format!("RESP write failed: {error}"))
    }
}

fn write_value(output: &mut impl Write, value: &RespValue) -> Result<(), String> {
    match value {
        RespValue::Simple(value) => line_value(output, b'+', value),
        RespValue::Error(value) => line_value(output, b'-', value),
        RespValue::Integer(value) => line_value(output, b':', &value.to_string()),
        RespValue::Bulk(None) => output
            .write_all(b"$-1\r\n")
            .map_err(|error| format!("RESP write failed: {error}")),
        RespValue::Bulk(Some(bytes)) => output
            .write_all(format!("${}\r\n", bytes.len()).as_bytes())
            .and_then(|_| output.write_all(bytes))
            .and_then(|_| output.write_all(b"\r\n"))
            .map_err(|error| format!("RESP write failed: {error}")),
        RespValue::Array(None) => output
            .write_all(b"*-1\r\n")
            .map_err(|error| format!("RESP write failed: {error}")),
        RespValue::Array(Some(values)) => {
            output
                .write_all(format!("*{}\r\n", values.len()).as_bytes())
                .map_err(|error| format!("RESP write failed: {error}"))?;
            for value in values {
                write_value(output, value)?;
            }
            Ok(())
        }
    }
}

fn line_value(output: &mut impl Write, prefix: u8, value: &str) -> Result<(), String> {
    if value.contains(['\r', '\n']) {
        return Err("RESP line values cannot contain CR or LF".into());
    }
    output
        .write_all(&[prefix])
        .and_then(|_| output.write_all(value.as_bytes()))
        .and_then(|_| output.write_all(b"\r\n"))
        .map_err(|error| format!("RESP write failed: {error}"))
}

pub struct RespServer {
    host: String,
    port: u16,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RespServer {
    pub fn start(host: &str, port: u16, broker: RuntimeBroker) -> Result<Self, String> {
        let listener = TcpListener::bind((host, port))
            .map_err(|error| format!("RESP bind {host}:{port} failed: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("RESP address failed: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("RESP listener setup failed: {error}"))?;
        let running = Arc::new(AtomicBool::new(true));
        let active = running.clone();
        let instance = format!("RUST-{}-{}", std::process::id(), address.port());
        let root = std::env::current_dir()
            .unwrap_or_default()
            .display()
            .to_string();
        let thread = std::thread::Builder::new()
            .name("hara-resp-listener".into())
            .spawn(move || {
                while active.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            if stream.set_nonblocking(false).is_err() {
                                continue;
                            }
                            let broker = broker.clone();
                            let instance = instance.clone();
                            let root = root.clone();
                            let _ = std::thread::Builder::new()
                                .name("hara-resp-client".into())
                                .spawn(move || serve(stream, broker, &instance, &root));
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| format!("RESP listener thread failed: {error}"))?;
        Ok(Self {
            host: host.into(),
            port: address.port(),
            running,
            thread: Some(thread),
        })
    }

    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for RespServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// RESP4 listener for the session fabric. It intentionally lives beside the
/// evaluator listener so existing REPL clients keep their legacy behavior.
pub struct FabricRespServer {
    host: String,
    port: u16,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FabricRespServer {
    pub fn start(host: &str, port: u16, service: FabricService) -> Result<Self, String> {
        let listener = TcpListener::bind((host, port))
            .map_err(|error| format!("RESP bind {host}:{port} failed: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("RESP address failed: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("RESP listener setup failed: {error}"))?;
        let running = Arc::new(AtomicBool::new(true));
        let active = running.clone();
        let instance = format!("FABRIC-{}-{}", std::process::id(), address.port());
        let thread = std::thread::Builder::new()
            .name("hara-fabric-listener".into())
            .spawn(move || {
                while active.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let service = service.clone();
                            let instance = instance.clone();
                            let _ = std::thread::Builder::new()
                                .name("hara-fabric-client".into())
                                .spawn(move || serve_fabric(stream, service, &instance));
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| format!("RESP listener thread failed: {error}"))?;
        Ok(Self {
            host: host.into(),
            port: address.port(),
            running,
            thread: Some(thread),
        })
    }

    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for FabricRespServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn serve_fabric(stream: TcpStream, service: FabricService, instance: &str) {
    let Ok(mut connection) = RespConnection::new(stream) else {
        return;
    };
    let mut space = "default".to_owned();
    let mut session = "ROOT".to_owned();
    loop {
        let request = match connection.read() {
            Ok(Some(RespValue::Array(Some(values)))) => values,
            Ok(Some(_)) => {
                let _ = connection.write(&RespValue::Error("BAD_REQUEST expected array".into()));
                continue;
            }
            Ok(None) => return,
            Err(error) => {
                let _ = connection.write(&RespValue::Error(format!("BAD_REQUEST {error}")));
                continue;
            }
        };
        if request.is_empty() {
            continue;
        }
        let Ok(operation) = fabric_text(&request, 0, "operation") else {
            let _ = connection.write(&RespValue::Error("BAD_REQUEST operation required".into()));
            continue;
        };
        let operation = operation.to_ascii_uppercase();
        if operation == "QUIT" {
            let _ = connection.write(&RespValue::Simple("OK".into()));
            return;
        }
        if operation == "HELLO" {
            let _ = connection.write(&RespValue::array([
                "SERVER",
                "HARA-FABRIC",
                "INSTANCE",
                instance,
                "PROTOCOL",
                "4",
                "SPACE",
                &space,
                "SESSION",
                &session,
            ]));
            continue;
        }
        let id = request
            .get(1)
            .and_then(RespValue::text)
            .unwrap_or_else(|| "?".into());
        let result = fabric_operation(
            &service,
            &mut space,
            &mut session,
            &operation,
            &request[2..],
        );
        match result {
            Ok(values) => {
                let mut frame = vec![RespValue::bulk("RESULT"), RespValue::bulk(&id)];
                frame.extend(values);
                let _ = connection.write(&RespValue::Array(Some(frame)));
                let _ = connection.write(&RespValue::array(["DONE", &id, "OK"]));
            }
            Err(error) => {
                let (code, message) = error
                    .split_once(':')
                    .map_or(("service/error", error.as_str()), |(code, message)| {
                        (code, message.trim())
                    });
                let _ = connection.write(&RespValue::array(["ERROR", &id, code, message]));
                let _ = connection.write(&RespValue::array(["DONE", &id, "ERROR"]));
            }
        }
    }
}

fn fabric_operation(
    service: &FabricService,
    attached_space: &mut String,
    attached_session: &mut String,
    operation: &str,
    arguments: &[RespValue],
) -> Result<Vec<RespValue>, String> {
    match operation {
        "SPACE" => {
            let action = fabric_text(arguments, 0, "SPACE action")?.to_ascii_uppercase();
            match action.as_str() {
                "CREATE" => {
                    let name = fabric_text(arguments, 1, "space name")?;
                    service.create_space(&name)?;
                    Ok(vec![RespValue::bulk(name)])
                }
                "ATTACH" => {
                    let name = fabric_text(arguments, 1, "space name")?;
                    if !service.list_spaces()?.contains(&name) {
                        return Err(format!("space/not-found: {name}"));
                    }
                    *attached_space = name.clone();
                    *attached_session = "ROOT".into();
                    Ok(vec![RespValue::bulk(name)])
                }
                "LIST" => Ok(service
                    .list_spaces()?
                    .into_iter()
                    .map(RespValue::bulk)
                    .collect()),
                "DROP" => {
                    let name = fabric_text(arguments, 1, "space name")?;
                    service.drop_space(&name)?;
                    if *attached_space == name {
                        *attached_space = "default".into();
                        *attached_session = "ROOT".into();
                    }
                    Ok(vec![RespValue::bulk("OK")])
                }
                "INFO" => Ok(vec![
                    RespValue::bulk(attached_space.clone()),
                    RespValue::bulk(attached_session.clone()),
                ]),
                _ => Err(format!("space/action: {action}")),
            }
        }
        "SESSION" => {
            let action = fabric_text(arguments, 0, "SESSION action")?.to_ascii_uppercase();
            match action.as_str() {
                "NEW" => {
                    let name = fabric_text(arguments, 1, "session name")?;
                    service.create_session(attached_space, &name)?;
                    Ok(vec![RespValue::bulk(name)])
                }
                "ATTACH" => {
                    let name = fabric_text(arguments, 1, "session name")?;
                    if !service.list_sessions(attached_space)?.contains(&name) {
                        return Err(format!("session/not-found: {name}"));
                    }
                    *attached_session = name.clone();
                    Ok(vec![RespValue::bulk(name)])
                }
                "DETACH" => {
                    *attached_session = "ROOT".into();
                    Ok(vec![RespValue::bulk("ROOT")])
                }
                "LIST" => Ok(service
                    .list_sessions(attached_space)?
                    .into_iter()
                    .map(RespValue::bulk)
                    .collect()),
                "CLOSE" => {
                    let name = fabric_text(arguments, 1, "session name")?;
                    service.close_session(attached_space, &name)?;
                    if *attached_session == name {
                        *attached_session = "ROOT".into();
                    }
                    Ok(vec![RespValue::bulk("OK")])
                }
                "INFO" => Ok(vec![
                    RespValue::bulk(attached_space.clone()),
                    RespValue::bulk(attached_session.clone()),
                ]),
                _ => Err(format!("session/action: {action}")),
            }
        }
        "EVAL" => Ok(vec![RespValue::bulk(service.eval(
            attached_space,
            attached_session,
            &fabric_text(arguments, 0, "source")?,
        )?)]),
        "MODULE" => {
            let action = fabric_text(arguments, 0, "MODULE action")?.to_ascii_uppercase();
            match action.as_str() {
                "PUT" => {
                    let manifest = fabric_text(arguments, 1, "manifest")?;
                    let module = fabric_bytes(arguments, 2, "module")?;
                    Ok(vec![RespValue::bulk(service.put_module(&manifest, &module)?)])
                }
                "LOAD" => {
                    let digest = fabric_text(arguments, 1, "digest")?;
                    Ok(vec![RespValue::bulk(service.load_module(
                        attached_space,
                        attached_session,
                        &digest,
                    )?)])
                }
                "CALL" => {
                    let qualified = fabric_text(arguments, 1, "namespace/export")?;
                    let args = fabric_bytes(arguments, 2, "HTA1 arguments")?;
                    Ok(vec![RespValue::Bulk(Some(service.invoke_module(
                        attached_space,
                        attached_session,
                        &qualified,
                        &args,
                    )?))])
                }
                _ => Err(format!("module/action: {action}")),
            }
        }
        "REPORT" => {
            let action = fabric_text(arguments, 0, "REPORT action")?.to_ascii_uppercase();
            match action.as_str() {
                "SEND" => {
                    let target = fabric_text(arguments, 1, "target session")?;
                    let signal = fabric_text(arguments, 2, "signal")?;
                    let data = fabric_bytes(arguments, 3, "report data")?;
                    let receipt = service.send_report(
                        attached_space,
                        attached_session,
                        &target,
                        &signal,
                        &data,
                    )?;
                    Ok(vec![
                        RespValue::bulk(receipt.id),
                        RespValue::Integer(receipt.delivered as i64),
                        RespValue::Integer(i64::from(receipt.retained)),
                        receipt
                            .sequence
                            .map_or(RespValue::Bulk(None), |value| RespValue::Integer(value as i64)),
                    ])
                }
                "SUBSCRIBE" => {
                    let signal = fabric_text(arguments, 1, "signal")?;
                    Ok(vec![RespValue::bulk(service.subscribe(
                        attached_space,
                        attached_session,
                        &signal,
                    )?)])
                }
                "UNSUBSCRIBE" => {
                    let id = fabric_text(arguments, 1, "subscription")?;
                    Ok(vec![RespValue::Integer(i64::from(service.unsubscribe(&id)?))])
                }
                "NEXT" => {
                    let id = fabric_text(arguments, 1, "subscription")?;
                    Ok(match service.next_report(&id)? {
                        Some(report) => report_values(report),
                        None => vec![RespValue::Bulk(None)],
                    })
                }
                "RETAIN" => {
                    let signal = fabric_text(arguments, 1, "signal")?;
                    let max_events = fabric_usize(arguments, 2, "max events")?;
                    let max_age_ms = fabric_u64(arguments, 3, "max age ms")?;
                    service.retain(
                        attached_space,
                        attached_session,
                        &signal,
                        max_events,
                        max_age_ms,
                    )?;
                    Ok(vec![RespValue::bulk("OK")])
                }
                "REPLAY" => {
                    let signal = fabric_text(arguments, 1, "signal")?;
                    let after = fabric_u64(arguments, 2, "after sequence")?;
                    let limit = fabric_usize(arguments, 3, "limit")?;
                    Ok(service
                        .replay(attached_space, attached_session, &signal, after, limit)?
                        .into_iter()
                        .map(|report| RespValue::Array(Some(report_values(report))))
                        .collect())
                }
                _ => Err(format!("report/action: {action}")),
            }
        }
        "METRICS" => Ok(vec![RespValue::bulk(service.metrics()?.json())]),
        "EVENTS" => {
            let cursor = fabric_u64(arguments, 0, "cursor")?;
            let limit = fabric_usize(arguments, 1, "limit")?;
            Ok(service
                .events_since(cursor, limit)?
                .into_iter()
                .map(|event| RespValue::bulk(event.json()))
                .collect())
        }
        "TOPOLOGY" => Ok(vec![RespValue::bulk(service.topology_json()?)]),
        "COMMANDS" => Ok(vec![RespValue::bulk(
            "HELLO SPACE SESSION EVAL MODULE REPORT METRICS EVENTS TOPOLOGY COMMANDS QUIT",
        )]),
        _ => Err(format!("protocol/unknown-operation: {operation}")),
    }
}

fn report_values(report: crate::service::Report) -> Vec<RespValue> {
    vec![
        RespValue::bulk(report.id),
        RespValue::bulk(report.space),
        RespValue::bulk(report.source),
        RespValue::bulk(report.target),
        RespValue::bulk(report.signal),
        RespValue::Bulk(Some(report.data)),
        RespValue::Integer(report.timestamp_ms as i64),
        report
            .sequence
            .map_or(RespValue::Bulk(None), |value| RespValue::Integer(value as i64)),
    ]
}

fn fabric_text(values: &[RespValue], index: usize, label: &str) -> Result<String, String> {
    values
        .get(index)
        .and_then(RespValue::text)
        .ok_or_else(|| format!("protocol/bad-request: {label} required"))
}

fn fabric_bytes(values: &[RespValue], index: usize, label: &str) -> Result<Vec<u8>, String> {
    match values.get(index) {
        Some(RespValue::Bulk(Some(bytes))) => Ok(bytes.clone()),
        Some(RespValue::Simple(value)) => Ok(value.as_bytes().to_vec()),
        _ => Err(format!("protocol/bad-request: {label} required")),
    }
}

fn fabric_u64(values: &[RespValue], index: usize, label: &str) -> Result<u64, String> {
    fabric_text(values, index, label)?
        .parse()
        .map_err(|_| format!("protocol/bad-request: {label} must be an unsigned integer"))
}

fn fabric_usize(values: &[RespValue], index: usize, label: &str) -> Result<usize, String> {
    fabric_text(values, index, label)?
        .parse()
        .map_err(|_| format!("protocol/bad-request: {label} must be an unsigned integer"))
}

fn serve(stream: TcpStream, broker: RuntimeBroker, instance: &str, root: &str) {
    let Ok(mut connection) = RespConnection::new(stream) else {
        return;
    };
    let mut protocol = 3_u8;
    let mut attached = "ROOT".to_owned();
    loop {
        let request = match connection.read() {
            Ok(Some(RespValue::Array(Some(values)))) => values,
            Ok(Some(_)) => {
                let _ = connection.write(&RespValue::Error("BAD_REQUEST expected array".into()));
                continue;
            }
            Ok(None) => return,
            Err(error) => {
                let _ = connection.write(&RespValue::Error(format!("BAD_REQUEST {error}")));
                continue;
            }
        };
        let words = request
            .iter()
            .map(RespValue::text)
            .collect::<Option<Vec<_>>>();
        let Some(words) = words else {
            let _ = connection.write(&RespValue::Error(
                "BAD_REQUEST textual arguments required".into(),
            ));
            continue;
        };
        if words.is_empty() {
            continue;
        }
        let operation = words[0].to_ascii_uppercase();
        if operation == "QUIT" {
            let _ = connection.write(&RespValue::Simple("OK".into()));
            return;
        }
        if operation == "HELLO" {
            protocol = words
                .get(1)
                .and_then(|value| value.parse().ok())
                .unwrap_or(3);
            let hello = RespValue::array([
                "SERVER",
                "HARA",
                "INSTANCE",
                instance,
                "PROTOCOL",
                &protocol.to_string(),
                "ROOT",
                root,
            ]);
            let _ = connection.write(&hello);
            continue;
        }
        if protocol >= 4 {
            let id = words.get(1).cloned().unwrap_or_else(|| "?".into());
            handle_v4(
                &mut connection,
                &broker,
                &mut attached,
                &operation,
                &id,
                &words[2..],
            );
        } else {
            handle_legacy(
                &mut connection,
                &broker,
                &mut attached,
                &operation,
                &words[1..],
            );
        }
    }
}

fn handle_v4(
    connection: &mut RespConnection,
    broker: &RuntimeBroker,
    attached: &mut String,
    operation: &str,
    id: &str,
    arguments: &[String],
) {
    let result = operation_result(broker, attached, operation, arguments);
    match result {
        Ok(value) => {
            let _ = connection.write(&RespValue::array(["RESULT", id, &value]));
            let _ = connection.write(&RespValue::array(["DONE", id, "OK"]));
        }
        Err((code, message)) => {
            let _ = connection.write(&RespValue::array(["ERROR", id, code, &message]));
            let _ = connection.write(&RespValue::array(["DONE", id, "ERROR"]));
        }
    }
}

fn handle_legacy(
    connection: &mut RespConnection,
    broker: &RuntimeBroker,
    attached: &mut String,
    operation: &str,
    arguments: &[String],
) {
    let result = if operation == "EVAL" && arguments.len() >= 2 {
        broker
            .eval(&arguments[0], &arguments[1])
            .map_err(|message| ("EVAL_ERROR", message))
    } else {
        operation_result(broker, attached, operation, arguments)
    };
    let response = match result {
        Ok(value) => RespValue::bulk(value),
        Err((code, message)) => RespValue::Error(format!("{code} {message}")),
    };
    let _ = connection.write(&response);
}

fn operation_result(
    broker: &RuntimeBroker,
    attached: &mut String,
    operation: &str,
    arguments: &[String],
) -> Result<String, (&'static str, String)> {
    match operation {
        "EVAL" => {
            let source = arguments
                .first()
                .ok_or(("BAD_REQUEST", "EVAL requires source".into()))?;
            broker
                .eval(attached, source)
                .map_err(|error| ("EVAL_ERROR", error))
        }
        "COMPLETE" => {
            let prefix = arguments.first().map_or("", String::as_str);
            broker
                .complete(attached, prefix)
                .map(|values| values.join("\n"))
                .map_err(|error| ("NO_SESSION", error))
        }
        "SESSION" => session_operation(broker, attached, arguments),
        "COMMANDS" => Ok("HELLO EVAL COMPLETE SESSION COMMANDS INFO QUIT".into()),
        "INFO" => broker.info(attached).map_err(|error| ("NO_SESSION", error)),
        _ => Err(("UNKNOWN_OP", format!("Unknown operation: {operation}"))),
    }
}

fn session_operation(
    broker: &RuntimeBroker,
    attached: &mut String,
    arguments: &[String],
) -> Result<String, (&'static str, String)> {
    let action = arguments
        .first()
        .map(|value| value.to_ascii_uppercase())
        .ok_or(("BAD_REQUEST", "SESSION requires an action".into()))?;
    match action.as_str() {
        "NEW" => broker
            .create(
                arguments
                    .get(1)
                    .ok_or(("BAD_REQUEST", "SESSION NEW requires name".into()))?,
            )
            .map_err(|error| ("BAD_REQUEST", error)),
        "LIST" => broker
            .list()
            .map(|values| values.join("\n"))
            .map_err(|error| ("INTERNAL_ERROR", error)),
        "ATTACH" => {
            let name = arguments
                .get(1)
                .ok_or(("BAD_REQUEST", "SESSION ATTACH requires name".into()))?;
            broker.info(name).map_err(|error| ("NO_SESSION", error))?;
            *attached = name.clone();
            Ok(name.clone())
        }
        "DETACH" => {
            *attached = "ROOT".into();
            Ok("ROOT".into())
        }
        "INFO" => broker.info(attached).map_err(|error| ("NO_SESSION", error)),
        "CLOSE" => broker
            .close(
                arguments
                    .get(1)
                    .ok_or(("BAD_REQUEST", "SESSION CLOSE requires name".into()))?,
            )
            .map_err(|error| ("BAD_REQUEST", error)),
        _ => Err(("BAD_REQUEST", format!("Unknown SESSION action: {action}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::{RespConnection, RespValue};
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn resp2_values_round_trip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let writer = std::thread::spawn(move || {
            let mut connection = RespConnection::new(TcpStream::connect(address).unwrap()).unwrap();
            connection
                .write(&RespValue::Array(Some(vec![
                    RespValue::Simple("OK".into()),
                    RespValue::Integer(42),
                    RespValue::Bulk(None),
                    RespValue::bulk("hello"),
                ])))
                .unwrap();
        });
        let (stream, _) = listener.accept().unwrap();
        let mut connection = RespConnection::new(stream).unwrap();
        assert_eq!(
            connection.read().unwrap().unwrap(),
            RespValue::Array(Some(vec![
                RespValue::Simple("OK".into()),
                RespValue::Integer(42),
                RespValue::Bulk(None),
                RespValue::bulk("hello"),
            ]))
        );
        writer.join().unwrap();
    }
    #[test]
    fn server_streams_protocol_four_and_shares_root_with_legacy_clients() {
        let broker = crate::native_cli::RuntimeBroker::start().unwrap();
        broker.eval("ROOT", "(def answer 41)").unwrap();
        let mut server = super::RespServer::start("127.0.0.1", 0, broker).unwrap();
        let endpoint = server.endpoint();

        let mut legacy = RespConnection::new(TcpStream::connect(&endpoint).unwrap()).unwrap();
        legacy
            .write(&RespValue::array(["EVAL", "ROOT", "(+ answer 1)"]))
            .unwrap();
        assert_eq!(legacy.read().unwrap().unwrap().text().unwrap(), "42");

        let mut modern = RespConnection::new(TcpStream::connect(&endpoint).unwrap()).unwrap();
        modern.write(&RespValue::array(["HELLO", "4"])).unwrap();
        let hello = modern.read().unwrap().unwrap();
        assert!(matches!(hello, RespValue::Array(Some(_))));
        modern
            .write(&RespValue::array(["EVAL", "REQ-1", "answer"]))
            .unwrap();
        assert_eq!(
            modern.read().unwrap().unwrap(),
            RespValue::array(["RESULT", "REQ-1", "41"])
        );
        assert_eq!(
            modern.read().unwrap().unwrap(),
            RespValue::array(["DONE", "REQ-1", "OK"])
        );
        server.stop();
    }

    #[test]
    fn fabric_protocol_manages_spaces_reports_and_redacted_analytics() {
        let service = crate::service::FabricService::start(crate::service::ServiceConfig {
            shards: 2,
            ..crate::service::ServiceConfig::default()
        })
        .unwrap();
        let mut server = super::FabricRespServer::start("127.0.0.1", 0, service).unwrap();
        let mut connection =
            RespConnection::new(TcpStream::connect(server.endpoint()).unwrap()).unwrap();
        connection
            .write(&RespValue::array(["HELLO", "4"]))
            .unwrap();
        assert!(matches!(
            connection.read().unwrap().unwrap(),
            RespValue::Array(Some(_))
        ));

        let command = |connection: &mut RespConnection, values: &[&str]| {
            connection.write(&RespValue::array(values.iter().copied())).unwrap();
            let result = connection.read().unwrap().unwrap();
            let done = connection.read().unwrap().unwrap();
            assert!(matches!(done, RespValue::Array(Some(_))));
            result
        };
        command(
            &mut connection,
            &["SPACE", "1", "CREATE", "workroom"],
        );
        command(
            &mut connection,
            &["SPACE", "2", "ATTACH", "workroom"],
        );
        command(
            &mut connection,
            &["SESSION", "3", "NEW", "researcher"],
        );
        command(
            &mut connection,
            &["SESSION", "4", "NEW", "reviewer"],
        );
        command(
            &mut connection,
            &["SESSION", "5", "ATTACH", "reviewer"],
        );
        let subscribed = command(
            &mut connection,
            &["REPORT", "6", "SUBSCRIBE", "finding"],
        );
        let RespValue::Array(Some(values)) = subscribed else {
            panic!("expected subscribe result");
        };
        let subscription = values[2].text().unwrap();
        command(
            &mut connection,
            &["SESSION", "7", "ATTACH", "researcher"],
        );
        connection
            .write(&RespValue::Array(Some(vec![
                RespValue::bulk("REPORT"),
                RespValue::bulk("8"),
                RespValue::bulk("SEND"),
                RespValue::bulk("reviewer"),
                RespValue::bulk("finding"),
                RespValue::Bulk(Some(vec![0, 1, 2, 255])),
            ])))
            .unwrap();
        let _ = connection.read().unwrap().unwrap();
        let _ = connection.read().unwrap().unwrap();
        let next = command(
            &mut connection,
            &["REPORT", "9", "NEXT", &subscription],
        );
        let RespValue::Array(Some(values)) = next else {
            panic!("expected report result");
        };
        assert_eq!(values[7], RespValue::Bulk(Some(vec![0, 1, 2, 255])));

        let metrics = command(&mut connection, &["METRICS", "10"]);
        assert!(metrics.text().is_none());
        let RespValue::Array(Some(values)) = metrics else {
            panic!("expected metrics result");
        };
        assert!(values[2]
            .text()
            .unwrap()
            .contains("\"reports_accepted\":1"));
        server.stop();
    }
}
