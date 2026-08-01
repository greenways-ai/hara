use super::{parse_forms, read_forms};
use crate::kernel::Form;
use std::fs;
#[test]
fn tracks_spans_comments_commas_and_reader_macros() {
    let forms = read_forms("; hi\n[1, 2] #'x #_gone :ok").unwrap();
    assert_eq!(forms.len(), 3);
    assert_eq!(forms[0].span.start.line, 2);
    assert!(matches!(&forms[1].form,Form::List(v)if matches!(&v[0],Form::Symbol(s)if s=="var")));
    assert_eq!(forms[2].form, Form::Keyword("ok".into()));
}
#[test]
fn preserves_recursive_source_locations_without_changing_forms() {
    let mut forms = read_forms("  \n[1 {:a\n (f x)}]").unwrap();
    let root = forms.remove(0);
    assert_eq!(root.form.to_string(), "[1 {:a (f x)}]");
    assert_eq!((root.span.start.line, root.span.start.column), (2, 1));
    assert_eq!(root.children.len(), 2);

    let map = &root.children[1];
    assert_eq!(map.form.to_string(), "{:a (f x)}");
    assert_eq!(map.children.len(), 2);
    let call = &map.children[1];
    assert_eq!(call.form.to_string(), "(f x)");
    assert_eq!((call.span.start.line, call.span.start.column), (3, 2));
    assert_eq!(call.children.len(), 2);
    assert_eq!(root.descendants().count(), 6);
}

#[test]
fn preserves_locations_through_dispatch_and_metadata() {
    let root = read_forms("#tag ^:private [#'x ##Inf]").unwrap().remove(0);
    assert_eq!(root.form.to_string(), "#tag[(var x) ##Inf]");
    assert_eq!(root.children.len(), 1);
    let metadata = &root.children[0];
    assert_eq!(metadata.children.len(), 2);
    let vector = &metadata.children[1];
    assert_eq!(vector.children.len(), 2);
    assert_eq!(vector.children[0].form.to_string(), "(var x)");
    assert_eq!(vector.children[1].form.to_string(), "##Inf");
    assert_eq!(root.descendants().count(), 7);
}

#[test]
fn reports_delimited_errors_with_position() {
    let error = parse_forms("[1\n2").unwrap_err();
    assert!(error.contains("line 2"));
    assert!(error.contains("EOF while reading vector"));
}

#[test]
fn matches_canonical_numbers_characters_and_duplicate_errors() {
    assert_eq!(
        parse_forms("123 123.45 0xFF 2r1010").unwrap(),
        vec![
            Form::Number(123),
            Form::Float(123.45),
            Form::Number(255),
            Form::Number(10)
        ]
    );
    assert_eq!(
        parse_forms("123N 0N +0N -0N 123.45M 0M").unwrap(),
        vec![
            Form::BigInteger("123".into()),
            Form::BigInteger("0".into()),
            Form::BigInteger("+0".into()),
            Form::BigInteger("-0".into()),
            Form::Decimal("123.45".into()),
            Form::Decimal("0".into()),
        ]
    );
    for unsupported in ["9223372036854775808", "1.2.3M", "12xN"] {
        assert!(parse_forms(unsupported)
            .unwrap_err()
            .contains("Invalid number"));
    }
    assert_eq!(
        parse_forms("\\newline \\u03bb \\o377").unwrap(),
        vec![
            Form::Character('\n'),
            Form::Character('λ'),
            Form::Character('ÿ')
        ]
    );
    assert!(parse_forms("{:a 1 :a 2}")
        .unwrap_err()
        .contains("Duplicate key"));
    assert!(parse_forms("#{1 1}")
        .unwrap_err()
        .contains("Duplicate item"));
    assert!(parse_forms("123a").unwrap_err().contains("Invalid number"));
}

