use radsuite_cite::CitationAnalyzer;

#[test]
fn analyzer_detects_common_apa_citation_forms() {
    let analyzer = CitationAnalyzer;
    let analysis = analyzer.analyse_paragraph(
        "Smith (2020) frames the issue alongside later work (Jones & Lee, 2021; Zhu et al., 2022). Zhu et al. also note the limitation.",
    );

    let texts: Vec<&str> = analysis
        .citations
        .iter()
        .map(|citation| citation.text.as_str())
        .collect();

    assert_eq!(
        texts,
        vec![
            "Smith (2020)",
            "Jones & Lee, 2021",
            "Zhu et al., 2022",
            "Zhu et al.",
        ]
    );
    assert!(!analysis.needs_citation);
    assert!(
        analysis
            .citations
            .iter()
            .all(|citation| citation.start < citation.end)
    );
}

#[test]
fn analyzer_flags_claims_that_need_citations() {
    let analyzer = CitationAnalyzer;

    let statistics = analyzer.analyse_paragraph(
        "A 2021 survey reported that 64 percent of respondents changed their study habits.",
    );
    let plain = analyzer.analyse_paragraph("This paragraph introduces the next activity.");

    assert!(statistics.needs_citation);
    assert!(!plain.needs_citation);
}

#[test]
fn analyzer_extracts_search_keywords_without_external_models() {
    let analyzer = CitationAnalyzer;

    let keywords = analyzer.extract_keywords(
        "Working memory research uses working memory tasks and attention measures.",
        3,
    );

    assert_eq!(keywords, vec!["working", "memory", "research"]);
}
