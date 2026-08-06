//! Optional direct PostgreSQL implementation of `hara.db-provider/1`.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use futures_util::future::poll_fn;
use hara_abi::{Error, NativeIdentity, NativeModule, TaskEvent, TaskId, Value};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc as tokio_mpsc, Notify};
use tokio_postgres::config::SslMode;
use tokio_postgres::types::{Format, FromSql, IsNull, Kind, ToSql, Type};
use tokio_postgres::{AsyncMessage, Client, Config, NoTls, Row};
use tokio_postgres_rustls::MakeRustlsConnect;
use uuid::Uuid;

const OPERATIONS: &[&str] = &[
    "describe",
    "open",
    "close",
    "version",
    "exec",
    "query",
    "wait-ready",
    "database-create",
    "database-drop",
    "server-start",
    "server-stop",
    "listen",
    "notification-next",
    "unlisten",
    "notify",
];
const CAPABILITIES: &[&str] = &[
    "sql",
    "transactions",
    "notifications",
    "database-admin",
    "numeric",
    "arrays",
    "tagged-decode",
];

struct Command {
    task: TaskId,
    operation: String,
    arguments: Vec<Value>,
}

struct Task {
    receiver: mpsc::Receiver<Result<Value, Error>>,
    settled: Option<Result<Value, Error>>,
}

#[derive(Clone)]
struct Subscription {
    connection: i64,
    channel: String,
    queue: Arc<Mutex<VecDeque<Value>>>,
    notify: Arc<Notify>,
}

pub struct PostgresModule {
    identity: NativeIdentity,
    commands: tokio_mpsc::UnboundedSender<Command>,
    completions: Arc<Mutex<HashMap<TaskId, mpsc::Sender<Result<Value, Error>>>>>,
    tasks: Mutex<HashMap<TaskId, Task>>,
    next_task: AtomicU64,
}

impl PostgresModule {
    pub fn new() -> Result<Self, Error> {
        let identity = NativeIdentity::new(
            "gh:hara-lang:std-db-postgres",
            "std.db.postgres",
            "hara-db-postgres",
            "hara.db-provider/1",
        )?;
        let (commands, receiver) = tokio_mpsc::unbounded_channel();
        let completions = Arc::new(Mutex::new(HashMap::new()));
        let worker_completions = completions.clone();
        std::thread::Builder::new()
            .name("hara-db-postgres".into())
            .spawn(move || worker(receiver, worker_completions))
            .map_err(|error| Error::new("postgres/worker", error.to_string()))?;
        Ok(Self {
            identity,
            commands,
            completions,
            tasks: Mutex::new(HashMap::new()),
            next_task: AtomicU64::new(0),
        })
    }
}

impl NativeModule for PostgresModule {
    fn identity(&self) -> &NativeIdentity {
        &self.identity
    }
    fn operations(&self) -> &[&str] {
        OPERATIONS
    }
    fn capabilities(&self) -> &[&str] {
        CAPABILITIES
    }

    fn start(&self, operation: &str, arguments: Vec<Value>) -> Result<TaskId, Error> {
        if !OPERATIONS.contains(&operation) {
            return Err(Error::new("postgres/operation-unknown", operation));
        }
        let task = self.next_task.fetch_add(1, Ordering::Relaxed) + 1;
        let (sender, receiver) = mpsc::channel();
        self.completions.lock().unwrap().insert(task, sender);
        self.tasks.lock().unwrap().insert(
            task,
            Task {
                receiver,
                settled: None,
            },
        );
        self.commands
            .send(Command {
                task,
                operation: operation.into(),
                arguments,
            })
            .map_err(|_| Error::new("postgres/worker-closed", "command worker is unavailable"))?;
        Ok(task)
    }

    fn poll(&self, task: TaskId) -> Result<TaskEvent, Error> {
        let mut tasks = self.tasks.lock().unwrap();
        let task = tasks
            .get_mut(&task)
            .ok_or_else(|| Error::new("postgres/task-missing", task.to_string()))?;
        if task.settled.is_none() {
            match task.receiver.try_recv() {
                Ok(value) => task.settled = Some(value),
                Err(mpsc::TryRecvError::Empty) => return Ok(TaskEvent::Pending),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(Error::new(
                        "postgres/worker-closed",
                        "result channel closed",
                    ))
                }
            }
        }
        Ok(match task.settled.clone().unwrap() {
            Ok(value) => TaskEvent::Resolved(value),
            Err(error) => TaskEvent::Rejected(error),
        })
    }

    fn wait(&self, task: TaskId, timeout: Option<Duration>) -> Result<TaskEvent, Error> {
        let mut tasks = self.tasks.lock().unwrap();
        let task = tasks
            .get_mut(&task)
            .ok_or_else(|| Error::new("postgres/task-missing", task.to_string()))?;
        if task.settled.is_none() {
            let result = match timeout {
                Some(timeout) => {
                    task.receiver
                        .recv_timeout(timeout)
                        .map_err(|error| match error {
                            mpsc::RecvTimeoutError::Timeout => {
                                Error::new("postgres/timeout", "operation timed out")
                            }
                            mpsc::RecvTimeoutError::Disconnected => {
                                Error::new("postgres/worker-closed", "result channel closed")
                            }
                        })?
                }
                None => task
                    .receiver
                    .recv()
                    .map_err(|_| Error::new("postgres/worker-closed", "result channel closed"))?,
            };
            task.settled = Some(result);
        }
        Ok(match task.settled.clone().unwrap() {
            Ok(value) => TaskEvent::Resolved(value),
            Err(error) => TaskEvent::Rejected(error),
        })
    }

    fn cancel(&self, task: TaskId) -> Result<(), Error> {
        self.completions.lock().unwrap().remove(&task);
        Ok(())
    }

    fn drop_task(&self, task: TaskId) {
        self.completions.lock().unwrap().remove(&task);
        self.tasks.lock().unwrap().remove(&task);
    }

    fn shutdown(&self) {}
}

pub fn module() -> Arc<dyn NativeModule> {
    Arc::new(PostgresModule::new().expect("valid PostgreSQL native module"))
}