#[test]
fn preserves_regex_tagged_and_extended_string_literals() {
    assert_eq!(
        parse_forms("#\"abc\" #math[:tensor 42]").unwrap(),
        vec![
            Form::Regex("abc".into()),
            Form::Tagged(
                "math".into(),
                Box::new(Form::Vector(vec![
                    Form::Keyword("tensor".into()),
                    Form::Number(42)
                ]))
            )
        ]
    );
    assert_eq!(
        parse_forms("\"\\t\\r\\n\\b\\f\\\\\\\"\\7\"").unwrap(),
        vec![Form::String("\t\r\n\u{0008}\u{000c}\\\"\u{0007}".into())]
    );
}
#[test]
fn preserves_metadata_and_rejects_unknown_dispatch_forms() {
    assert_eq!(
        parse_forms("^:private [1]").unwrap(),
        vec![Form::Metadata(
            Box::new(Form::Map(vec![(
                Form::Keyword("private".into()),
                Form::Bool(true)
            )])),
            Box::new(Form::Vector(vec![Form::Number(1)]))
        )]
    );
    assert!(parse_forms("#[1 2]")
        .unwrap_err()
        .contains("No dispatch macro for: ["));
}
#[test]
fn matches_extended_canonical_reader_categories() {
    for unsupported in [
        "1.00M",
        "0M",
        "9223372036854775808",
        "123N",
        "0N",
        "+0N",
        "-0N",
    ] {
        assert!(parse_forms(unsupported)
            .unwrap_err()
            .contains("Invalid number"));
    }
    assert!(parse_forms("1/2")
        .unwrap_err()
        .contains("Ratios are not supported"));
    assert_eq!(
        parse_forms(r##"#"\d+""##).unwrap(),
        vec![Form::Regex(r"\d+".into())]
    );
    let symbolic = parse_forms("##Inf ##-Inf ##NaN").unwrap();
    assert!(matches!(symbolic[0], Form::Float(value) if value == f64::INFINITY));
    assert!(matches!(symbolic[1], Form::Float(value) if value == f64::NEG_INFINITY));
    assert!(matches!(symbolic[2], Form::Float(value) if value.is_nan()));
    assert!(parse_forms("#'1")
        .unwrap_err()
        .contains("Var quote expects a symbol"));
    assert!(parse_forms("^1 [2]")
        .unwrap_err()
        .contains("Metadata must be"));
    assert!(parse_forms("^:private 1")
        .unwrap_err()
        .contains("Metadata can only be applied"));
}
#[test]
fn allows_multi_slash_symbols_but_not_keywords_like_java() {
    assert_eq!(
        parse_forms("a/b/c").unwrap(),
        vec![Form::Symbol("a/b/c".into())]
    );
    assert!(parse_forms(":a/b/c").unwrap_err().contains("Keyword"));
}

#[test]
fn validates_keywords_and_merges_metadata_like_java() {
    for (invalid, expected) in [
        (":", "cannot be empty"),
        (":/", "single slash"),
        (":/name", "start with a slash"),
        (":name/", "end with a slash"),
        (":a/b/c", "only contain one slash"),
    ] {
        assert!(parse_forms(invalid).unwrap_err().contains(expected));
    }
    assert_eq!(
        parse_forms(":ns/name").unwrap(),
        vec![Form::Keyword("ns/name".into())]
    );
    assert_eq!(
        parse_forms("^:private ^{:tag fast} item").unwrap(),
        vec![Form::Metadata(
            Box::new(Form::Map(vec![
                (Form::Keyword("tag".into()), Form::Symbol("fast".into())),
                (Form::Keyword("private".into()), Form::Bool(true)),
            ])),
            Box::new(Form::Symbol("item".into())),
        )]
    );
    assert_eq!(
        parse_forms("^:ignored :keyword").unwrap(),
        vec![Form::Keyword("keyword".into())]
    );
}

#[test]
fn matches_java_malformed_reader_failures() {
    for (source, expected) in [
        (")", "Unmatched delimiter"),
        ("\"", "EOF while reading string"),
        ("\"\\u12X4\"", "Invalid digit"),
        ("\"\\q\"", "Unsupported escape character"),
        ("\\u12X4", "Invalid digit"),
        ("{:a 1 :b}", "even number of forms"),
        ("#", "EOF while reading hash dispatch"),
    ] {
        let error = parse_forms(source).unwrap_err();
        assert!(error.contains(expected), "{source}: {error}");
    }
}

#[test]
fn supports_dispatch_and_quote_forms() {
    let forms = parse_forms("#{1 2} 'x @y #tag {:a 1}").unwrap();
    assert_eq!(forms.len(), 4);
    assert!(matches!(forms[0], Form::Set(_)));
    assert!(matches!(&forms[3], Form::Tagged(tag, _) if tag == "tag"));
}
#[test]
fn rejects_dispatch_forms_absent_from_the_java_reference() {
    for source in [
        "#(inc %)",
        "#:hello{:a 1}",
        "#?(:clj hello)",
        "#?@(:clj [x])",
        "#=(f)",
        "[#|1]",
    ] {
        let error = parse_forms(source).unwrap_err();
        assert!(
            error.contains("No dispatch macro for:"),
            "{source}: {error}"
        );
    }
}

#[test]
fn matches_java_symbol_and_number_macro_termination() {
    assert_eq!(
        parse_forms("1#_2 3").unwrap(),
        vec![Form::Number(1), Form::Number(3)]
    );
    assert_eq!(
        parse_forms("1\u{27}foo").unwrap(),
        vec![
            Form::Number(1),
            Form::List(vec![
                Form::Symbol("quote".into()),
                Form::Symbol("foo".into())
            ])
        ]
    );
    assert_eq!(
        parse_forms("foo\u{27}bar").unwrap(),
        vec![Form::Symbol("foo\u{27}bar".into())]
    );
    assert_eq!(
        parse_forms("foo#bar 1").unwrap(),
        vec![Form::Symbol("foo#bar".into()), Form::Number(1)]
    );
    assert_eq!(
        parse_forms("foo@bar").unwrap(),
        vec![
            Form::Symbol("foo".into()),
            Form::List(vec![
                Form::Symbol("deref".into()),
                Form::Symbol("bar".into())
            ])
        ]
    );
    assert_eq!(
        parse_forms("foo^:tag [1]").unwrap(),
        vec![
            Form::Symbol("foo".into()),
            Form::Metadata(
                Box::new(Form::Map(vec![(
                    Form::Keyword("tag".into()),
                    Form::Bool(true)
                )])),
                Box::new(Form::Vector(vec![Form::Number(1)]))
            )
        ]
    );
}

#[test]
fn shared_reader_corpus_matches_canonical_forms_and_errors() {
    let Some(path) = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|root| root.join("specs/01-lang/001-language/draft/conformance/reader.edn"))
        .find(|candidate| candidate.is_file())
    else {
        eprintln!(
            "skipping: specs/01-lang/001-language/draft/conformance/reader.edn unavailable (specs submodule not initialized)"
        );
        return;
    };
    let manifest_source = fs::read_to_string(path).unwrap();
    let manifest = parse_forms(&manifest_source).unwrap().remove(0);
    let Form::Map(manifest) = manifest else {
        panic!("reader parity manifest must be a map");
    };
    let cases = map_value(&manifest, "cases").expect("manifest must contain :cases");
    let Form::Vector(cases) = cases else {
        panic!("reader parity :cases must be a vector");
    };

    for case in cases {
        let Form::Map(case) = case else {
            panic!("reader parity case must be a map");
        };
        let id = map_value(case, "id").expect("case must contain :id");
        let source = string_value(case, "source");
        let readable = map_value(case, "rust-readable").or_else(|| map_value(case, "readable"));
        let expected_error = map_value(case, "rust-error").or_else(|| map_value(case, "error"));
        match (readable, expected_error) {
            (Some(Form::String(readable)), None) => {
                let forms = parse_forms(source)
                    .unwrap_or_else(|error| panic!("{id} unexpectedly failed: {error}"));
                let actual = forms
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                assert_eq!(&actual, readable, "{id}");
                let round_trip = parse_forms(&actual)
                    .unwrap()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                assert_eq!(round_trip, actual, "{id} canonical output must round-trip");
            }
            (None, Some(Form::String(expected))) => {
                let error = match parse_forms(source) {
                    Ok(_) => panic!("{id} should fail"),
                    Err(error) => error,
                };
                assert!(
                    error.contains(expected),
                    "{id}: expected <{expected}> in <{error}>"
                );
            }
            _ => panic!("{id} must contain exactly one of :readable or :error"),
        }
    }
}

fn map_value<'a>(entries: &'a [(Form, Form)], name: &str) -> Option<&'a Form> {
    entries.iter().find_map(|(key, value)| match key {
        Form::Keyword(keyword) if keyword == name => Some(value),
        _ => None,
    })
}

fn string_value<'a>(entries: &'a [(Form, Form)], name: &str) -> &'a str {
    match map_value(entries, name) {
        Some(Form::String(value)) => value,
        _ => panic!("case must contain string :{name}"),
    }
}
