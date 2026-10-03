//! SPEC-119 A2 to A6: the guard's credentials, read only through the credential loader, and every
//! refusal by the credential's id (R6, R7, R8; T15). A1 and A39 (section 14): the listen address
//! must be loopback, and the role reads its tokens only through the loader (R3, R6; ADR-329 D6).

// An integration test is test code: its helpers panic on a failed fixture, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};

use deck_streak_kernel::{
    CredentialError, CredentialLoader, CredentialsDirectory, Environment, Redactor,
};
use deck_streak_mcp::settings::LISTEN;
use deck_streak_mcp::{Grants, ListenAddress, ListenRefusal, McpError, Scopes};

const CORE: &str = "mcp-core-token";
const LAW_TRACK: &str = "mcp-law-track-token";

/// A token of `length` characters, built from parts at run time.
fn token(stem: &str, length: usize) -> String {
    let mut value = format!("settings-{stem}-");
    let fill = length.saturating_sub(value.chars().count());
    value.push_str(&"k".repeat(fill));
    value.truncate(length);
    value
}

/// How a credential sits in the credentials directory.
enum Credential {
    /// A file holding the value and the line feed systemd writes.
    Text(String),
    /// A file holding these bytes exactly.
    Bytes(Vec<u8>),
    /// A directory of the credential's name, which the loader cannot read as a file.
    Unreadable,
}

/// A credentials directory holding `credentials`.
fn directory(credentials: &[(&str, Credential)]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for (id, credential) in credentials {
        let path = directory.path().join(id);
        match credential {
            Credential::Text(value) => fs::write(path, format!("{value}\n")),
            Credential::Bytes(bytes) => fs::write(path, bytes),
            Credential::Unreadable => fs::create_dir(path),
        }
        .expect("a credential");
    }
    directory
}

/// The guard's grants loaded from `credentials`.
fn load(credentials: &[(&str, Credential)]) -> Result<Grants, McpError> {
    let directory = directory(credentials);
    let path = CredentialsDirectory::new(directory.path()).expect("an absolute path");
    Grants::load(&CredentialLoader::new(path, Redactor::new()))
}

/// The scope names each loaded grant holds, in load order.
fn scope_names(grants: &Grants) -> Vec<Vec<&'static str>> {
    grants.scopes().into_iter().map(Scopes::names).collect()
}

/// The broken forms of a credential the loader refuses: empty, unreadable and non-text.
fn broken() -> Vec<(&'static str, Credential)> {
    vec![
        ("empty", Credential::Bytes(b"\n".to_vec())),
        ("unreadable", Credential::Unreadable),
        ("non-text", Credential::Bytes(vec![0xff, 0xfe, 0xfd, b'\n'])),
    ]
}

#[test]
fn the_core_credential_is_required() {
    // A missing core credential refuses start by its id.
    let missing = load(&[(LAW_TRACK, Credential::Text(token("law", 40)))]);
    assert!(
        matches!(
            missing,
            Err(McpError::Credential(CredentialError::Missing { id: CORE }))
        ),
        "a missing core credential: {missing:?}"
    );

    // So does an empty, an unreadable and a non-text one.
    for (form, credential) in broken() {
        let refused = load(&[(CORE, credential)]);
        let by_id = match &refused {
            Err(McpError::Credential(CredentialError::Empty { id })) => {
                form == "empty" && *id == CORE
            }
            Err(McpError::Credential(CredentialError::Unreadable { id, .. })) => {
                form == "unreadable" && *id == CORE
            }
            Err(McpError::Credential(CredentialError::NotText { id })) => {
                form == "non-text" && *id == CORE
            }
            _ => false,
        };
        assert!(by_id, "a {form} core credential: {refused:?}");
    }

    // And a valid core credential alone starts, granting core.
    let grants =
        load(&[(CORE, Credential::Text(token("core", 40)))]).expect("the core token starts");
    assert_eq!(scope_names(&grants), vec![vec!["core"]]);
}

#[test]
fn a_missing_scope_credential_grants_nothing() {
    let grants = load(&[(CORE, Credential::Text(token("core", 40)))])
        .expect("a missing law-track credential still starts");
    assert_eq!(scope_names(&grants), vec![vec!["core"]]);

    let both = load(&[
        (CORE, Credential::Text(token("core", 40))),
        (LAW_TRACK, Credential::Text(token("law", 40))),
    ])
    .expect("both credentials start");
    assert_eq!(
        scope_names(&both),
        vec![vec!["core"], vec!["core", "law_track"]]
    );
}

