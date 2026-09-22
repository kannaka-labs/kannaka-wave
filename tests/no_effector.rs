//! The conscience's structural invariant, ported from kannaka-steward's
//! `invariant-no-wallet.test.ts`: nothing in this crate can sign, spend,
//! spawn a process, or open a connection outside the one HTTP client. The
//! rails emit packages; the effector is somewhere else, carried there by a
//! person. This test makes that a property of the source rather than a
//! promise in a comment.
//!
//! It scans code, not prose: comments are stripped first, so the documents
//! that say "there is no wallet" are allowed. String literals are kept. The
//! positive control proves every rule fires; the negative control proves the
//! same words in comments and inside longer identifiers do not.

use std::fs;
use std::path::{Path, PathBuf};

/// A forbidden construct and what it would let the crate do.
struct Rule {
    token: &'static str,
    why: &'static str,
}

const FORBIDDEN: &[Rule] = &[
    Rule {
        token: "Command",
        why: "spawn a process (std::process::Command)",
    },
    Rule {
        token: "Wallet",
        why: "hold a signing wallet",
    },
    Rule {
        token: "private_key",
        why: "hold a signing secret",
    },
    Rule {
        token: "secret_key",
        why: "hold a signing secret",
    },
    Rule {
        token: "mnemonic",
        why: "hold a seed phrase",
    },
    Rule {
        token: "sign_transaction",
        why: "sign",
    },
    Rule {
        token: "send_transaction",
        why: "broadcast",
    },
    Rule {
        token: "UdpSocket",
        why: "open a socket",
    },
];

/// Network primitives, allowed in exactly one file: the crate's one HTTP
/// client, which speaks to the encoder and voice (ADR-0001 §Status).
const NETWORK: &[&str] = &["TcpStream", "TcpListener"];
const NETWORK_HOME: &str = "src/http.rs";

/// Remove `//` and `/* */` comments (nested, as Rust allows), keeping string
/// and char literals and every newline, so line numbers still point true.
fn strip_comments(src: &str) -> String {
    let c: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '/' if c.get(i + 1) == Some(&'/') => {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
            }
            '/' if c.get(i + 1) == Some(&'*') => {
                let mut depth = 1;
                i += 2;
                while i < c.len() && depth > 0 {
                    if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                        depth += 1;
                        i += 2;
                    } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        if c[i] == '\n' {
                            out.push('\n');
                        }
                        i += 1;
                    }
                }
            }
            '"' => {
                out.push('"');
                i += 1;
                while i < c.len() && c[i] != '"' {
                    if c[i] == '\\' && i + 1 < c.len() {
                        out.push(c[i]);
                        i += 1;
                    }
                    out.push(c[i]);
                    i += 1;
                }
                if i < c.len() {
                    out.push('"');
                    i += 1;
                }
            }
            // A char literal ('"', '\n', 'x'); otherwise a lifetime.
            '\'' if c.get(i + 1) == Some(&'\\') => {
                let end = (i + 2..c.len())
                    .find(|&j| c[j] == '\'')
                    .unwrap_or(c.len() - 1);
                out.extend(&c[i..=end]);
                i = end + 1;
            }
            '\'' if c.get(i + 2) == Some(&'\'') => {
                out.extend(&c[i..i + 3]);
                i += 3;
            }
            ch => {
                out.push(ch);
                i += 1;
            }
        }
    }
    out
}

fn is_ident(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Line numbers where `token` occurs as a whole identifier.
fn occurrences(code: &str, token: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let mut from = 0;
        while let Some(off) = line[from..].find(token) {
            let at = from + off;
            let before = line[..at].chars().next_back();
            let after = line[at + token.len()..].chars().next();
            if !before.is_some_and(is_ident) && !after.is_some_and(is_ident) {
                hits.push(n + 1);
            }
            from = at + token.len();
        }
    }
    hits
}

fn scan(code: &str) -> Vec<(usize, &'static str)> {
    let code = strip_comments(code);
    let mut hits = Vec::new();
    for r in FORBIDDEN {
        for n in occurrences(&code, r.token) {
            hits.push((n, r.token));
        }
    }
    hits
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable src/") {
        let p = entry.expect("dir entry").path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let rel = p
                .strip_prefix(root)
                .expect("under the crate")
                .to_string_lossy()
                .replace('\\', "/");
            (rel, fs::read_to_string(&p).expect("utf-8 source"))
        })
        .collect()
}

#[test]
fn the_scan_has_something_to_scan() {
    let files = sources();
    assert!(files.iter().any(|(p, _)| p == "src/conscience.rs"));
    assert!(files.iter().any(|(p, _)| p == "src/bin/wave.rs"));
}

#[test]
fn no_source_file_can_sign_spend_or_spawn() {
    let mut violations = Vec::new();
    for (path, src) in sources() {
        for (line, token) in scan(&src) {
            let why = FORBIDDEN.iter().find(|r| r.token == token).unwrap().why;
            violations.push(format!(
                "{path}:{line}  {token}  (would let the crate {why})"
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "\nThe conscience emits packages; it never acts. Forbidden constructs in src/:\n{}\n",
        violations.join("\n")
    );
}

#[test]
fn only_the_one_http_client_opens_a_connection() {
    let mut violations = Vec::new();
    for (path, src) in sources() {
        if path == NETWORK_HOME {
            continue;
        }
        let code = strip_comments(&src);
        for token in NETWORK {
            for line in occurrences(&code, token) {
                violations.push(format!("{path}:{line}  {token}"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "\nA network primitive outside {NETWORK_HOME}. Every connection goes through the one client:\n{}\n",
        violations.join("\n")
    );
}

#[test]
fn positive_control_every_rule_fires() {
    let malicious = "\
use std::process::Command;
let w = Wallet::new();
let k = account.private_key;
let s = secret_key(&seed);
let m = mnemonic.words();
tx.sign_transaction(k);
node.send_transaction(tx);
let u = UdpSocket::bind(addr);
";
    let fired: std::collections::HashSet<&str> =
        scan(malicious).into_iter().map(|(_, t)| t).collect();
    for r in FORBIDDEN {
        assert!(fired.contains(r.token), "rule {} did not fire", r.token);
    }
}

#[test]
fn negative_control_prose_and_longer_identifiers_are_not_flagged() {
    let prose = "\
// There is no Wallet here, no private_key, no mnemonic, and no Command.
/* No sign_transaction. /* nested: no send_transaction */ Ever. */
/// The effector is somewhere else; a person carries the package to it.
let bound_wallet_address = \"https://example.com/wallet\"; // a Wallet's address
let commander = CommandLine::default();
let quote = '\"'; let s = \"a // not a comment\"; let lt: &'static str = \"\";
";
    assert_eq!(scan(prose), Vec::<(usize, &str)>::new());
}

#[test]
fn line_numbers_survive_comment_stripping() {
    let src = "/* one\n two\n three */\nlet c = Command::new(\"x\");\n";
    assert_eq!(scan(src), vec![(4, "Command")]);
}