fn worker(
    mut commands: tokio_mpsc::UnboundedReceiver<Command>,
    completions: Arc<Mutex<HashMap<TaskId, mpsc::Sender<Result<Value, Error>>>>>,
) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build();
    let Ok(runtime) = runtime else { return };
    runtime.block_on(async move {
        let subscriptions = Arc::new(Mutex::new(HashMap::<i64, Subscription>::new()));
        let mut state = WorkerState {
            connections: HashMap::new(),
            subscriptions: subscriptions.clone(),
            next_connection: 0,
            next_subscription: 0,
        };
        while let Some(command) = commands.recv().await {
            if command.operation == "notification-next" {
                let result = expect_i64(command.arguments.first(), "subscription").and_then(|id| {
                    state
                        .subscriptions
                        .lock()
                        .unwrap()
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| Error::new("postgres/subscription-closed", id.to_string()))
                });
                let completions = completions.clone();
                tokio::spawn(async move {
                    let result = match result {
                        Ok(subscription) => notification_next(subscription).await,
                        Err(error) => Err(error),
                    };
                    if let Some(sender) = completions.lock().unwrap().remove(&command.task) {
                        let _ = sender.send(result);
                    }
                });
                continue;
            }
            let result = state.call(&command.operation, command.arguments).await;
            if let Some(sender) = completions.lock().unwrap().remove(&command.task) {
                let _ = sender.send(result);
            }
        }
    });
}

struct WorkerState {
    connections: HashMap<i64, Client>,
    subscriptions: Arc<Mutex<HashMap<i64, Subscription>>>,
    next_connection: i64,
    next_subscription: i64,
}

impl WorkerState {
    async fn call(&mut self, operation: &str, arguments: Vec<Value>) -> Result<Value, Error> {
        match operation {
            "describe" => Ok(record([
                ("engine", Value::String("postgresql".into())),
                ("provider", Value::String("postgres".into())),
                ("mode", Value::String("remote".into())),
                ("capabilities", strings(CAPABILITIES)),
            ])),
            "open" => {
                self.open(expect_record(arguments.first(), "options")?)
                    .await
            }
            "close" => {
                self.close(expect_i64(arguments.first(), "connection")?)
                    .await
            }
            "version" => {
                self.version(expect_i64(arguments.first(), "connection")?)
                    .await
            }
            "exec" => self.execute(arguments, false).await,
            "query" => self.execute(arguments, true).await,
            "wait-ready" => {
                self.wait_ready(expect_record(arguments.first(), "options")?)
                    .await
            }
            "database-create" => self.database_admin(arguments, true).await,
            "database-drop" => self.database_admin(arguments, false).await,
            "listen" => self.listen(arguments).await,
            "notification-next" => {
                self.notification_next(expect_i64(arguments.first(), "subscription")?)
                    .await
            }
            "unlisten" => {
                self.unlisten(expect_i64(arguments.first(), "subscription")?)
                    .await
            }
            "notify" => self.notify(arguments).await,
            "server-start" | "server-stop" => {
                Err(Error::new("postgres/capability-unsupported", operation))
            }
            _ => Err(Error::new("postgres/operation-unknown", operation)),
        }
    }

    async fn open(&mut self, options: &BTreeMap<String, Value>) -> Result<Value, Error> {
        let config = connection_config(options)?;
        self.next_connection += 1;
        let id = self.next_connection;
        let client = connect(config, id, self.subscriptions.clone()).await?;
        self.connections.insert(id, client);
        Ok(record([
            ("id", Value::Integer(id)),
            ("engine", Value::String("postgresql".into())),
            ("provider", Value::String("postgres".into())),
            ("mode", Value::String("remote".into())),
            ("capabilities", strings(CAPABILITIES)),
        ]))
    }

