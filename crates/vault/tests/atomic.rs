//! Every write lands through a temporary name the sync bridge ignores, in the target's own
//! directory, then a file sync, the rename and a directory sync, in that order; a step that fails
//! leaves the target as it was and removes the temporary file (SPEC-042 A1, R2). No write reaches a
//! journal folder, and every file the vault context writes goes through the atomic writer
//! (SPEC-118 A5, A6, R5).

// An integration test is test code: its helpers panic on an unreadable file, and the source scan
// prints its examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use deck_streak_vault::{
    DirEntry, EntryKind, Inbox, JournalGuard, LayoutInForce, RealFs, VaultError, VaultFile,
    VaultFs, atomic,
};

/// One step a write took, as the recording file system saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    CreateNew(PathBuf),
    Write(PathBuf),
    SyncFile(PathBuf),
    Rename(PathBuf, PathBuf),
    SyncDir(PathBuf),
    RemoveFile(PathBuf),
}

/// The step the recording file system makes fail, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fail {
    Nothing,
    Write,
    SyncFile,
    Rename,
}

type Log = Arc<Mutex<Vec<Step>>>;

/// A file system that records every write step, does it on the real file system, and fails the
/// step it was told to.
struct Recording {
    log: Log,
    fail: Fail,
}

impl Recording {
    fn new(fail: Fail) -> Self {
        Self {
            log: Arc::default(),
            fail,
        }
    }

    fn steps(&self) -> Vec<Step> {
        self.log.lock().expect("the log").clone()
    }
}

fn record(log: &Log, step: Step) {
    log.lock().expect("the log").push(step);
}

fn planted() -> io::Error {
    io::Error::other("a planted failure")
}

struct RecordingFile {
    inner: Box<dyn VaultFile>,
    path: PathBuf,
    log: Log,
    fail: Fail,
}

impl VaultFile for RecordingFile {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        record(&self.log, Step::Write(self.path.clone()));
        if self.fail == Fail::Write {
            return Err(planted());
        }
        self.inner.write_all(bytes)
    }

    fn sync(&mut self) -> io::Result<()> {
        record(&self.log, Step::SyncFile(self.path.clone()));
        if self.fail == Fail::SyncFile {
            return Err(planted());
        }
        self.inner.sync()
    }
}

impl VaultFs for Recording {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
        record(&self.log, Step::CreateNew(path.to_path_buf()));
        Ok(Box::new(RecordingFile {
            inner: RealFs.create_new(path)?,
            path: path.to_path_buf(),
            log: Arc::clone(&self.log),
            fail: self.fail,
        }))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        record(
            &self.log,
            Step::Rename(from.to_path_buf(), to.to_path_buf()),
        );
        if self.fail == Fail::Rename {
            return Err(planted());
        }
        RealFs.rename(from, to)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        record(&self.log, Step::SyncDir(dir.to_path_buf()));
        RealFs.sync_dir(dir)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        record(&self.log, Step::RemoveFile(path.to_path_buf()));
        RealFs.remove_file(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        RealFs.read(path)
    }

    fn kind(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        RealFs.kind(path)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        RealFs.list(dir)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        RealFs.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        RealFs.remove_dir(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        RealFs.canonicalize(path)
    }
}

/// The sync bridge's ignore pattern, `\.tmp\.\d+\.|\.tmp$|\.crswap$|^~|\.crdownload$`, matched by
/// hand: a name it matches is never replicated to the owner's devices.
#[allow(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the bridge's pattern is case-sensitive"
)]
fn bridge_ignores(name: &str) -> bool {
    let tmp_then_digits_then_dot = name.match_indices(".tmp.").any(|(at, _)| {
        let rest = &name[at + ".tmp.".len()..];
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        digits > 0 && rest[digits..].starts_with('.')
    });
    tmp_then_digits_then_dot
        || name.ends_with(".tmp")
        || name.ends_with(".crswap")
        || name.starts_with('~')
        || name.ends_with(".crdownload")
}

#[test]
fn a_write_lands_through_an_ignored_temp_name_then_fsync_rename_and_dir_fsync() {
    let vault = tempfile::tempdir().expect("a temporary vault");
    let target = vault.path().join("law-evidence.md");
    std::fs::write(&target, "the note before\n").expect("the note before the write");
    let fs = Recording::new(Fail::Nothing);

    atomic::write(&fs, &target, b"the note after\n").expect("the write lands");

    let temp = vault
        .path()
        .join(format!(".law-evidence.md.{}.tmp", std::process::id()));
    assert_eq!(
        fs.steps(),
        vec![
            Step::CreateNew(temp.clone()),
            Step::Write(temp.clone()),
            Step::SyncFile(temp.clone()),
            Step::Rename(temp.clone(), target.clone()),
            Step::SyncDir(vault.path().to_path_buf()),
        ],
        "a temp file in the target's own directory, then its fsync, the rename and the dir fsync"
    );
    let name = temp
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a UTF-8 name");
    assert!(
        bridge_ignores(name),
        "{name} is not ignored by the sync bridge"
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("the target"),
        "the note after\n"
    );
    assert!(!temp.exists(), "the temporary file was left behind");
}

