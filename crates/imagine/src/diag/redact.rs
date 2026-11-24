//! Taking personal details out of text that leaves the PC (the support bundle) or goes into the log.
//!
//!  * The Windows user name in paths: `C:\Users\<name>\...` becomes `%USERPROFILE%\...` (also with `/`, with the doubled
//!    backslashes of JSON, and for any other profile folder). The user name where a path or an address holds it (right after
//!    a `\`, `/` or `@`, or right before an `@`, as a whole word) becomes `<user>`.
//!  * Character names (only when the person has not ticked "Include character names"): each known name becomes
//!    `Character 1`, `Character 2`, ... in the order the names are given, wherever it stands as a word, in any case. They
//!    are replaced before the user name is, so a character who shares the Windows user's name is still `Character N`.
//!
//! A "word" here is a run of letters and digits: `rubito-stats.csv` and `characters\Rokujuu\dash` hold the names
//! `rubito` and `Rokujuu`; `Administrator` does not hold the user name `Admin`.

use regex::{Regex, RegexBuilder};
use std::sync::OnceLock;

/// What replaces the user's profile folder in a path.
pub const PROFILE: &str = "%USERPROFILE%";
/// What replaces the bare user name.
pub const USER: &str = "<user>";

pub struct Redactor {
    /// the person's own profile folder, in the forms it can be written (plain, `/`, JSON-escaped), or None
    profile: Option<Regex>,
    /// `X:\Users\<anything>`: another profile folder (a path from a log or a config written elsewhere)
    any_profile: Regex,
    user: Option<String>,
    /// (name, replacement)
    names: Vec<(String, String)>,
}

fn ci(pattern: &str) -> Regex {
    RegexBuilder::new(pattern).case_insensitive(true).build().expect("static pattern")
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric()
}

/// Every place `needle` stands as a whole word in `text` (case-insensitive), replaced.
fn replace_words(text: &str, needle: &str, with: &str) -> String {
    if needle.is_empty() {
        return text.to_string();
    }
    let re = ci(&regex::escape(needle));
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in re.find_iter(text) {
        let before = text[..m.start()].chars().next_back();
        let after = text[m.end()..].chars().next();
        if before.is_some_and(is_word) || after.is_some_and(is_word) {
            continue;
        }
        out.push_str(&text[last..m.start()]);
        out.push_str(with);
        last = m.end();
    }
    out.push_str(&text[last..]);
    out
}

/// The user name as it stands in a path or an address: right after a `\`, `/` or `@`, or right before an `@`, as a whole
/// word. (Elsewhere the word may just be a word: a user called "User" must not turn "user error" into "<user> error".)
fn replace_user_in_paths(text: &str, user: &str) -> String {
    if user.is_empty() {
        return text.to_string();
    }
    let word_after = |s: &str, end: usize| s[end..].chars().next().is_some_and(is_word);
    let after_slash = ci(&format!(r"([\\/@]){}", regex::escape(user)));
    let s = after_slash
        .replace_all(text, |c: &regex::Captures| {
            let end = c.get(0).map(|m| m.end()).unwrap_or(0);
            if word_after(text, end) { c[0].to_string() } else { format!("{}{USER}", &c[1]) }
        })
        .into_owned();
    // `Joe@PC`: in front of an @, as a whole word
    let before_at = ci(&format!(r"{}@", regex::escape(user)));
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    for m in before_at.find_iter(&s) {
        if s[..m.start()].chars().next_back().is_some_and(is_word) {
            continue;
        }
        out.push_str(&s[last..m.start()]);
        out.push_str(USER);
        out.push('@');
        last = m.end();
    }
    out.push_str(&s[last..]);
    out
}

impl Redactor {
    /// For the person using this PC: USERPROFILE and USERNAME from the environment.
    pub fn for_this_pc() -> Redactor {
        Redactor::new(std::env::var("USERPROFILE").ok().as_deref(), std::env::var("USERNAME").ok().as_deref())
    }