#[test]
fn a_broken_scope_credential_refuses_start() {
    for (form, credential) in broken() {
        let refused = load(&[
            (CORE, Credential::Text(token("core", 40))),
            (LAW_TRACK, credential),
        ]);
        let by_id = match &refused {
            Err(McpError::Credential(CredentialError::Empty { id })) => {
                form == "empty" && *id == LAW_TRACK
            }
            Err(McpError::Credential(CredentialError::Unreadable { id, .. })) => {
                form == "unreadable" && *id == LAW_TRACK
            }
            Err(McpError::Credential(CredentialError::NotText { id })) => {
                form == "non-text" && *id == LAW_TRACK
            }
            _ => false,
        };
        assert!(by_id, "a {form} law-track credential: {refused:?}");
    }
}

#[test]
fn a_short_credential_refuses_start() {
    // 31 characters refuse start, by the credential's id.
    let short_core = load(&[(CORE, Credential::Text(token("core", 31)))]);
    assert!(
        matches!(short_core, Err(McpError::WeakCredential { id: CORE })),
        "a 31-character core token: {short_core:?}"
    );
    let short_law = load(&[
        (CORE, Credential::Text(token("core", 40))),
        (LAW_TRACK, Credential::Text(token("law", 31))),
    ]);
    assert!(
        matches!(short_law, Err(McpError::WeakCredential { id: LAW_TRACK })),
        "a 31-character law-track token: {short_law:?}"
    );

    // A token ending in a carriage return (a file written with CRLF) can never be presented, so it
    // refuses start as unpresentable, before its length is read.
    let carriage = load(&[(CORE, Credential::Text(format!("{}\r", token("core", 40))))]);
    assert!(
        matches!(
            carriage,
            Err(McpError::UnpresentableCredential { id: CORE })
        ),
        "a core token ending in a carriage return: {carriage:?}"
    );
    let short_carriage = load(&[
        (CORE, Credential::Text(token("core", 40))),
        (
            LAW_TRACK,
            Credential::Text(format!("{}\r", token("law", 20))),
        ),
    ]);
    assert!(
        matches!(
            short_carriage,
            Err(McpError::UnpresentableCredential { id: LAW_TRACK })
        ),
        "a short law-track token ending in a carriage return: {short_carriage:?}"
    );

    // And 32 characters start, for either credential.
    let grants = load(&[
        (CORE, Credential::Text(token("core", 32))),
        (LAW_TRACK, Credential::Text(token("law", 32))),
    ])
    .expect("32-character tokens start");
    assert_eq!(
        scope_names(&grants),
        vec![vec!["core"], vec!["core", "law_track"]]
    );
}

#[test]
fn two_credentials_with_one_value_refuse_start() {
    let value = token("shared", 40);
    let refused = load(&[
        (CORE, Credential::Text(value.clone())),
        (LAW_TRACK, Credential::Text(value)),
    ]);
    assert!(
        matches!(
            refused,
            Err(McpError::SharedCredential {
                first: CORE,
                second: LAW_TRACK
            })
        ),
        "two credentials with one value: {refused:?}"
    );

    // Two values that differ only in their last character are two grants.
    let mut other = token("shared", 40);
    other.pop();
    other.push('x');
    let distinct = load(&[
        (CORE, Credential::Text(token("shared", 40))),
        (LAW_TRACK, Credential::Text(other)),
    ])
    .expect("two values start");
    assert_eq!(
        scope_names(&distinct),
        vec![vec!["core"], vec!["core", "law_track"]]
    );
}

/// The listen setting read from an environment holding `value`, or none.
fn listen(value: Option<&str>) -> Result<ListenAddress, McpError> {
    ListenAddress::from_env(&Environment::from_vars(value.map(|value| (LISTEN, value))))
}

/// Requires `refused` to be the listen setting's refusal for `reason`, naming the setting and not
/// carrying `value`.
fn assert_listen_refused(
    refused: &Result<ListenAddress, McpError>,
    reason: ListenRefusal,
    value: &str,
) {
    match refused {
        Err(
            error @ McpError::Listen {
                setting,
                reason: found,
            },
        ) => {
            assert_eq!(*setting, "DECKSTREAK_MCP_LISTEN", "{error}");
            assert_eq!(*found, reason, "{value:?}: {error}");
            let text = error.to_string();
            assert!(text.contains("DECKSTREAK_MCP_LISTEN"), "{text}");
            assert!(
                value.is_empty() || !text.contains(value),
                "the refusal carries the value: {text}"
            );
        }
        other => panic!("{value:?} was not refused as {reason:?}: {other:?}"),
    }
}

