use radsuite_desktop::library_links::build_uc_library_link;

#[test]
fn builds_an_openathens_link_from_a_doi() {
    assert_eq!(
        build_uc_library_link(Some("10.1080/1553118X.2022.2137674"), None),
        Some(
            "https://go.openathens.net/redirector/canterbury.ac.nz?url=https://doi.org/10.1080/1553118X.2022.2137674"
                .to_string(),
        ),
    );
}

#[test]
fn wraps_a_direct_url_for_uc_access() {
    assert_eq!(
        build_uc_library_link(None, Some("https://example.org/article?id=42")),
        Some(
            "https://go.openathens.net/redirector/canterbury.ac.nz?url=https://example.org/article?id=42"
                .to_string(),
        ),
    );
}

#[test]
fn removes_existing_proxy_wrappers_before_rebuilding_the_link() {
    assert_eq!(
        build_uc_library_link(
            None,
            Some(
                "https://go.openathens.net/redirector/canterbury.ac.nz?url=https://example.org/article%3Fid%3D42",
            ),
        ),
        Some(
            "https://go.openathens.net/redirector/canterbury.ac.nz?url=https://example.org/article?id=42"
                .to_string(),
        ),
    );
}

#[test]
fn returns_none_without_a_usable_source() {
    assert_eq!(build_uc_library_link(None, None), None);
    assert_eq!(build_uc_library_link(Some("doi:"), Some("  ")), None);
}
