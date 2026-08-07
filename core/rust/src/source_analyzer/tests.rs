const TEST_ANALYZER: &str = r#"
  (defn ^{:schema [:fn [] :any]}
    describe
    []
    ["test-hara-analyzer" "0.1.0" ["clojure"] [".clj"] ["symbols"] 1048576])

  (defn ^{:schema [:fn [:any] :any]}
    analyze
    [tree]
    [-1 [] [] []])

  0
"#;

#[test]
fn direct_value_worker_compiles_and_materializes() {
    let mut analyzer = SourceAnalyzer::compile(TEST_ANALYZER).expect("compile analyzer");
    let request = crate::json::read(
        r#"{"protocol_version":"1.0","request_id":"x","op":"analyze","language":"clojure","path":"x.clj","blob_oid":"abc","source":"(def x 1)"}"#,
    )
    .unwrap();
    let response = analyzer.handle(&request);
    assert_eq!(
        response
            .object_field("result")
            .and_then(|result| result.object_field("file"))
            .and_then(|file| file.object_field("path"))
            .and_then(Json::as_str),
        Some("x.clj")
    );
}

#[test]
fn source_index_uses_utf16_display_columns() {
    let index = SourceIndex::new("😀x");
    assert!(matches!(
        index.position("😀x", 4).object_field("column"),
        Some(Json::Integer(3))
    ));
}

#[test]
fn normalization_preserves_semicolons_inside_strings_and_characters() {
    assert_eq!(
        normalize_form("(defn x ; comment\n [] \"semi;colon\" \\; )"),
        "(defn x [] \"semi;colon\" \\; )"
    );
}

#[test]
fn collection_nodes_do_not_render_subtrees_during_indexing() {
    assert_eq!(token_text(&Form::List(vec![])), None);
    assert_eq!(token_text(&Form::Symbol("x".into())), Some("x".into()));
}

#[test]
fn definition_kind_codes_are_protocol_data_not_token_indexes() {
    assert_eq!(definition_kind(1), Ok("variable"));
    assert_eq!(definition_kind(9), Ok("test"));
    assert!(definition_kind(10).is_err());
}

#[test]
fn nested_list_heads_render_without_entering_the_descendant_set() {
    let shape = Shape::Vector(vec![
        Shape::Vector(vec![Shape::Keyword("call".into())]),
        Shape::Vector(vec![Shape::Keyword("string".into())]),
    ]);
    let summary = summarize_shape(&shape);
    assert_eq!(summary.rendered, "[[:call] [:string]]");
    assert_eq!(summary.node_count, 2);
    assert_eq!(summary.depth, 2);
    assert_eq!(
        summary.features,
        BTreeSet::from(["[[:call] [:string]]".into(), "[:string]".into()])
    );
}