    async fn close(&mut self, id: i64) -> Result<Value, Error> {
        let removed = self.connections.remove(&id).is_some();
        let subscriptions = self
            .subscriptions
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(key, value)| (value.connection == id).then_some(*key))
            .collect::<Vec<_>>();
        for subscription in subscriptions {
            let _ = self.unlisten(subscription).await;
        }
        Ok(Value::Boolean(removed))
    }

    fn client(&self, id: i64) -> Result<&Client, Error> {
        self.connections
            .get(&id)
            .ok_or_else(|| Error::new("postgres/connection-closed", id.to_string()))
    }

    async fn version(&self, id: i64) -> Result<Value, Error> {
        let row = self
            .client(id)?
            .query_one("select version()", &[])
            .await
            .map_err(query_error)?;
        Ok(record([
            ("engine", Value::String("postgresql".into())),
            ("provider", Value::String("postgres".into())),
            ("version", Value::String(row.get::<_, String>(0))),
        ]))
    }

    async fn execute(&self, arguments: Vec<Value>, rows: bool) -> Result<Value, Error> {
        let id = expect_i64(arguments.first(), "connection")?;
        let sql = expect_string(arguments.get(1), "sql")?;
        let values = expect_vector(arguments.get(2), "parameters")?;
        let client = self.client(id)?;
        let decode = if rows {
            decode_mode(arguments.get(3))?
        } else {
            DecodeMode::Simple
        };
        let inferred = client.prepare(sql).await.map_err(query_error)?;
        let parameter_types = inferred
            .params()
            .iter()
            .zip(values.iter())
            .map(|(inferred, value)| parameter_type(value, inferred))
            .collect::<Result<Vec<_>, _>>()?;
        let statement = if parameter_types != inferred.params() {
            client
                .prepare_typed(sql, &parameter_types)
                .await
                .map_err(query_error)?
        } else {
            inferred
        };
        if statement.params().len() != values.len() {
            return Err(Error::new(
                "postgres/config-invalid",
                "parameter count does not match statement",
            ));
        }
        let parameters = values
            .iter()
            .zip(statement.params())
            .map(|(value, ty)| parameter(value, ty))
            .collect::<Result<Vec<_>, _>>()?;
        let references = parameters
            .iter()
            .map(|value| value.as_ref() as &(dyn ToSql + Sync))
            .collect::<Vec<_>>();
        if rows {
            let result = client
                .query(&statement, &references)
                .await
                .map_err(query_error)?;
            result_value(&result, decode)
        } else {
            let affected = client
                .execute(&statement, &references)
                .await
                .map_err(query_error)?;
            Ok(record([
                ("columns", Value::Vector(Vec::new())),
                ("rows", Value::Vector(Vec::new())),
                ("affected", Value::Integer(affected as i64)),
            ]))
        }
    }

    async fn wait_ready(&mut self, options: &BTreeMap<String, Value>) -> Result<Value, Error> {
        let timeout = integer_option(options, "timeout-ms", 10_000).max(0) as u64;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout);
        loop {
            match connect(connection_config(options)?, 0, self.subscriptions.clone()).await {
                Ok(client) => {
                    drop(client);
                    return Ok(record([
                        ("ready", Value::Boolean(true)),
                        ("provider", Value::String("postgres".into())),
                    ]));
                }
                Err(_error) if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
                Err(error) => return Err(Error::new("postgres/timeout", error.detail)),
            }
        }
    }

    async fn database_admin(
        &mut self,
        arguments: Vec<Value>,
        create: bool,
    ) -> Result<Value, Error> {
        let options = expect_record(arguments.first(), "options")?;
        let database = expect_string(arguments.get(1), "database")?;
        let client = connect(connection_config(options)?, 0, self.subscriptions.clone()).await?;
        let command = if create {
            "create database"
        } else {
            "drop database"
        };
        client
            .batch_execute(&format!("{command} {}", identifier(database)?))
            .await
            .map_err(query_error)?;
        Ok(record([
            (
                "status",
                Value::String(if create { "created" } else { "dropped" }.into()),
            ),
            ("database", Value::String(database.into())),
        ]))
    }

    async fn listen(&mut self, arguments: Vec<Value>) -> Result<Value, Error> {
        let connection = expect_i64(arguments.first(), "connection")?;
        let channel = expect_string(arguments.get(1), "channel")?.to_owned();
        self.client(connection)?
            .batch_execute(&format!("listen {}", identifier(&channel)?))
            .await
            .map_err(query_error)?;
        self.next_subscription += 1;
        let id = self.next_subscription;
        self.subscriptions.lock().unwrap().insert(
            id,
            Subscription {
                connection,
                channel: channel.clone(),
                queue: Arc::new(Mutex::new(VecDeque::new())),
                notify: Arc::new(Notify::new()),
            },
        );
        Ok(record([
            ("id", Value::Integer(id)),
            ("channel", Value::String(channel)),
        ]))
    }

    async fn notification_next(&self, id: i64) -> Result<Value, Error> {
        let subscription = self
            .subscriptions
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::new("postgres/subscription-closed", id.to_string()))?;
        notification_next(subscription).await
    }

    async fn unlisten(&mut self, id: i64) -> Result<Value, Error> {
        let subscription = self.subscriptions.lock().unwrap().remove(&id);
        let Some(subscription) = subscription else {
            return Ok(Value::Boolean(false));
        };
        let still_used = self.subscriptions.lock().unwrap().values().any(|value| {
            value.connection == subscription.connection && value.channel == subscription.channel
        });
        if !still_used {
            self.client(subscription.connection)?
                .batch_execute(&format!("unlisten {}", identifier(&subscription.channel)?))
                .await
                .map_err(query_error)?;
        }
        Ok(Value::Boolean(true))
    }

    async fn notify(&self, arguments: Vec<Value>) -> Result<Value, Error> {
        let connection = expect_i64(arguments.first(), "connection")?;
        let channel = expect_string(arguments.get(1), "channel")?;
        let payload = expect_string(arguments.get(2), "payload")?;
        self.client(connection)?
            .execute("select pg_notify($1, $2)", &[&channel, &payload])
            .await
            .map_err(query_error)?;
        Ok(Value::Boolean(true))
    }
}

async fn notification_next(subscription: Subscription) -> Result<Value, Error> {
    loop {
        if let Some(value) = subscription.queue.lock().unwrap().pop_front() {
            return Ok(value);
        }
        subscription.notify.notified().await;
    }
}

async fn connect(
    config: Config,
    connection_id: i64,
    subscriptions: Arc<Mutex<HashMap<i64, Subscription>>>,
) -> Result<Client, Error> {
    if config.get_ssl_mode() == SslMode::Disable {
        let (client, connection) = config.connect(NoTls).await.map_err(connect_error)?;
        tokio::spawn(connection_driver(connection, connection_id, subscriptions));
        return Ok(client);
    }
    let certificates = rustls_native_certs::load_native_certs();
    let mut roots = rustls::RootCertStore::empty();
    for certificate in certificates.certs {
        let _ = roots.add(certificate);
    }
    if roots.is_empty() {
        return Err(Error::new(
            "postgres/tls",
            "no trusted root certificates are available",
        ));
    }
    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let (client, connection) = config
        .connect(MakeRustlsConnect::new(tls))
        .await
        .map_err(connect_error)?;
    tokio::spawn(connection_driver(connection, connection_id, subscriptions));
    Ok(client)
}