#[test]
fn the_listen_address_must_be_loopback() {
    // Not a loopback address, the unspecified ones that listen on every interface among them:
    // refused, naming the setting.
    let foreign = [
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8790)),
        SocketAddr::from((Ipv4Addr::new(10, 0, 0, 1), 8790)),
        SocketAddr::from((Ipv6Addr::UNSPECIFIED, 8790)),
        SocketAddr::from((Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1), 8790)),
    ];
    for address in foreign {
        let value = address.to_string();
        assert_listen_refused(&listen(Some(&value)), ListenRefusal::NotLoopback, &value);
    }

    // Unset, blank, or not a socket address with a port: refused, naming the setting.
    assert_listen_refused(&listen(None), ListenRefusal::Unset, "");
    assert_listen_refused(&listen(Some("  ")), ListenRefusal::Unset, "");
    for value in ["localhost:8790", "8790", "no-such-address", "[::1]"] {
        assert_listen_refused(&listen(Some(value)), ListenRefusal::NotAnAddress, value);
    }

    // A loopback address of either family is the address the server listens on.
    for address in [
        SocketAddr::from((Ipv4Addr::LOCALHOST, 8790)),
        SocketAddr::from((Ipv4Addr::new(127, 0, 0, 2), 8790)),
        SocketAddr::from((Ipv6Addr::LOCALHOST, 8790)),
    ] {
        let value = address.to_string();
        let listen = listen(Some(&value)).expect("a loopback address starts");
        assert_eq!(listen.socket_address(), address);
    }
}

/// The repository root, two levels above this crate.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The reads of a file or a variable that would put a token beside the loader, by the call's
/// spelling.
const OTHER_READS: [&str; 10] = [
    "env::var(",
    "env::var_os(",
    "env::vars(",
    "env::vars_os(",
    "env::args",
    "fs::read(",
    "fs::read_to_string(",
    "File::open(",
    "OpenOptions",
    "read_to_string(",
];

/// The code of `text`: every line with its `//` comment cut off.
fn code(text: &str) -> String {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each read of [`OTHER_READS`] in `text`'s code, as `<name>: <read>`.
fn other_reads(name: &str, text: &str) -> Vec<String> {
    let code = code(text);
    OTHER_READS
        .iter()
        .filter(|read| code.contains(*read))
        .map(|read| format!("{name}: {read}"))
        .collect()
}

#[test]
fn the_role_reads_its_tokens_only_through_the_loader() {
    // A token-shaped file beside the credentials directory, and files of other names inside it,
    // grant nothing: the loader reads the credential's own id alone, and refuses by that id.
    let parent = tempfile::tempdir().expect("a temporary directory");
    let directory = parent.path().join("credentials");
    fs::create_dir(&directory).expect("the credentials directory");
    let value = token("core", 40);
    fs::write(parent.path().join(CORE), format!("{value}\n")).expect("a file beside it");
    fs::write(directory.join(format!("{CORE}.txt")), format!("{value}\n")).expect("a file");
    fs::write(
        directory.join("DECKSTREAK_MCP_CORE_TOKEN"),
        format!("{value}\n"),
    )
    .expect("a file");
    let path = CredentialsDirectory::new(&directory).expect("an absolute path");
    let refused = Grants::load(&CredentialLoader::new(path, Redactor::new()));
    assert!(
        matches!(
            refused,
            Err(McpError::Credential(CredentialError::Missing { id: CORE }))
        ),
        "a core token found outside the loader's id: {refused:?}"
    );

    // The role's source loads the grants once, through one loader.
    let root = repository();
    let role_path = root.join("crates/daemon/src/role_mcp.rs");
    assert!(
        role_path.is_file(),
        "the mcp role's source is missing: {}",
        role_path.display()
    );
    let role = fs::read_to_string(&role_path).expect("the role's source");
    let role_code = code(&role);
    assert_eq!(
        role_code.matches("Grants::load(").count(),
        1,
        "the role loads its grants once"
    );
    assert_eq!(
        role_code.matches("CredentialLoader::new(").count(),
        1,
        "the role builds one loader"
    );

    // And neither the role nor the adapter reads a file or a variable any other way.
    let mut sources = vec![("crates/daemon/src/role_mcp.rs".to_owned(), role)];
    let adapter = root.join("crates/mcp/src");
    let mut names: Vec<PathBuf> = fs::read_dir(&adapter)
        .expect("the adapter's sources")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    names.sort();
    for path in names {
        let text = fs::read_to_string(&path).expect("a source");
        sources.push((path.display().to_string(), text));
    }
    println!("examined {} source file(s)", sources.len());
    assert!(sources.len() > 1, "examined no adapter source");
    let found: Vec<String> = sources
        .iter()
        .flat_map(|(name, text)| other_reads(name, text))
        .collect();
    assert_eq!(found, Vec::<String>::new(), "a read beside the loader");

    // The census sees a planted read, so a census gone blind fails here.
    let planted = "let value = std::fs::read_to_string(path);";
    assert_eq!(
        other_reads("planted", planted),
        vec!["planted: fs::read_to_string(", "planted: read_to_string("]
    );
}
