use hara_abi::{NativeModule, TaskEvent, Value};
use hara_db_postgres::PostgresModule;
use std::collections::BTreeMap;

fn call(module: &PostgresModule, operation: &str, arguments: Vec<Value>) -> Value {
    let task = module.start(operation, arguments).unwrap();
    let event = module.wait(task, None).unwrap();
    module.drop_task(task);
    match event {
        TaskEvent::Resolved(value) => value,
        TaskEvent::Rejected(error) => panic!("{}: {}", error.code, error.detail),
        TaskEvent::Pending => panic!("pending task after wait"),
    }
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Record(value) = value else {
        panic!("record")
    };
    value.get(name).unwrap()
}

#[test]
fn live_postgres_query_transaction_and_notifications() {
    let Ok(url) = std::env::var("HARA_TEST_POSTGRES_URL") else {
        return;
    };
    let module = PostgresModule::new().unwrap();
    let opened = call(
        &module,
        "open",
        vec![Value::Record(BTreeMap::from([
            ("url".into(), Value::String(url)),
            ("tls".into(), Value::String("disable".into())),
        ]))],
    );
    let Value::Integer(connection) = field(&opened, "id") else {
        panic!("id")
    };
    let connection = *connection;

    let exec = |sql: &str, parameters: Vec<Value>| {
        call(
            &module,
            "exec",
            vec![
                Value::Integer(connection),
                Value::String(sql.into()),
                Value::Vector(parameters),
            ],
        )
    };
    let query = |sql: &str, parameters: Vec<Value>| {
        call(
            &module,
            "query",
            vec![
                Value::Integer(connection),
                Value::String(sql.into()),
                Value::Vector(parameters),
            ],
        )
    };
    let query_tagged = |sql: &str, parameters: Vec<Value>| {
        call(
            &module,
            "query",
            vec![
                Value::Integer(connection),
                Value::String(sql.into()),
                Value::Vector(parameters),
                Value::Record(BTreeMap::from([(
                    "decode".into(),
                    Value::Keyword("tagged".into()),
                )])),
            ],
        )
    };

    exec(
        "create temporary table hara_items (id int primary key, name text not null)",
        vec![],
    );
    exec(
        "insert into hara_items (id, name) values ($1, $2)",
        vec![Value::Integer(1), Value::String("wombat".into())],
    );
    let result = query(
        "select id, name from hara_items where id = $1",
        vec![Value::Integer(1)],
    );
    assert_eq!(
        field(&result, "rows"),
        &Value::Vector(vec![Value::Vector(vec![
            Value::Integer(1),
            Value::String("wombat".into())
        ])])
    );

    exec("begin", vec![]);
    exec(
        "insert into hara_items (id, name) values ($1, $2)",
        vec![Value::Integer(2), Value::String("rollback".into())],
    );
    exec("rollback", vec![]);
    let result = query("select count(*) from hara_items", vec![]);
    assert_eq!(
        field(&result, "rows"),
        &Value::Vector(vec![Value::Vector(vec![Value::Integer(1)])])
    );

    let simple = query(
        "select 12.3400::numeric, array[1,2,null]::int8[], '[0:2]={4,5,6}'::int8[], array[[1,2],[3,4]]::int4[]",
        vec![],
    );
    let Value::Vector(simple_rows) = field(&simple, "rows") else {
        panic!("rows")
    };
    let Value::Vector(simple_row) = &simple_rows[0] else {
        panic!("row")
    };
    assert_eq!(simple_row[0], Value::Float(12.34));
    assert_eq!(
        simple_row[1],
        Value::Vector(vec![Value::Integer(1), Value::Integer(2), Value::Nil])
    );
    assert_eq!(
        field(&simple_row[2], "$postgres"),
        &Value::String("array".into())
    );
    assert_eq!(
        simple_row[3],
        Value::Vector(vec![
            Value::Vector(vec![Value::Integer(1), Value::Integer(2)]),
            Value::Vector(vec![Value::Integer(3), Value::Integer(4)]),
        ])
    );

    let tagged = query_tagged(
        "select 12.3400::numeric, array[1,2]::numeric[], array[]::int8[]",
        vec![],
    );
    let Value::Vector(tagged_rows) = field(&tagged, "rows") else {
        panic!("rows")
    };
    let Value::Vector(tagged_row) = &tagged_rows[0] else {
        panic!("row")
    };
    assert_eq!(
        field(&tagged_row[0], "$postgres"),
        &Value::String("numeric".into())
    );
    assert_eq!(
        field(&tagged_row[0], "value"),
        &Value::String("12.3400".into())
    );
    assert_eq!(
        field(&tagged_row[1], "$postgres"),
        &Value::String("array".into())
    );
    assert_eq!(
        field(&tagged_row[2], "dimensions"),
        &Value::Vector(Vec::new())
    );

    let numeric_parameter = query_tagged(
        "select $1",
        vec![Value::Record(BTreeMap::from([
            ("$postgres".into(), Value::String("numeric".into())),
            ("value".into(), Value::String("9007199254740993.25".into())),
        ]))],
    );
    let Value::Vector(rows) = field(&numeric_parameter, "rows") else {
        panic!("rows")
    };
    let Value::Vector(row) = &rows[0] else {
        panic!("row")
    };
    assert_eq!(
        field(&row[0], "value"),
        &Value::String("9007199254740993.25".into())
    );

    let array_parameter = query_tagged(
        "select $1",
        vec![Value::Record(BTreeMap::from([
            ("$postgres".into(), Value::String("array".into())),
            ("element".into(), Value::String("int8".into())),
            (
                "dimensions".into(),
                Value::Vector(vec![Value::Vector(vec![
                    Value::Integer(0),
                    Value::Integer(3),
                ])]),
            ),
            (
                "value".into(),
                Value::Vector(vec![
                    Value::Integer(7),
                    Value::Integer(8),
                    Value::Integer(9),
                ]),
            ),
        ]))],
    );
    let Value::Vector(rows) = field(&array_parameter, "rows") else {
        panic!("rows")
    };
    let Value::Vector(row) = &rows[0] else {
        panic!("row")
    };
    assert_eq!(
        field(&row[0], "dimensions"),
        &Value::Vector(vec![Value::Vector(vec![
            Value::Integer(0),
            Value::Integer(3)
        ])])
    );

    let listened = call(
        &module,
        "listen",
        vec![
            Value::Integer(connection),
            Value::String("hara_items_changed".into()),
        ],
    );
    let Value::Integer(subscription) = field(&listened, "id") else {
        panic!("subscription")
    };
    let notification_task = module
        .start("notification-next", vec![Value::Integer(*subscription)])
        .unwrap();
    call(
        &module,
        "notify",
        vec![
            Value::Integer(connection),
            Value::String("hara_items_changed".into()),
            Value::String("wombat".into()),
        ],
    );
    let event = module.wait(notification_task, None).unwrap();
    module.drop_task(notification_task);
    let TaskEvent::Resolved(notification) = event else {
        panic!("notification")
    };
    assert_eq!(
        field(&notification, "payload"),
        &Value::String("wombat".into())
    );

    assert_eq!(
        call(&module, "close", vec![Value::Integer(connection)]),
        Value::Boolean(true)
    );
}
