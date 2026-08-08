use super::halc::{decode_halc, HalcOrigin};
use std::collections::BTreeMap;

pub const HALC_TRACE_SCHEMA: &str = "hal.halc-trace/v1";

const MAGIC_BYTES: usize = 4;
const VERSION_OFFSET: usize = MAGIC_BYTES;
const FLAGS_OFFSET: usize = VERSION_OFFSET + 2;
const PAYLOAD_LENGTH_OFFSET: usize = FLAGS_OFFSET + 2;
const CHECKSUM_OFFSET: usize = PAYLOAD_LENGTH_OFFSET + 4;
const CHECKSUM_BYTES: usize = 32;
const HEADER_BYTES: usize = CHECKSUM_OFFSET + CHECKSUM_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalcTraceStatus {
    Ok,
    Error,
}

impl HalcTraceStatus {
    pub fn as_keyword(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalcTraceValue {
    String(String),
    Integer(u64),
    Boolean(bool),
    Strings(Vec<String>),
}

pub type HalcTraceEvidence = BTreeMap<String, HalcTraceValue>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HalcTraceEvent {
    pub id: u64,
    pub sequence: u64,
    pub stage: &'static str,
    pub status: HalcTraceStatus,
    pub evidence: HalcTraceEvidence,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HalcArtifactTrace {
    pub schema: &'static str,
    pub id: String,
    pub status: HalcTraceStatus,
    pub events: Vec<HalcTraceEvent>,
    pub result: Option<HalcTraceEvidence>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HalcArtifactInspection {
    pub origin: HalcOrigin,
    pub format_version: u16,
    pub flags: u16,
    pub payload_length: u32,
    pub payload_checksum: String,
    pub namespace: String,
    pub resource: String,
    pub source_hash: String,
    pub form_count: usize,
    pub schema_definitions: Vec<String>,
    pub schema_functions: Vec<String>,
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn origin_name(origin: HalcOrigin) -> &'static str {
    match origin {
        HalcOrigin::Halc => "halc",
        HalcOrigin::LegacyHir => "legacy-hir",
    }
}

pub fn inspect_halc_artifact(bytes: &[u8]) -> Result<HalcArtifactInspection, String> {
    let module = decode_halc(bytes)?;
    if bytes.len() < HEADER_BYTES {
        return Err("truncated artifact".into());
    }

    let mut schema_definitions: Vec<String> =
        module.schemas.definitions.keys().cloned().collect();
    schema_definitions.sort();
    let mut schema_functions: Vec<String> = module.schemas.functions.keys().cloned().collect();
    schema_functions.sort();

    Ok(HalcArtifactInspection {
        origin: module.origin,
        format_version: read_u16(bytes, VERSION_OFFSET),
        flags: read_u16(bytes, FLAGS_OFFSET),
        payload_length: read_u32(bytes, PAYLOAD_LENGTH_OFFSET),
        payload_checksum: hex(&bytes[CHECKSUM_OFFSET..HEADER_BYTES]),
        namespace: module.namespace,
        resource: module.resource,
        source_hash: hex(&module.source_hash),
        form_count: module.forms.len(),
        schema_definitions,
        schema_functions,
    })
}

fn string(value: impl Into<String>) -> HalcTraceValue {
    HalcTraceValue::String(value.into())
}

fn integer(value: usize) -> HalcTraceValue {
    HalcTraceValue::Integer(value as u64)
}

fn event(
    id: u64,
    stage: &'static str,
    status: HalcTraceStatus,
    evidence: HalcTraceEvidence,
    error: Option<String>,
) -> HalcTraceEvent {
    HalcTraceEvent {
        id,
        sequence: id,
        stage,
        status,
        evidence,
        error,
    }
}

fn successful_trace(id: String, inspection: HalcArtifactInspection) -> HalcArtifactTrace {
    let mut module_identity = HalcTraceEvidence::new();
    module_identity.insert("module/namespace".into(), string(&inspection.namespace));
    module_identity.insert("module/resource".into(), string(&inspection.resource));
    module_identity.insert("source/hash".into(), string(&inspection.source_hash));
    module_identity.insert("form/count".into(), integer(inspection.form_count));

    let mut schema_index = HalcTraceEvidence::new();
    schema_index.insert(
        "schema/definitions".into(),
        HalcTraceValue::Strings(inspection.schema_definitions.clone()),
    );
    schema_index.insert(
        "schema/functions".into(),
        HalcTraceValue::Strings(inspection.schema_functions.clone()),
    );

    let mut envelope = HalcTraceEvidence::new();
    envelope.insert("artifact/origin".into(), string(origin_name(inspection.origin)));
    envelope.insert(
        "format/version".into(),
        HalcTraceValue::Integer(inspection.format_version as u64),
    );
    envelope.insert(
        "format/flags".into(),
        HalcTraceValue::Integer(inspection.flags as u64),
    );
    envelope.insert(
        "payload/bytes".into(),
        HalcTraceValue::Integer(inspection.payload_length as u64),
    );
    envelope.insert(
        "payload/checksum".into(),
        string(&inspection.payload_checksum),
    );

    let mut validation = HalcTraceEvidence::new();
    validation.insert("artifact/valid".into(), HalcTraceValue::Boolean(true));
    validation.insert(
        "payload/checksum".into(),
        string(&inspection.payload_checksum),
    );

    let mut decoded = HalcTraceEvidence::new();
    decoded.insert("decode/parity".into(), HalcTraceValue::Boolean(true));
    decoded.insert("form/count".into(), integer(inspection.form_count));

    let events = vec![
        event(1, "module/identity", HalcTraceStatus::Ok, module_identity.clone(), None),
        event(2, "schema/index", HalcTraceStatus::Ok, schema_index.clone(), None),
        event(3, "envelope/build", HalcTraceStatus::Ok, envelope.clone(), None),
        event(4, "artifact/validate", HalcTraceStatus::Ok, validation, None),
        event(5, "artifact/decode", HalcTraceStatus::Ok, decoded, None),
    ];

    let mut result = module_identity;
    result.extend(schema_index);
    result.extend(envelope);
    result.insert("decode/parity".into(), HalcTraceValue::Boolean(true));

    HalcArtifactTrace {
        schema: HALC_TRACE_SCHEMA,
        id,
        status: HalcTraceStatus::Ok,
        events,
        result: Some(result),
        error: None,
    }
}

fn failed_trace(id: String, bytes: &[u8], error: String) -> HalcArtifactTrace {
    let mut evidence = HalcTraceEvidence::new();
    evidence.insert("artifact/bytes".into(), integer(bytes.len()));
    let failure = event(
        1,
        "artifact/validate",
        HalcTraceStatus::Error,
        evidence,
        Some(error.clone()),
    );
    HalcArtifactTrace {
        schema: HALC_TRACE_SCHEMA,
        id,
        status: HalcTraceStatus::Error,
        events: vec![failure],
        result: None,
        error: Some(error),
    }
}

pub fn trace_halc_artifact(id: impl Into<String>, bytes: &[u8]) -> HalcArtifactTrace {
    let id = id.into();
    let id = if id.is_empty() {
        "halc-artifact".to_owned()
    } else {
        id
    };
    match inspect_halc_artifact(bytes) {
        Ok(inspection) => successful_trace(id, inspection),
        Err(error) => failed_trace(id, bytes, error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::{halc::encode_halc_module, parse_forms};

    fn artifact() -> Vec<u8> {
        let source = "(ns demo.schema) \
                      (def Customer [:map [:id :int]]) \
                      (defn ^{:schema #'-/Customer} customer-id [customer] customer)";
        encode_halc_module(
            "demo.schema",
            "demo/schema.hal",
            source,
            parse_forms(source).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn inspects_and_traces_the_production_halc_artifact() {
        let artifact = artifact();
        let inspection = inspect_halc_artifact(&artifact).unwrap();
        assert_eq!(inspection.origin, HalcOrigin::Halc);
        assert_eq!(inspection.format_version, 1);
        assert_eq!(inspection.flags, 1);
        assert_eq!(inspection.namespace, "demo.schema");
        assert_eq!(inspection.resource, "demo/schema.hal");
        assert_eq!(inspection.form_count, 3);
        assert_eq!(
            inspection.schema_definitions,
            vec!["demo.schema/Customer".to_owned()]
        );
        assert_eq!(
            inspection.schema_functions,
            vec!["demo.schema/customer-id".to_owned()]
        );

        let trace = trace_halc_artifact("trace-1", &artifact);
        assert_eq!(trace.schema, HALC_TRACE_SCHEMA);
        assert_eq!(trace.status, HalcTraceStatus::Ok);
        assert_eq!(
            trace.events.iter().map(|event| event.stage).collect::<Vec<_>>(),
            vec![
                "module/identity",
                "schema/index",
                "envelope/build",
                "artifact/validate",
                "artifact/decode",
            ]
        );
        assert_eq!(
            trace.result.as_ref().unwrap().get("module/namespace"),
            Some(&HalcTraceValue::String("demo.schema".to_owned()))
        );
        assert_eq!(
            trace.result.as_ref().unwrap().get("decode/parity"),
            Some(&HalcTraceValue::Boolean(true))
        );
    }

    #[test]
    fn classifies_legacy_hir_without_weakening_validation() {
        let mut artifact = artifact();
        artifact[..MAGIC_BYTES].copy_from_slice(b"HIR\0");
        let inspection = inspect_halc_artifact(&artifact).unwrap();
        assert_eq!(inspection.origin, HalcOrigin::LegacyHir);
        assert_eq!(inspection.format_version, 1);
        assert_eq!(inspection.flags, 1);
    }

    #[test]
    fn invalid_artifacts_become_precise_validation_failure_traces() {
        let trace = trace_halc_artifact("bad", b"NOPE");
        assert_eq!(trace.status, HalcTraceStatus::Error);
        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.events[0].stage, "artifact/validate");
        assert_eq!(trace.events[0].status, HalcTraceStatus::Error);
        assert_eq!(trace.events[0].error.as_deref(), Some("bad magic"));
        assert_eq!(trace.error.as_deref(), Some("bad magic"));
        assert!(trace.result.is_none());
    }
}
