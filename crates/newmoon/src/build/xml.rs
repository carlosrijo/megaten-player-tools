//! The XML helpers the database builders (DemonDb, SkillDb, Dungeons) each had a private copy of in C#:
//! `M`, `Mem`, `Obj`, `Records`, `Load`, and the text of an `XElement`.
//!
//! The decoded client and server tables are `<object name="..."><member name="...">value</member>...</object>` trees
//! (`<element>` for list items, `<pair><key/><value/></pair>` for maps). C# read them with `XDocument` / `XmlReader`
//! with `IgnoreWhitespace`: a text node made only of white space does not exist, so `XElement.Value` (all the text
//! under an element) leaves those out. [`value`] does the same.

use std::path::Path;

use roxmltree::{Document, Node};

use crate::Result;
use crate::data::json;

/// File text as the XML readers saw it (UTF-8; a byte order mark is dropped).
pub fn read_text(path: &Path) -> Result<String> {
    json::read_all_text(path)
}

/// Parses a document, with DOCTYPEs allowed (the .NET reader allowed them).
pub fn parse(text: &str) -> Result<Document<'_>> {
    let opt = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    Ok(Document::parse_with_options(text, opt)?)
}

/// A bare `&` (not one of the five entities or a character reference) made into `&amp;`: a few files carry one in a
/// name. C# `Regex.Replace(text, "&(?!amp;|lt;|gt;|quot;|apos;|#)", "&amp;")`.
pub fn fix_bare_ampersands(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let ok = ["amp;", "lt;", "gt;", "quot;", "apos;", "#"].iter().any(|p| after.starts_with(p));
        out.push_str(if ok { "&" } else { "&amp;" });
        rest = after;
    }
    out.push_str(rest);
    out
}

/// The text of an XML file that parses: the text itself, else with its bare `&` fixed, else `None` (what the C#
/// `Load` helpers did: try `XDocument.Load`, on an `XmlException` the fixed text).
pub fn loadable_text(path: &Path) -> Result<Option<String>> {
    let text = read_text(path)?;
    if parse(&text).is_ok() {
        return Ok(Some(text));
    }
    let fixed = fix_bare_ampersands(&text);
    Ok(if parse(&fixed).is_ok() { Some(fixed) } else { None })
}

fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// `XElement.Value` of a document read without insignificant white space: the text of everything under the element.
pub fn value(n: Node) -> String {
    let mut s = String::new();
    for d in n.descendants() {
        if d.is_text() {
            let t = d.text().unwrap_or("");
            if !t.chars().all(is_xml_space) {
                s.push_str(t);
            }
        }
    }
    s
}

/// `o.Elements(name)`: the child elements called `name`.
pub fn elements<'a, 'i>(o: Node<'a, 'i>, name: &'static str) -> impl Iterator<Item = Node<'a, 'i>> {
    o.children().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

/// `o.Element(name)`.
pub fn element<'a, 'i>(o: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    o.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

/// `Mem(o, name)`: the first `<member name="...">` of an object.
pub fn mem<'a, 'i>(o: Option<Node<'a, 'i>>, name: &str) -> Option<Node<'a, 'i>> {
    o?.children().find(|m| m.is_element() && m.tag_name().name() == "member" && m.attribute("name") == Some(name))
}

/// `Obj(o, name)`: the `<object>` inside that member.
pub fn obj<'a, 'i>(o: Option<Node<'a, 'i>>, name: &str) -> Option<Node<'a, 'i>> {
    element(mem(o, name)?, "object")
}

/// `M(o, name)`: that member's text, trimmed; `None` when the object or member is missing.
pub fn m(o: Option<Node>, name: &str) -> Option<String> {
    mem(o, name).map(|n| value(n).trim().to_string())
}

/// `Records(file)`: the `<object>` elements of a table in document order, each one whole, not looked into
/// (the C# streamed them with an `XmlReader`: every `object` found under non-`object` elements).
pub fn records<'a, 'i>(doc: &'a Document<'i>) -> Vec<Node<'a, 'i>> {
    fn collect<'a, 'i>(n: Node<'a, 'i>, out: &mut Vec<Node<'a, 'i>>) {
        for c in n.children().filter(Node::is_element) {
            if c.tag_name().name() == "object" {
                out.push(c);
            } else {
                collect(c, out);
            }
        }
    }
    let mut out = Vec::new();
    collect(doc.root_element(), &mut out);
    out
}

/// `Directory.EnumerateFiles(dir, "*.xml", option)` in the order .NET gave them: a folder's own files in the file
/// system's order, its sub folders after them, first in first out. A missing folder is an error (C#
/// `DirectoryNotFoundException`).
pub fn enumerate_xml_files(dir: &Path, recursive: bool) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    let mut pending = std::collections::VecDeque::from([dir.to_path_buf()]);
    let mut first = true;
    while let Some(d) = pending.pop_front() {
        let rd = match std::fs::read_dir(&d) {
            Ok(rd) => rd,
            Err(e) if first => return Err(anyhow::anyhow!("Could not find a part of the path '{}': {e}", d.display())),
            Err(_) => continue,
        };
        first = false;
        for entry in rd.flatten() {
            let Ok(ty) = entry.file_type() else { continue };
            let path = entry.path();
            if ty.is_dir() {
                if recursive {
                    pending.push_back(path);
                }
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")) {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// `TextInfo.ToTitleCase(s.ToLowerInvariant().Replace('_', ' '))` of the en-US culture: every word (run of letters,
/// digits and apostrophes) starts with a capital, the rest lower case.
pub fn title(s: &str) -> String {
    let lower = s.to_lowercase().replace('_', " ");
    let mut out = String::with_capacity(lower.len());
    let mut in_word = false;
    for c in lower.chars() {
        if c.is_alphabetic() {
            if in_word {
                out.push(c);
            } else {
                out.extend(c.to_uppercase());
                in_word = true;
            }
        } else if in_word && (c == '\'' || c == '\u{2019}' || c.is_numeric() || !is_word_separator(c)) {
            out.push(c);
        } else {
            out.push(c);
            in_word = false;
        }
    }
    out
}

/// The characters that end a word in `ToTitleCase`: white space, controls, punctuation and the math/modifier symbols.
fn is_word_separator(c: char) -> bool {
    c.is_whitespace() || c.is_control() || (c.is_ascii_punctuation() && c != '$') || (!c.is_ascii() && !c.is_alphanumeric() && !c.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_case_like_net() {
        assert_eq!(title("GOD_SPIRIT"), "God Spirit");
        assert_eq!(title(""), "");
        assert_eq!(title("200710_halloween"), "200710 Halloween");
        assert_eq!(title("cure2 x-y"), "Cure2 X-Y");
    }

    #[test]
    fn bare_ampersands() {
        assert_eq!(fix_bare_ampersands("a & b &amp; c &#38; &lt;"), "a &amp; b &amp; c &#38; &lt;");
    }

    #[test]
    fn values_and_members() {
        let doc = parse(
            "<objects><object name=\"A\"><member name=\"x\"> 5 <![CDATA[ y]]></member><member name=\"o\"><object name=\"B\"><member name=\"z\">7</member></object></member></object></objects>",
        )
        .unwrap();
        let recs = records(&doc);
        assert_eq!(recs.len(), 1);
        assert_eq!(m(Some(recs[0]), "x").as_deref(), Some("5  y"));
        assert_eq!(m(obj(Some(recs[0]), "o"), "z").as_deref(), Some("7"));
        assert_eq!(m(Some(recs[0]), "nope"), None);
    }
}