#[test]
fn a_failed_step_leaves_the_target_as_it_was_and_removes_the_temp_file() {
    for fail in [Fail::Write, Fail::SyncFile, Fail::Rename] {
        let vault = tempfile::tempdir().expect("a temporary vault");
        let target = vault.path().join("law-evidence.md");
        std::fs::write(&target, "the note before\n").expect("the note before the write");
        let fs = Recording::new(fail);

        let refused = atomic::write(&fs, &target, b"the note after\n");

        assert!(refused.is_err(), "a failed {fail:?} failed the write");
        assert_eq!(
            std::fs::read_to_string(&target).expect("the target"),
            "the note before\n",
            "a failed {fail:?} left the target as it was"
        );
        let temp = vault
            .path()
            .join(format!(".law-evidence.md.{}.tmp", std::process::id()));
        assert_eq!(
            fs.steps().last(),
            Some(&Step::RemoveFile(temp.clone())),
            "a failed {fail:?} removes the temporary file"
        );
        assert!(!temp.exists(), "a failed {fail:?} left the temporary file");
    }
}

#[test]
fn no_vault_write_reaches_a_journal_folder() {
    let vault = tempfile::tempdir().expect("a temporary vault");
    let root = vault.path();
    let journal = root.join("Journal");
    fs::create_dir_all(journal.join("2026")).expect("the journal folder");
    fs::create_dir(root.join("90-Inbox")).expect("the inbox folder");
    fs::create_dir(root.join("Journaling")).expect("a sibling folder");
    let folders = vec![journal.clone()];

    // The refusal is lexical: the folder itself, a file under it, another case, and a `..` that
    // climbs back into it are refused before any file is touched.
    for path in [
        journal.clone(),
        journal.join("a.md"),
        journal.join("2026/b.md"),
        root.join("JOURNAL/c.md"),
        root.join("journal/2026/d.md"),
        root.join("90-Inbox/../Journal/e.md"),
        root.join("90-Inbox/./../jOuRnAl/f.md"),
    ] {
        let refused = atomic::refuse_journal(&folders, &path);
        assert!(
            matches!(refused, Err(VaultError::JournalRefused)),
            "{} lies under the journal: {refused:?}",
            path.display()
        );
    }
    for path in [
        root.join("Journaling/a.md"),
        root.join("Journal.md"),
        root.join("90-Inbox/a.md"),
        root.join("90-Inbox/Journal/a.md"),
        root.join("Journal/../90-Inbox/b.md"),
    ] {
        assert!(
            atomic::refuse_journal(&folders, &path).is_ok(),
            "{} lies outside the journal",
            path.display()
        );
    }

    // The writer and its streamed form refuse through the guarded file system.
    let guard = JournalGuard::new(RealFs, folders.clone());
    assert_eq!(guard.journal(), folders.as_slice());
    for target in [journal.join("a.md"), root.join("JOURNAL/2026/b.md")] {
        let refused = atomic::write(&guard, &target, b"a journal entry\n");
        assert!(
            matches!(refused, Err(VaultError::JournalRefused)),
            "the writer refuses {}: {refused:?}",
            target.display()
        );
    }
    let refused = atomic::stream(&guard, &journal.join("c.pdf"));
    assert!(
        matches!(refused, Err(VaultError::JournalRefused)),
        "the streamed writer refuses the journal: {refused:?}"
    );

    // The guard itself refuses a file created, a file renamed in and a folder made under the
    // journal, so a caller that skips the writer is refused too.
    let created = guard.create_new(&journal.join("d.md")).err();
    assert_eq!(
        created.map(|error| error.kind()),
        Some(io::ErrorKind::PermissionDenied),
        "the guard refuses a file created under the journal"
    );
    let outside = root.join("90-Inbox/e.md");
    fs::write(&outside, "a capture\n").expect("a file outside the journal");
    let renamed = guard.rename(&outside, &journal.join("e.md")).err();
    assert_eq!(
        renamed.map(|error| error.kind()),
        Some(io::ErrorKind::PermissionDenied),
        "the guard refuses a rename into the journal"
    );
    assert!(outside.is_file(), "the refused rename moved nothing");
    let made = guard.create_dir(&journal.join("2027")).err();
    assert_eq!(
        made.map(|error| error.kind()),
        Some(io::ErrorKind::PermissionDenied),
        "the guard refuses a folder made under the journal"
    );
    let mut left: Vec<_> = fs::read_dir(&journal)
        .expect("the journal")
        .map(|entry| entry.expect("an entry").file_name())
        .collect();
    left.sort();
    assert_eq!(left, vec!["2026"], "nothing was written under the journal");

    // A write outside the journal still lands, a sibling whose name starts the same included.
    atomic::write(&guard, &root.join("Journaling/f.md"), b"not the journal\n")
        .expect("a write outside the journal lands");

    // An inbox under a journal folder is refused before any capture is written.
    let layout = LayoutInForce {
        inbox: "Journal/2026".to_owned(),
        journal: vec!["Journal".to_owned()],
    };
    assert_eq!(layout.journal_paths(root), vec![journal.clone()]);
    let refused = Inbox::locate(&RealFs, root, &layout);
    assert!(
        matches!(refused, Err(VaultError::JournalRefused)),
        "an inbox under the journal is refused: {refused:?}"
    );
    assert_eq!(VaultError::JournalRefused.to_string(), "journal_refused");
}