    /// `profile`: the user's profile folder (`C:\Users\Joe`); `user`: the Windows user name. Either may be unknown.
    pub fn new(profile: Option<&str>, user: Option<&str>) -> Redactor {
        let profile = profile.map(|p| p.trim().trim_end_matches(['\\', '/'])).filter(|p| p.len() > 3);
        let profile_re = profile.map(|p| {
            let plain = p.replace('/', "\\");
            let forms = [plain.clone(), plain.replace('\\', "/"), plain.replace('\\', "\\\\")];
            let alt: Vec<String> = forms.iter().map(|f| regex::escape(f)).collect();
            ci(&alt.join("|"))
        });
        // the folder's own name is the user name when the Windows user name is not known
        let from_profile = profile.and_then(|p| p.rsplit(['\\', '/']).next()).map(str::to_string);
        let user = user.map(|u| u.trim().to_string()).filter(|u| u.chars().count() >= 3).or(from_profile.filter(|u| u.chars().count() >= 3));
        Redactor {
            profile: profile_re,
            any_profile: ci(r#"[A-Za-z]:(?:\\{1,2}|/)Users(?:\\{1,2}|/)([^\\/:*?"<>|\s]+)"#),
            user,
            names: Vec::new(),
        }
    }

    /// The character names to hide, in the order that gives `Character 1`, `Character 2`, ... (repeats and blanks are ignored).
    pub fn hide_names<S: AsRef<str>>(&mut self, names: &[S]) {
        for n in names {
            let n = n.as_ref().trim();
            if n.chars().count() < 2 || self.names.iter().any(|(x, _)| x.eq_ignore_ascii_case(n)) {
                continue;
            }
            let label = format!("Character {}", self.names.len() + 1);
            self.names.push((n.to_string(), label));
        }
    }

    /// What a name is replaced with (None when it is not hidden).
    pub fn label_of(&self, name: &str) -> Option<&str> {
        self.names.iter().find(|(x, _)| x.eq_ignore_ascii_case(name.trim())).map(|(_, l)| l.as_str())
    }

    /// The profile folder in a path, in any spelling.
    fn profile_paths(&self, text: &str) -> String {
        let mut s = text.to_string();
        if let Some(p) = &self.profile {
            s = p.replace_all(&s, PROFILE).into_owned();
        }
        s = self
            .any_profile
            .replace_all(&s, |c: &regex::Captures| {
                let who = c[1].to_ascii_lowercase();
                if matches!(who.as_str(), "public" | "default" | "all") { c[0].to_string() } else { PROFILE.to_string() }
            })
            .into_owned();
        s
    }

    /// Paths and the user name only.
    pub fn user_only(&self, text: &str) -> String {
        let s = self.profile_paths(text);
        match &self.user {
            Some(u) => replace_user_in_paths(&s, u),
            None => s,
        }
    }

    /// Everything: paths, the hidden character names, then the user name (the names first, so a character who shares the
    /// user's name is still `Character N`).
    pub fn text(&self, text: &str) -> String {
        let mut s = self.profile_paths(text);
        for (name, label) in &self.names {
            s = replace_words(&s, name, label);
            // the folder and spreadsheet names it was turned into ("Chidori Y." is "Chidori Y_" on disk)
            let on_disk = crate::data::characters::safe_file_name(name);
            if on_disk != *name {
                s = replace_words(&s, &on_disk, label);
            }
        }
        match &self.user {
            Some(u) => replace_user_in_paths(&s, u),
            None => s,
        }
    }

    /// "Include character names" is ticked: a name that is also the user's own name stays, so the user name is then only
    /// taken out of the profile folder paths.
    pub fn keep_names<S: AsRef<str>>(&mut self, names: &[S]) {
        if let Some(u) = &self.user {
            if names.iter().any(|n| n.as_ref().trim().eq_ignore_ascii_case(u)) {
                self.user = None;
            }
        }
    }
}

/// `Redactor::for_this_pc().user_only(text)`, with the redactor made once: for lines that go into the log.
pub fn user_only(text: &str) -> String {
    static ME: OnceLock<Redactor> = OnceLock::new();
    ME.get_or_init(Redactor::for_this_pc).user_only(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joe() -> Redactor {
        Redactor::new(Some(r"C:\Users\Joe Cool"), Some("JoeCool"))
    }

    #[test]
    fn the_profile_folder_in_every_form() {
        let r = joe();
        assert_eq!(r.text(r"C:\Users\Joe Cool\Documents\x.csv"), r"%USERPROFILE%\Documents\x.csv");
        assert_eq!(r.text("c:/users/joe cool/Documents"), "%USERPROFILE%/Documents");
        assert_eq!(r.text(r#"{"statsFolder": "C:\\Users\\Joe Cool\\Documents\\smt-stats"}"#), r#"{"statsFolder": "%USERPROFILE%\\Documents\\smt-stats"}"#);
    }

    #[test]
    fn another_profile_folder_and_the_system_ones() {
        let r = joe();
        assert_eq!(r.text(r"D:\Users\Zed\AppData\Roaming\x"), r"%USERPROFILE%\AppData\Roaming\x");
        assert_eq!(r.text(r"C:\Users\Public\Documents"), r"C:\Users\Public\Documents");
        // no profile known: the generic pattern still catches it
        let r = Redactor::new(None, None);
        assert_eq!(r.text(r"at C:\Users\Admin\.cargo\registry\src\x.rs:12"), r"at %USERPROFILE%\.cargo\registry\src\x.rs:12");
    }

    #[test]
    fn the_user_name_is_a_whole_word() {
        let r = Redactor::new(Some(r"C:\Users\Admin"), Some("Admin"));
        assert_eq!(r.text(r"on \\PC\Admin\share, in /home/admin/x, mail admin@host"), r"on \\PC\<user>\share, in /home/<user>/x, mail <user>@host");
        // a word that is only a word stays: a user called "Admin" must not turn "admin error" into "<user> error"
        assert_eq!(r.text("logged in as admin: admin error"), "logged in as admin: admin error");
        assert_eq!(r.text(r"Administrator and \admins\ stay"), r"Administrator and \admins\ stay");
        assert_eq!(r.text(r"JSON \\Admin\\x"), r"JSON \\<user>\\x");
        // a short or unknown name is left alone rather than damaging the text
        assert_eq!(Redactor::new(None, Some("Al")).text("Al went"), "Al went");
    }

    #[test]
    fn character_names_become_numbered_characters() {
        let mut r = joe();
        r.hide_names(&["Rubito", "Rokujuu", "rubito", " "]);
        assert_eq!(r.label_of("RUBITO"), Some("Character 1"));
        assert_eq!(r.text(r"data\characters\Rokujuu\dash and rubito-stats.csv, Rubito's gear"), r"data\characters\Character 2\dash and Character 1-stats.csv, Character 1's gear");
        // inside a longer word: untouched
        assert_eq!(r.text("Rubitomaniac"), "Rubitomaniac");
        // names are not hidden unless asked
        assert_eq!(joe().text("Rubito"), "Rubito");
    }

    /// A name changed to make a folder ("Chidori Y." is "Chidori Y_" on disk) is hidden there too.
    #[test]
    fn a_name_is_hidden_in_its_folder_name_too() {
        let mut r = joe();
        r.hide_names(&["Chidori Y."]);
        assert_eq!(r.text(r"data\characters\Chidori Y_\dash, chidori y_-stats.csv, Chidori Y. logged in"), r"data\characters\Character 1\dash, Character 1-stats.csv, Character 1 logged in");
    }

    #[test]
    fn japanese_text_passes_through() {
        let mut r = joe();
        r.hide_names(&["ルビト"]);
        assert_eq!(r.text("名前 ルビト です"), "名前 Character 1 です");
    }
}