async fn connection_driver<S, T>(
    mut connection: tokio_postgres::Connection<S, T>,
    connection_id: i64,
    subscriptions: Arc<Mutex<HashMap<i64, Subscription>>>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    T: tokio_postgres::tls::TlsStream + Unpin + Send + 'static,
{
    loop {
        match poll_fn(|context| connection.poll_message(context)).await {
            Some(Ok(AsyncMessage::Notification(notification))) => {
                for subscription in subscriptions.lock().unwrap().values() {
                    if subscription.connection == connection_id
                        && subscription.channel == notification.channel()
                    {
                        subscription.queue.lock().unwrap().push_back(record([
                            ("channel", Value::String(notification.channel().into())),
                            ("payload", Value::String(notification.payload().into())),
                            ("pid", Value::Integer(notification.process_id() as i64)),
                        ]));
                        subscription.notify.notify_one();
                    }
                }
            }
            Some(Ok(_)) => {}
            Some(Err(_)) | None => break,
        }
    }
}

#[derive(Debug)]
struct Null;
impl ToSql for Null {
    fn to_sql(
        &self,
        _ty: &Type,
        _out: &mut bytes::BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        Ok(IsNull::Yes)
    }
    fn accepts(_ty: &Type) -> bool {
        true
    }
    tokio_postgres::types::to_sql_checked!();
}

#[derive(Debug)]
struct TextParameter(String);
impl ToSql for TextParameter {
    fn to_sql(
        &self,
        _ty: &Type,
        out: &mut bytes::BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        out.extend_from_slice(self.0.as_bytes());
        Ok(IsNull::No)
    }
    fn accepts(_ty: &Type) -> bool {
        true
    }
    fn encode_format(&self, _ty: &Type) -> Format {
        Format::Text
    }
    tokio_postgres::types::to_sql_checked!();
}

#[derive(Debug)]
struct RawValue(Vec<u8>);
impl<'a> FromSql<'a> for RawValue {
    fn from_sql(
        _ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(raw.to_vec()))
    }
    fn accepts(_ty: &Type) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DecodeMode {
    Simple,
    Tagged,
}

fn decode_mode(value: Option<&Value>) -> Result<DecodeMode, Error> {
    let Some(value) = value else {
        return Ok(DecodeMode::Simple);
    };
    let options = expect_record(Some(value), "query options")?;
    match options
        .get("decode")
        .and_then(string_value)
        .unwrap_or("simple")
    {
        "simple" => Ok(DecodeMode::Simple),
        "tagged" => Ok(DecodeMode::Tagged),
        _ => Err(Error::new(
            "postgres/config-invalid",
            "decode must be simple or tagged",
        )),
    }
}

fn postgres_tag(value: &Value) -> Option<(&str, &BTreeMap<String, Value>)> {
    let Value::Record(fields) = value else {
        return None;
    };
    let tag = fields.get("$postgres").and_then(string_value)?;
    matches!(tag, "numeric" | "array").then_some((tag, fields))
}

fn numeric_tag(value: impl Into<String>) -> Value {
    record([
        ("$postgres", Value::String("numeric".into())),
        ("value", Value::String(value.into())),
    ])
}

fn array_tag(element: &Type, dimensions: &[(i32, i32)], value: Value) -> Value {
    record([
        ("$postgres", Value::String("array".into())),
        ("element", Value::String(element.name().into())),
        (
            "dimensions",
            Value::Vector(
                dimensions
                    .iter()
                    .map(|(lower, length)| {
                        Value::Vector(vec![
                            Value::Integer(*lower as i64),
                            Value::Integer(*length as i64),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("value", value),
    ])
}

fn parameter_type(value: &Value, inferred: &Type) -> Result<Type, Error> {
    if matches!(*inferred, Type::JSON | Type::JSONB) {
        return Ok(inferred.clone());
    }
    let Some((tag, fields)) = postgres_tag(value) else {
        return Ok(inferred.clone());
    };
    let explicit =
        match tag {
            "numeric" => Type::NUMERIC,
            "array" => array_type(fields.get("element").and_then(string_value).ok_or_else(
                || Error::new("postgres/config-invalid", "array element is required"),
            )?)?,
            _ => unreachable!(),
        };
    if matches!(*inferred, Type::TEXT | Type::UNKNOWN) {
        Ok(explicit)
    } else {
        Ok(inferred.clone())
    }
}

fn parameter(value: &Value, ty: &Type) -> Result<Box<dyn ToSql + Sync>, Error> {
    if matches!(value, Value::Nil) {
        return Ok(Box::new(Null));
    }
    macro_rules! boxed {
        ($value:expr) => {
            Ok(Box::new($value))
        };
    }
    if matches!(*ty, Type::JSON | Type::JSONB) {
        return boxed!(abi_json(value)?);
    }
    if *ty == Type::NUMERIC {
        let numeric = match postgres_tag(value) {
            Some(("numeric", fields)) => fields.get("value").ok_or_else(|| {
                Error::new("postgres/config-invalid", "numeric value is required")
            })?,
            Some(_) => return Err(type_error(ty)),
            None => value,
        };
        return boxed!(TextParameter(numeric_parameter_text(numeric)?));
    }
    if let Kind::Array(member) = ty.kind() {
        let (array, dimensions) = match postgres_tag(value) {
            Some(("array", fields)) => {
                let declared = fields
                    .get("element")
                    .and_then(string_value)
                    .ok_or_else(|| {
                        Error::new("postgres/config-invalid", "array element is required")
                    })?;
                if array_type(declared)? != *ty {
                    return Err(Error::new(
                        "postgres/type-unsupported",
                        format!("array element {declared} does not match {}", member.name()),
                    ));
                }
                (
                    fields.get("value").ok_or_else(|| {
                        Error::new("postgres/config-invalid", "array value is required")
                    })?,
                    tagged_dimensions(fields.get("dimensions"))?,
                )
            }
            Some(_) => return Err(type_error(ty)),
            None => (value, None),
        };
        return boxed!(TextParameter(array_parameter_text(
            array,
            member,
            dimensions.as_deref(),
        )?));
    }
    match (value, ty) {
        (Value::Boolean(value), &Type::BOOL) => boxed!(*value),
        (Value::Integer(value), &Type::INT2) => {
            boxed!(i16::try_from(*value).map_err(|_| type_error(ty))?)
        }
        (Value::Integer(value), &Type::INT4) => {
            boxed!(i32::try_from(*value).map_err(|_| type_error(ty))?)
        }
        (Value::Integer(value), &Type::INT8) => boxed!(*value),
        (Value::Float(value), &Type::FLOAT4) => boxed!(*value as f32),
        (Value::Float(value), &Type::FLOAT8) => boxed!(*value),
        (Value::String(value), &Type::TEXT | &Type::VARCHAR | &Type::BPCHAR | &Type::NAME) => {
            boxed!(value.clone())
        }
        (Value::String(value), &Type::UUID) => {
            boxed!(value.parse::<Uuid>().map_err(|_| type_error(ty))?)
        }
        (Value::String(value), &Type::DATE) => {
            boxed!(value.parse::<NaiveDate>().map_err(|_| type_error(ty))?)
        }
        (Value::String(value), &Type::TIME) => {
            boxed!(value.parse::<NaiveTime>().map_err(|_| type_error(ty))?)
        }
        (Value::String(value), &Type::TIMESTAMP) => {
            boxed!(value.parse::<NaiveDateTime>().map_err(|_| type_error(ty))?)
        }
        (Value::String(value), &Type::TIMESTAMPTZ) => {
            boxed!(value.parse::<DateTime<Utc>>().map_err(|_| type_error(ty))?)
        }
        (Value::Bytes(value), &Type::BYTEA) => boxed!(value.clone()),
        _ => Err(type_error(ty)),
    }
}

fn result_value(rows: &[Row], decode: DecodeMode) -> Result<Value, Error> {
    let columns = rows.first().map(|row| row.columns()).unwrap_or(&[]);
    let names = Value::Vector(
        columns
            .iter()
            .map(|column| Value::String(column.name().into()))
            .collect(),
    );
    let rows = rows
        .iter()
        .map(|row| {
            row.columns()
                .iter()
                .enumerate()
                .map(|(index, column)| column_value(row, index, column.type_(), decode))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Vector)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(record([
        ("columns", names),
        ("rows", Value::Vector(rows.clone())),
        ("affected", Value::Integer(rows.len() as i64)),
    ]))
}

fn column_value(row: &Row, index: usize, ty: &Type, decode: DecodeMode) -> Result<Value, Error> {
    macro_rules! nullable {
        ($type:ty, $map:expr) => {{
            let value: Option<$type> = row.try_get(index).map_err(query_error)?;
            Ok(value.map($map).unwrap_or(Value::Nil))
        }};
    }
    if *ty == Type::NUMERIC || matches!(ty.kind(), Kind::Array(_)) {
        let value: Option<RawValue> = row.try_get(index).map_err(query_error)?;
        return value
            .map(|value| decode_raw(ty, &value.0, decode))
            .unwrap_or(Ok(Value::Nil));
    }
    match *ty {
        Type::BOOL => nullable!(bool, Value::Boolean),
        Type::INT2 => nullable!(i16, |value| Value::Integer(value as i64)),
        Type::INT4 => nullable!(i32, |value| Value::Integer(value as i64)),
        Type::INT8 => nullable!(i64, Value::Integer),
        Type::FLOAT4 => nullable!(f32, |value| Value::Float(value as f64)),
        Type::FLOAT8 => nullable!(f64, Value::Float),
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => nullable!(String, Value::String),
        Type::BYTEA => nullable!(Vec<u8>, Value::Bytes),
        Type::UUID => nullable!(Uuid, |value| Value::String(value.to_string())),
        Type::JSON | Type::JSONB => nullable!(serde_json::Value, json_abi),
        Type::DATE => nullable!(NaiveDate, |value| Value::String(value.to_string())),
        Type::TIME => nullable!(NaiveTime, |value| Value::String(value.to_string())),
        Type::TIMESTAMP => nullable!(NaiveDateTime, |value| Value::String(value.to_string())),
        Type::TIMESTAMPTZ => nullable!(DateTime<Utc>, |value| Value::String(value.to_rfc3339())),
        _ => Err(type_error(ty)),
    }
}

fn decode_raw(ty: &Type, raw: &[u8], decode: DecodeMode) -> Result<Value, Error> {
    macro_rules! scalar {
        ($type:ty, $map:expr) => {
            <$type as FromSql>::from_sql(ty, raw)
                .map($map)
                .map_err(|_| type_error(ty))
        };
    }
    if *ty == Type::NUMERIC {
        let text = decode_numeric(raw)?;
        return Ok(match decode {
            DecodeMode::Tagged => numeric_tag(text),
            DecodeMode::Simple => simple_numeric(text),
        });
    }
    if let Kind::Array(member) = ty.kind() {
        return decode_array(raw, member, decode);
    }
    match *ty {
        Type::BOOL => scalar!(bool, Value::Boolean),
        Type::INT2 => scalar!(i16, |value| Value::Integer(value as i64)),
        Type::INT4 => scalar!(i32, |value| Value::Integer(value as i64)),
        Type::INT8 => scalar!(i64, Value::Integer),
        Type::FLOAT4 => scalar!(f32, |value| Value::Float(value as f64)),
        Type::FLOAT8 => scalar!(f64, Value::Float),
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => scalar!(String, Value::String),
        Type::BYTEA => scalar!(Vec<u8>, Value::Bytes),
        Type::UUID => scalar!(Uuid, |value| Value::String(value.to_string())),
        Type::JSON | Type::JSONB => scalar!(serde_json::Value, json_abi),
        Type::DATE => scalar!(NaiveDate, |value| Value::String(value.to_string())),
        Type::TIME => scalar!(NaiveTime, |value| Value::String(value.to_string())),
        Type::TIMESTAMP => scalar!(NaiveDateTime, |value| Value::String(value.to_string())),
        Type::TIMESTAMPTZ => scalar!(DateTime<Utc>, |value| Value::String(value.to_rfc3339())),
        _ => Err(type_error(ty)),
    }
}

fn simple_numeric(text: String) -> Value {
    if !matches!(text.as_str(), "NaN" | "Infinity" | "-Infinity") {
        let integral = text
            .split_once('.')
            .map(|(whole, fraction)| fraction.bytes().all(|byte| byte == b'0').then_some(whole))
            .unwrap_or(Some(text.as_str()));
        if let Some(value) = integral.and_then(|value| value.parse::<i64>().ok()) {
            return Value::Integer(value);
        }
        if let Ok(value) = text.parse::<f64>() {
            if value.is_finite() {
                return Value::Float(value);
            }
        }
    }
    numeric_tag(text)
}

fn decode_numeric(raw: &[u8]) -> Result<String, Error> {
    let mut cursor = BinaryCursor::new(raw);
    let digits = cursor.i16()?;
    let weight = cursor.i16()?;
    let sign = cursor.u16()?;
    let scale = cursor.u16()? as usize;
    if digits < 0 {
        return Err(Error::new("postgres/query", "invalid NUMERIC digit count"));
    }
    let values = (0..digits)
        .map(|_| cursor.u16())
        .collect::<Result<Vec<_>, _>>()?;
    if cursor.remaining() != 0 || values.iter().any(|value| *value >= 10_000) {
        return Err(Error::new("postgres/query", "invalid NUMERIC payload"));
    }
    match sign {
        0xC000 => return Ok("NaN".into()),
        0xD000 => return Ok("Infinity".into()),
        0xF000 => return Ok("-Infinity".into()),
        0x0000 | 0x4000 => {}
        _ => return Err(Error::new("postgres/query", "invalid NUMERIC sign")),
    }
    let mut output = String::new();
    if sign == 0x4000 {
        output.push('-');
    }
    if weight < 0 {
        output.push('0');
    } else {
        for position in 0..=weight {
            let value = values.get(position as usize).copied().unwrap_or(0);
            if position == 0 {
                output.push_str(&value.to_string());
            } else {
                output.push_str(&format!("{value:04}"));
            }
        }
    }
    if scale > 0 {
        output.push('.');
        let groups = scale.div_ceil(4);
        let mut fraction = String::with_capacity(groups * 4);
        for group in 0..groups {
            let index = weight as isize + 1 + group as isize;
            let value = if index >= 0 {
                values.get(index as usize).copied().unwrap_or(0)
            } else {
                0
            };
            fraction.push_str(&format!("{value:04}"));
        }
        output.push_str(&fraction[..scale]);
    }
    Ok(output)
}

fn decode_array(raw: &[u8], member: &Type, decode: DecodeMode) -> Result<Value, Error> {
    let mut cursor = BinaryCursor::new(raw);
    let dimension_count = cursor.i32()?;
    let _flags = cursor.i32()?;
    let element_oid = cursor.u32()?;
    if dimension_count < 0 || element_oid != member.oid() {
        return Err(Error::new("postgres/query", "invalid ARRAY header"));
    }
    let dimensions = (0..dimension_count)
        .map(|_| Ok((cursor.i32()?, cursor.i32()?)))
        .map(|result: Result<(i32, i32), Error>| result.map(|(length, lower)| (lower, length)))
        .collect::<Result<Vec<_>, _>>()?;
    if dimensions.iter().any(|(_, length)| *length < 0) {
        return Err(Error::new("postgres/query", "invalid ARRAY dimension"));
    }
    let total = dimensions.iter().try_fold(1usize, |total, (_, length)| {
        total
            .checked_mul(*length as usize)
            .ok_or_else(|| Error::new("postgres/query", "ARRAY is too large"))
    })?;
    let total = if dimensions.is_empty() { 0 } else { total };
    let mut flat = Vec::with_capacity(total);
    for _ in 0..total {
        let length = cursor.i32()?;
        if length == -1 {
            flat.push(Value::Nil);
        } else if length < -1 {
            return Err(Error::new("postgres/query", "invalid ARRAY element length"));
        } else {
            flat.push(decode_raw(member, cursor.take(length as usize)?, decode)?);
        }
    }
    if cursor.remaining() != 0 {
        return Err(Error::new("postgres/query", "trailing ARRAY data"));
    }
    let mut values = flat.into_iter();
    let nested = nest_array(&mut values, &dimensions);
    if decode == DecodeMode::Tagged || dimensions.iter().any(|(lower, _)| *lower != 1) {
        Ok(array_tag(member, &dimensions, nested))
    } else {
        Ok(nested)
    }
}

fn nest_array(values: &mut impl Iterator<Item = Value>, dimensions: &[(i32, i32)]) -> Value {
    let Some((_, length)) = dimensions.first() else {
        return Value::Vector(Vec::new());
    };
    Value::Vector(
        (0..*length)
            .map(|_| {
                if dimensions.len() == 1 {
                    values.next().unwrap_or(Value::Nil)
                } else {
                    nest_array(values, &dimensions[1..])
                }
            })
            .collect(),
    )
}

fn numeric_parameter_text(value: &Value) -> Result<String, Error> {
    match value {
        Value::Integer(value) => Ok(value.to_string()),
        Value::Float(value) if value.is_nan() => Ok("NaN".into()),
        Value::Float(value) if *value == f64::INFINITY => Ok("Infinity".into()),
        Value::Float(value) if *value == f64::NEG_INFINITY => Ok("-Infinity".into()),
        Value::Float(value) => Ok(value.to_string()),
        Value::Decimal(value) | Value::String(value) => Ok(value.clone()),
        _ => Err(Error::new(
            "postgres/type-unsupported",
            "NUMERIC requires a number or decimal string",
        )),
    }
}

fn array_type(name: &str) -> Result<Type, Error> {
    Ok(match name {
        "bool" | "boolean" => Type::BOOL_ARRAY,
        "int2" | "smallint" => Type::INT2_ARRAY,
        "int4" | "int" | "integer" => Type::INT4_ARRAY,
        "int8" | "long" | "bigint" => Type::INT8_ARRAY,
        "float4" | "real" => Type::FLOAT4_ARRAY,
        "float8" | "double" | "double-precision" => Type::FLOAT8_ARRAY,
        "numeric" | "decimal" => Type::NUMERIC_ARRAY,
        "text" => Type::TEXT_ARRAY,
        "varchar" => Type::VARCHAR_ARRAY,
        "bpchar" | "char" => Type::BPCHAR_ARRAY,
        "name" => Type::NAME_ARRAY,
        "uuid" => Type::UUID_ARRAY,
        "json" => Type::JSON_ARRAY,
        "jsonb" => Type::JSONB_ARRAY,
        "bytea" | "bytes" => Type::BYTEA_ARRAY,
        "date" => Type::DATE_ARRAY,
        "time" => Type::TIME_ARRAY,
        "timestamp" => Type::TIMESTAMP_ARRAY,
        "timestamptz" => Type::TIMESTAMPTZ_ARRAY,
        _ => return Err(Error::new("postgres/type-unsupported", name)),
    })
}

fn tagged_dimensions(value: Option<&Value>) -> Result<Option<Vec<(i32, i32)>>, Error> {
    let Some(value) = value else {
        return Ok(None);
    };
    if matches!(value, Value::Nil) {
        return Ok(None);
    }
    let dimensions = expect_vector(Some(value), "array dimensions")?
        .iter()
        .map(|dimension| {
            let pair = expect_vector(Some(dimension), "array dimension")?;
            if pair.len() != 2 {
                return Err(Error::new(
                    "postgres/config-invalid",
                    "array dimensions must be [lower-bound length] pairs",
                ));
            }
            let lower = i32::try_from(expect_i64(pair.first(), "array lower bound")?)
                .map_err(|_| Error::new("postgres/config-invalid", "array lower bound overflow"))?;
            let length = i32::try_from(expect_i64(pair.get(1), "array length")?)
                .map_err(|_| Error::new("postgres/config-invalid", "array length overflow"))?;
            if length < 0 {
                return Err(Error::new(
                    "postgres/config-invalid",
                    "array length must be non-negative",
                ));
            }
            Ok((lower, length))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(dimensions))
}

fn array_parameter_text(
    value: &Value,
    member: &Type,
    declared: Option<&[(i32, i32)]>,
) -> Result<String, Error> {
    let shape = array_shape(value)?;
    let dimensions = match declared {
        Some(dimensions) => {
            if dimensions.len() != shape.len()
                || dimensions
                    .iter()
                    .zip(shape.iter())
                    .any(|((_, length), actual)| *length as usize != *actual)
            {
                return Err(Error::new(
                    "postgres/config-invalid",
                    "array dimensions do not match value",
                ));
            }
            dimensions.to_vec()
        }
        None => shape
            .iter()
            .map(|length| (1, *length as i32))
            .collect::<Vec<_>>(),
    };
    let mut output = String::new();
    if dimensions.iter().any(|(lower, _)| *lower != 1) {
        for (lower, length) in &dimensions {
            let upper = lower
                .checked_add(*length)
                .and_then(|value| value.checked_sub(1))
                .ok_or_else(|| Error::new("postgres/config-invalid", "array bound overflow"))?;
            output.push_str(&format!("[{lower}:{upper}]"));
        }
        output.push('=');
    }
    output.push_str(&array_literal(value, member, 0, dimensions.len())?);
    Ok(output)
}

fn array_shape(value: &Value) -> Result<Vec<usize>, Error> {
    let Value::Vector(values) = value else {
        return Err(Error::new(
            "postgres/config-invalid",
            "array value must be a vector",
        ));
    };
    if values.is_empty() {
        return Ok(vec![0]);
    }
    let child_shapes = values
        .iter()
        .filter_map(|value| matches!(value, Value::Vector(_)).then(|| array_shape(value)))
        .collect::<Result<Vec<_>, _>>()?;
    if !child_shapes.is_empty() {
        if child_shapes.len() != values.len()
            || child_shapes
                .iter()
                .skip(1)
                .any(|shape| shape != &child_shapes[0])
        {
            return Err(Error::new(
                "postgres/config-invalid",
                "array value must be rectangular",
            ));
        }
        let mut shape = vec![values.len()];
        shape.extend_from_slice(&child_shapes[0]);
        Ok(shape)
    } else {
        Ok(vec![values.len()])
    }
}

fn array_literal(
    value: &Value,
    member: &Type,
    depth: usize,
    dimensions: usize,
) -> Result<String, Error> {
    let Value::Vector(values) = value else {
        return Err(Error::new(
            "postgres/config-invalid",
            "array nesting mismatch",
        ));
    };
    let mut output = String::from("{");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        if depth + 1 < dimensions {
            output.push_str(&array_literal(value, member, depth + 1, dimensions)?);
        } else {
            output.push_str(&array_element_text(value, member)?);
        }
    }
    output.push('}');
    Ok(output)
}

fn array_element_text(value: &Value, ty: &Type) -> Result<String, Error> {
    if matches!(value, Value::Nil) {
        return Ok("NULL".into());
    }
    let text = match *ty {
        Type::BOOL => match value {
            Value::Boolean(value) => value.to_string(),
            _ => return Err(type_error(ty)),
        },
        Type::INT2 | Type::INT4 | Type::INT8 => match value {
            Value::Integer(value) => value.to_string(),
            _ => return Err(type_error(ty)),
        },
        Type::FLOAT4 | Type::FLOAT8 => match value {
            Value::Integer(value) => value.to_string(),
            Value::Float(value) => value.to_string(),
            _ => return Err(type_error(ty)),
        },
        Type::NUMERIC => {
            let value = match postgres_tag(value) {
                Some(("numeric", fields)) => fields.get("value").ok_or_else(|| {
                    Error::new("postgres/config-invalid", "numeric value is required")
                })?,
                Some(_) => return Err(type_error(ty)),
                None => value,
            };
            numeric_parameter_text(value)?
        }
        Type::TEXT
        | Type::VARCHAR
        | Type::BPCHAR
        | Type::NAME
        | Type::UUID
        | Type::DATE
        | Type::TIME
        | Type::TIMESTAMP
        | Type::TIMESTAMPTZ => quote_array_text(expect_string(Some(value), "array element")?),
        Type::JSON | Type::JSONB => quote_array_text(&abi_json(value)?.to_string()),
        Type::BYTEA => match value {
            Value::Bytes(bytes) => {
                let mut encoded = String::from("\\x");
                for byte in bytes {
                    encoded.push_str(&format!("{byte:02x}"));
                }
                quote_array_text(&encoded)
            }
            _ => return Err(type_error(ty)),
        },
        _ => return Err(type_error(ty)),
    };
    Ok(text)
}

fn quote_array_text(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        if matches!(character, '\\' | '"') {
            output.push('\\');
        }
        output.push(character);
    }
    output.push('"');
    output
}

struct BinaryCursor<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl<'a> BinaryCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], Error> {
        let end = self
            .cursor
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| Error::new("postgres/query", "truncated binary value"))?;
        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }
    fn i16(&mut self) -> Result<i16, Error> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }
}

fn connection_config(options: &BTreeMap<String, Value>) -> Result<Config, Error> {
    let mut config = if let Some(Value::String(url)) = options.get("url") {
        url.parse::<Config>()
            .map_err(|_| Error::new("postgres/config-invalid", "invalid connection URL"))?
    } else {
        let mut config = Config::new();
        config.host(string_option(options, "host", "localhost"));
        config.port(
            integer_option(options, "port", 5432)
                .try_into()
                .map_err(|_| Error::new("postgres/config-invalid", "invalid port"))?,
        );
        let user = options
            .get("user")
            .and_then(string_value)
            .map(str::to_owned)
            .or_else(|| std::env::var("PGUSER").ok())
            .or_else(|| std::env::var("USER").ok())
            .ok_or_else(|| Error::new("postgres/config-invalid", "user is required"))?;
        config.user(&user);
        config.dbname(
            options
                .get("database")
                .or_else(|| options.get("dbname"))
                .and_then(string_value)
                .unwrap_or(&user),
        );
        if let Some(password) = options.get("password").and_then(string_value) {
            config.password(password);
        }
        config
    };
    let tls = options
        .get("tls")
        .and_then(string_value)
        .unwrap_or("prefer");
    config.ssl_mode(match tls {
        "disable" => SslMode::Disable,
        "require" => SslMode::Require,
        "prefer" => SslMode::Prefer,
        _ => {
            return Err(Error::new(
                "postgres/config-invalid",
                "tls must be disable, prefer or require",
            ))
        }
    });
    config.connect_timeout(Duration::from_millis(
        integer_option(options, "connect-timeout-ms", 10_000).max(1) as u64,
    ));
    Ok(config)
}

fn record<const N: usize>(values: [(&str, Value); N]) -> Value {
    Value::Record(
        values
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}
fn strings(values: &[&str]) -> Value {
    Value::Vector(
        values
            .iter()
            .map(|value| Value::String((*value).into()))
            .collect(),
    )
}
fn expect_record<'a>(
    value: Option<&'a Value>,
    name: &str,
) -> Result<&'a BTreeMap<String, Value>, Error> {
    match value {
        Some(Value::Record(value)) => Ok(value),
        _ => Err(Error::new(
            "postgres/config-invalid",
            format!("{name} must be a map"),
        )),
    }
}
fn expect_vector<'a>(value: Option<&'a Value>, name: &str) -> Result<&'a [Value], Error> {
    match value {
        Some(Value::Vector(value)) => Ok(value),
        _ => Err(Error::new(
            "postgres/config-invalid",
            format!("{name} must be a vector"),
        )),
    }
}
fn expect_string<'a>(value: Option<&'a Value>, name: &str) -> Result<&'a str, Error> {
    value.and_then(string_value).ok_or_else(|| {
        Error::new(
            "postgres/config-invalid",
            format!("{name} must be a string"),
        )
    })
}
fn expect_i64(value: Option<&Value>, name: &str) -> Result<i64, Error> {
    match value {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(Error::new(
            "postgres/config-invalid",
            format!("{name} must be an integer"),
        )),
    }
}
fn string_value(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value),
        Value::Keyword(value) => Some(value),
        _ => None,
    }
}
fn string_option<'a>(
    options: &'a BTreeMap<String, Value>,
    name: &str,
    fallback: &'a str,
) -> &'a str {
    options.get(name).and_then(string_value).unwrap_or(fallback)
}
fn integer_option(options: &BTreeMap<String, Value>, name: &str, fallback: i64) -> i64 {
    match options.get(name) {
        Some(Value::Integer(value)) => *value,
        _ => fallback,
    }
}
fn identifier(value: &str) -> Result<String, Error> {
    if value.is_empty()
        || value
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'))
    {
        Err(Error::new(
            "postgres/config-invalid",
            "invalid SQL identifier",
        ))
    } else {
        Ok(format!("\"{}\"", value.replace('"', "\"\"")))
    }
}
fn connect_error(_error: tokio_postgres::Error) -> Error {
    Error::new("postgres/connect", "connection failed")
}
fn query_error(error: tokio_postgres::Error) -> Error {
    let detail = error
        .as_db_error()
        .map(|db| format!("{} ({})", db.message(), db.code().code()))
        .unwrap_or_else(|| "database operation failed".into());
    Error::new("postgres/query", detail)
}
fn type_error(ty: &Type) -> Error {
    Error::new("postgres/type-unsupported", ty.name())
}

