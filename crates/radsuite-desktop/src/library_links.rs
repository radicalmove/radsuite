use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

const OPENATHENS_PREFIX: &str = "https://go.openathens.net/redirector/canterbury.ac.nz?url=";
const PROXY_PREFIXES: [&str; 10] = [
    "http://go.openathens.net/redirector/canterbury.ac.nz?url=",
    OPENATHENS_PREFIX,
    "http://ezproxy.canterbury.ac.nz/login?url=",
    "https://ezproxy.canterbury.ac.nz/login?url=",
    "http://login.ezproxy.canterbury.ac.nz/login?url=",
    "https://login.ezproxy.canterbury.ac.nz/login?url=",
    "http://ezproxy.canterbury.ac.nz/login?qurl=",
    "https://ezproxy.canterbury.ac.nz/login?qurl=",
    "http://login.ezproxy.canterbury.ac.nz/login?qurl=",
    "https://login.ezproxy.canterbury.ac.nz/login?qurl=",
];

const URL_SAFE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b':')
    .remove(b'/')
    .remove(b'?')
    .remove(b'#')
    .remove(b'[')
    .remove(b']')
    .remove(b'@')
    .remove(b'!')
    .remove(b'$')
    .remove(b'&')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b'*')
    .remove(b'+')
    .remove(b',')
    .remove(b';')
    .remove(b'=')
    .remove(b'%');

/// Build the stable UC library link used by the original RADcite exports.
///
/// DOI values take precedence over ordinary URLs. Existing UC proxy wrappers
/// are removed before the canonical URL is wrapped again, which keeps repeated
/// exports from nesting redirects.
pub fn build_uc_library_link(doi: Option<&str>, url: Option<&str>) -> Option<String> {
    let canonical = doi.and_then(normalise_doi).or_else(|| {
        url.and_then(|value| {
            let unwrapped = unwrap_proxy(value.trim());
            is_http_url(&unwrapped).then(|| normalise_http_scheme(&unwrapped))
        })
    })?;

    Some(format!(
        "{OPENATHENS_PREFIX}{}",
        utf8_percent_encode(&canonical, URL_SAFE)
    ))
}

fn normalise_doi(value: &str) -> Option<String> {
    let mut doi = value.trim();
    if doi.is_empty() {
        return None;
    }

    if let Some((_, suffix)) = doi.to_ascii_lowercase().split_once("doi.org/") {
        let offset = doi.len() - suffix.len();
        doi = &doi[offset..];
    } else if doi
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("doi:"))
    {
        doi = doi.get(4..).unwrap_or_default().trim_start();
    }

    let doi = doi.trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '\'', '"']);
    let is_doi = regex::Regex::new(r"^10\.\d{4,9}/\S+$")
        .expect("DOI regex")
        .is_match(doi);
    is_doi.then(|| format!("https://doi.org/{doi}"))
}

fn unwrap_proxy(value: &str) -> String {
    let mut candidate = value.trim().to_string();
    if let Some(prefix) = PROXY_PREFIXES.iter().find(|prefix| {
        candidate
            .to_ascii_lowercase()
            .starts_with(&prefix.to_ascii_lowercase())
    }) {
        candidate = candidate[prefix.len()..].to_string();
    }

    percent_decode_str(&candidate)
        .decode_utf8_lossy()
        .into_owned()
}

fn is_http_url(value: &str) -> bool {
    value
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        || value
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
}

fn normalise_http_scheme(value: &str) -> String {
    value
        .get(..7)
        .filter(|prefix| prefix.eq_ignore_ascii_case("http://"))
        .map_or_else(
            || value.to_string(),
            |prefix| format!("https://{}", &value[prefix.len()..]),
        )
}

#[cfg(test)]
mod tests {
    use super::{normalise_doi, unwrap_proxy};

    #[test]
    fn normalises_doi_forms() {
        assert_eq!(
            normalise_doi("doi:10.1234/example"),
            Some("https://doi.org/10.1234/example".to_string())
        );
        assert_eq!(
            normalise_doi("https://doi.org/10.1234/example."),
            Some("https://doi.org/10.1234/example".to_string())
        );
        assert_eq!(normalise_doi("doi:"), None);
    }

    #[test]
    fn unwraps_openathens_and_ezproxy_links() {
        assert_eq!(
            unwrap_proxy(
                "https://go.openathens.net/redirector/canterbury.ac.nz?url=https://example.org/a%3Fb%3D1"
            ),
            "https://example.org/a?b=1"
        );
        assert_eq!(
            unwrap_proxy("https://ezproxy.canterbury.ac.nz/login?qurl=https%3A%2F%2Fexample.org"),
            "https://example.org"
        );
    }

    #[test]
    fn tolerates_non_ascii_doi_input_without_panicking() {
        assert_eq!(normalise_doi("é"), None);
    }
}
