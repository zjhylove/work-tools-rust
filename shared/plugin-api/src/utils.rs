/// Escape special characters for XML/HTML content.
///
/// Escapes `&`, `<`, `>`, `"`, and `'` to their entity references.
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