fn abi_json(value: &Value) -> Result<serde_json::Value, Error> {
    Ok(match value {
        Value::Nil => serde_json::Value::Null,
        Value::Boolean(value) => (*value).into(),
        Value::Integer(value) => (*value).into(),
        Value::Float(value) => (*value).into(),
        Value::Decimal(value) | Value::String(value) | Value::Keyword(value) => {
            value.clone().into()
        }
        Value::Vector(values) => {
            serde_json::Value::Array(values.iter().map(abi_json).collect::<Result<Vec<_>, _>>()?)
        }
        Value::Record(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| Ok((key.clone(), abi_json(value)?)))
                .collect::<Result<_, Error>>()?,
        ),
        Value::Bytes(_) => {
            return Err(Error::new(
                "postgres/type-unsupported",
                "bytes cannot be JSON",
            ))
        }
    })
}
fn json_abi(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(value) => Value::Boolean(value),
        serde_json::Value::Number(value) => value
            .as_i64()
            .map(Value::Integer)
            .or_else(|| value.as_f64().map(Value::Float))
            .unwrap_or_else(|| Value::Decimal(value.to_string())),
        serde_json::Value::String(value) => Value::String(value),
        serde_json::Value::Array(values) => {
            Value::Vector(values.into_iter().map(json_abi).collect())
        }
        serde_json::Value::Object(values) => Value::Record(
            values
                .into_iter()
                .map(|(key, value)| (key, json_abi(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_capabilities_are_stable() {
        let module = PostgresModule::new().unwrap();
        assert_eq!(module.identity().export, "std.db.postgres");
        assert_eq!(module.identity().abi, "hara.db-provider/1");
        assert!(module.capabilities().contains(&"notifications"));
    }

    #[test]
    fn config_rejects_ambiguous_tls_without_leaking_passwords() {
        let options = BTreeMap::from([
            ("user".into(), Value::String("postgres".into())),
            ("password".into(), Value::String("secret".into())),
            ("tls".into(), Value::String("maybe".into())),
        ]);
        let error = connection_config(&options).unwrap_err();
        assert_eq!(error.code, "postgres/config-invalid");
        assert!(!error.detail.contains("secret"));
    }

    #[test]
    fn numeric_binary_decoding_and_simple_conversion_are_stable() {
        let raw = [
            0, 2, // ndigits
            0, 0, // weight
            0, 0, // positive
            0, 4, // scale
            0, 12, 13, 72, // 12, 3400
        ];
        assert_eq!(decode_numeric(&raw).unwrap(), "12.3400");
        assert_eq!(simple_numeric("12.000".into()), Value::Integer(12));
        assert_eq!(simple_numeric("12.3400".into()), Value::Float(12.34));
        assert_eq!(simple_numeric("1e10000".into()), numeric_tag("1e10000"));
    }

    #[test]
    fn array_text_preserves_dimensions_and_rejects_ragged_values() {
        let value = Value::Vector(vec![
            Value::Vector(vec![Value::Integer(1), Value::Nil]),
            Value::Vector(vec![Value::Integer(2), Value::Integer(3)]),
        ]);
        assert_eq!(
            array_parameter_text(&value, &Type::INT8, Some(&[(0, 2), (1, 2)])).unwrap(),
            "[0:1][1:2]={{1,NULL},{2,3}}"
        );
        let ragged = Value::Vector(vec![
            Value::Vector(vec![Value::Integer(1)]),
            Value::Vector(vec![Value::Integer(2), Value::Integer(3)]),
        ]);
        assert_eq!(
            array_parameter_text(&ragged, &Type::INT8, None)
                .unwrap_err()
                .code,
            "postgres/config-invalid"
        );
    }
}
