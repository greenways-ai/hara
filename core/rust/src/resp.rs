#![cfg(not(target_arch = "wasm32"))]

//! RESP listener and wire codec for the native runtime broker. Wire format,
//! dialects, and operation semantics are specified in
//! `specs/01-lang/007-resp/draft/hal-resp-protocol.md`.

use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::native_cli::RuntimeBroker;

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
}