/// The calls that write a file's bytes. Each is allowed only inside the atomic writer and the
/// file-system port it writes through.
const FILE_WRITES: [&str; 5] = [
    "create_new(",
    "write_all(",
    "fs::write(",
    "File::create(",
    "OpenOptions",
];

/// Every `.rs` file under `dir`, sorted.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).expect("a source folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `text` with every comment, string literal and character literal blanked, newlines kept, so a
/// name in a comment or a string is not a call and line numbers still hold.
fn code_of(text: &str) -> Vec<char> {
    let chars: Vec<char> = text.chars().collect();
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let prev_ident = i > 0 && is_ident(chars[i - 1]);
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.extend([' ', ' ']);
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.extend([' ', ' ']);
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
        } else if c == 'r'
            && matches!(next, Some('"' | '#'))
            && (!prev_ident || (chars[i - 1] == 'b' && (i < 2 || !is_ident(chars[i - 2]))))
        {
            let mut j = i + 1;
            while chars.get(j) == Some(&'#') {
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                let hashes = j - i - 1;
                let mut k = j + 1;
                while k < chars.len() {
                    if chars[k] == '"' && (1..=hashes).all(|h| chars.get(k + h) == Some(&'#')) {
                        k += 1 + hashes;
                        break;
                    }
                    k += 1;
                }
                out.extend(chars[i..k.min(chars.len())].iter().map(|&c| blank(c)));
                i = k;
            } else {
                out.push(c);
                i += 1;
            }
        } else if c == '"' {
            out.push(' ');
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    out.push(' ');
                    if let Some(&escaped) = chars.get(i + 1) {
                        out.push(blank(escaped));
                    }
                    i += 2;
                } else if chars[i] == '"' {
                    out.push(' ');
                    i += 1;
                    break;
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
        } else if c == '\'' && next == Some('\\') {
            let mut k = i + 3;
            while k < chars.len() && chars[k] != '\'' {
                k += 1;
            }
            out.extend(chars[i..=k.min(chars.len() - 1)].iter().map(|_| ' '));
            i = k + 1;
        } else if c == '\'' && chars.get(i + 2) == Some(&'\'') {
            out.extend([' ', ' ', ' ']);
            i += 3;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `code` with every item under `#[cfg(test)]` blanked: the attribute, then through the first `;`
/// or the `}` that closes the first `{`.
fn without_test_items(mut code: Vec<char>) -> Vec<char> {
    let marker: Vec<char> = "#[cfg(test)]".chars().collect();
    let mut i = 0;
    while i + marker.len() <= code.len() {
        if code[i..i + marker.len()] != marker[..] {
            i += 1;
            continue;
        }
        let mut end = i + marker.len();
        let mut depth = 0usize;
        while end < code.len() {
            match code[end] {
                ';' if depth == 0 => break,
                '{' => depth += 1,
                '}' => {
                    if depth <= 1 {
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            end += 1;
        }
        let stop = end.min(code.len() - 1);
        for c in &mut code[i..=stop] {
            if *c != '\n' {
                *c = ' ';
            }
        }
        i = stop + 1;
    }
    code
}

#[test]
fn every_vault_file_write_is_the_atomic_writer() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut outside = Vec::new();
    let mut inside = 0;
    let files = sources(&src);
    for path in &files {
        let name = path
            .strip_prefix(&src)
            .expect("a file under src")
            .to_string_lossy()
            .into_owned();
        let text = fs::read_to_string(path).expect("a source file");
        let code: String = without_test_items(code_of(&text)).into_iter().collect();
        let writer = name == "atomic.rs" || name == "fs.rs";
        for (number, line) in code.lines().enumerate() {
            for call in FILE_WRITES {
                let found = line.matches(call).count();
                if found > 0 && writer {
                    inside += found;
                } else if found > 0 {
                    outside.push(format!("{name}:{}: {call}", number + 1));
                }
            }
        }
    }
    println!(
        "examined {} source file(s) of crates/vault/src; {inside} file-writing call(s) inside \
         the atomic writer and its port, {} outside",
        files.len(),
        outside.len()
    );
    assert!(
        outside.is_empty(),
        "file-writing calls outside the atomic writer: {outside:#?}"
    );
    assert!(
        files.len() > 10 && inside > 0,
        "the scan read the crate and saw the writer's own calls: {} file(s), {inside} call(s)",
        files.len()
    );
}
