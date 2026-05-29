use std::path::PathBuf;

use radsuite_cite::{CsvReadingExtractionRequest, extract_csv_reading_candidates};
use radsuite_core::ReadingCategory;

#[test]
fn readings_csv_import_extracts_course_readings_inventory() {
    let path = write_csv(
        "course-readings-inventory.csv",
        r#"section_seq,section_title,week,citation,talis_article_id
5,Week 2 - Positivism,02,"""Biosocial Theories of Crime"" in Miller, M., Schreck, C. & Tewksbury, R. (2015). Criminological Theory: A Brief Introduction (4th ed.). Pearson.",26922
8,Week 5 - The Rise of Critical Criminology,05,"""Marxist, Postmodern and Green Criminology"" in Bernard, T.J., Snipes, J.B., Gerould, A.L., & Vold, G.B. (2019). Vold's Theoretical Criminology. (8th ed.). Oxford University Press. Pages 293-301",25805
"#,
    );

    let candidates = extract_csv_reading_candidates(CsvReadingExtractionRequest {
        path,
        original_filename: "course_readings.csv".to_string(),
    })
    .expect("extract csv readings");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].reading_category, ReadingCategory::Compulsory);
    assert_eq!(candidates[0].module_order, Some(2));
    assert_eq!(
        candidates[0].module_title.as_deref(),
        Some("Week 2 - Positivism")
    );
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("02"));
    assert_eq!(
        candidates[0].apa_citation,
        "\"Biosocial Theories of Crime\" in Miller, M., Schreck, C. & Tewksbury, R. (2015). Criminological Theory: A Brief Introduction (4th ed.). Pearson."
    );
    assert_eq!(candidates[0].citation_text, None);
    assert_eq!(candidates[0].url, None);

    assert_eq!(candidates[1].module_order, Some(5));
    assert_eq!(
        candidates[1].module_title.as_deref(),
        Some("Week 5 - The Rise of Critical Criminology")
    );
    assert_eq!(candidates[1].lesson_code.as_deref(), Some("05"));
}

#[test]
fn readings_csv_import_supports_alias_headers_categories_and_urls() {
    let path = write_csv(
        "course-readings-aliases.csv",
        r#"module_title,lesson,reading,reading_category,url
Module 3,3.1,"Taylor, R. (2023). Optional primer.",optional,https://example.com/primer
Module 3,3.1,"Taylor, R. (2023). Optional primer.",optional,https://example.com/primer
"#,
    );

    let candidates = extract_csv_reading_candidates(CsvReadingExtractionRequest {
        path,
        original_filename: "aliases.csv".to_string(),
    })
    .expect("extract csv readings");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].reading_category, ReadingCategory::Optional);
    assert_eq!(candidates[0].module_order, Some(3));
    assert_eq!(candidates[0].module_title.as_deref(), Some("Module 3"));
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("3.1"));
    assert_eq!(
        candidates[0].apa_citation,
        "Taylor, R. (2023). Optional primer."
    );
    assert_eq!(
        candidates[0].url.as_deref(),
        Some("https://example.com/primer")
    );
}

fn write_csv(filename: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("radsuite-{filename}"));
    std::fs::write(&path, contents).expect("write csv fixture");
    path
}
