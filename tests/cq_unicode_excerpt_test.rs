use std::sync::Arc;

use open_ontologies::cq::{CompetencyQuestion, run_cq_suite};
use open_ontologies::graph::GraphStore;

const QUERY: &str = "SELECT ?description WHERE { ?product <http://www.w3.org/2000/01/rdf-schema#comment> ?description }";

fn query_and_report(description: &str) -> (String, String) {
    let graph = Arc::new(GraphStore::new());
    graph
        .load_turtle(
            &format!(
                "<https://example.org/product> <http://www.w3.org/2000/01/rdf-schema#comment> {} .",
                serde_json::to_string(description).unwrap()
            ),
            None,
        )
        .unwrap();
    let query = graph.sparql_select(QUERY).unwrap();
    let query_result: serde_json::Value = serde_json::from_str(&query).unwrap();
    assert_eq!(query_result["results"].as_array().unwrap().len(), 1);
    let questions = [CompetencyQuestion {
        id: "product-description".into(),
        question: "What description is available for each product?".into(),
        sparql: QUERY.into(),
        expected_min_rows: Some(1),
    }];
    let report = run_cq_suite(&graph, &questions);
    assert_eq!(report.total, 1);
    assert_eq!(report.passed, 1);
    assert_eq!(report.results[0].row_count, 1);
    assert!(report.results[0].passed);
    (query, report.results[0].raw_excerpt.clone())
}

#[test]
fn long_multilingual_results_return_a_readable_excerpt_and_normal_counts() {
    let paragraph = "この製品は日常の配送業務で利用する標準的な資材です。担当者は商品番号と数量を確認してから出荷予定を記録します。保管場所と配送先の名称は注文書に記載されています。説明文は利用者が商品を選択するときの参考情報として表示されます。";
    for description in [paragraph.repeat(4), "Delivery 配送 📦 ".repeat(100)] {
        let (query, excerpt) = query_and_report(&description);
        assert!(query.len() > 800);
        let prefix = excerpt.strip_suffix("...").unwrap();
        assert!(prefix.len() <= 800);
        assert!(query.starts_with(prefix));
        let remainder = &query[prefix.len()..];
        assert!(prefix.len() + remainder.chars().next().unwrap().len_utf8() > 800);
        assert!(prefix.contains("配送"));
    }
}

#[test]
fn ascii_results_keep_the_existing_exact_800_byte_excerpt() {
    let description =
        "This product is a standard material for daily shipping operations. ".repeat(20);
    let (query, excerpt) = query_and_report(&description);
    assert_eq!(excerpt, format!("{}...", &query[..800]));
}

#[test]
fn short_multilingual_results_are_returned_without_truncation() {
    let (query, excerpt) = query_and_report("通常の配送資材です。");
    assert!(query.len() <= 800);
    assert_eq!(excerpt, query);
}
