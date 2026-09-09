//! The one HTTP client in the crate: a blocking `POST` of a JSON body over
//! `std::net`, and the two JSON operations the organs need (quote a string,
//! pull one string field out of a reply). No library, on purpose; the organs
//! talk to one server (ollama) whose replies are small and flat.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Why a request failed.
#[derive(Debug)]
pub enum HttpError {
    /// Connect, write or read failed.
    Io(std::io::Error),
    /// The server answered with something other than 200, or unparseably.
    Bad(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Io(e) => write!(f, "io: {e}"),
            HttpError::Bad(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for HttpError {}

impl From<std::io::Error> for HttpError {
    fn from(e: std::io::Error) -> Self {
        HttpError::Io(e)
    }
}

/// `POST path` with a JSON body; returns the response body on 200.
pub fn post_json(
    host: &str,
    port: u16,
    path: &str,
    body: &str,
    timeout: Duration,
) -> Result<String, HttpError> {
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut s = TcpStream::connect((host, port))?;
    s.set_read_timeout(Some(timeout))?;
    s.set_write_timeout(Some(timeout))?;
    s.write_all(req.as_bytes())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw);
    let (head, payload) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| HttpError::Bad("no http header terminator".into()))?;
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        let line = head.lines().next().unwrap_or("");
        return Err(HttpError::Bad(format!("status {line}")));
    }
    let chunked = head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked");
    Ok(if chunked {
        dechunk(payload)
    } else {
        payload.to_string()
    })
}

/// Quote a string as a JSON literal.
pub fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The value of the first top-level-looking `"key":"…"` string field, with
/// JSON escapes (including `\uXXXX` surrogate pairs) decoded.
pub fn json_string_field(payload: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let at = payload.find(&needle)?;
    let rest = &payload[at + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                '/' => out.push('/'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'u' => {
                    let mut code = hex4(&mut chars)?;
                    if (0xD800..0xDC00).contains(&code) {
                        // High surrogate; the low one must follow as \uXXXX.
                        if chars.next()? != '\\' || chars.next()? != 'u' {
                            return None;
                        }
                        let low = hex4(&mut chars)?;
                        code = 0x10000 + ((code - 0xD800) << 10) + (low.checked_sub(0xDC00)?);
                    }
                    out.push(char::from_u32(code)?);
                }
                _ => return None,
            },
            c => out.push(c),
        }
    }
    None
}

fn hex4(chars: &mut std::str::Chars<'_>) -> Option<u32> {
    let mut v = 0u32;
    for _ in 0..4 {
        v = (v << 4) | chars.next()?.to_digit(16)?;
    }
    Some(v)
}

fn dechunk(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some((size_line, after)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size_line.trim().split(';').next().unwrap_or("0"), 16)
            .unwrap_or(0);
        if size == 0 {
            break;
        }
        if after.len() < size {
            out.push_str(after);
            break;
        }
        out.push_str(&after[..size]);
        rest = after[size..].strip_prefix("\r\n").unwrap_or(&after[size..]);
    }
    out
}

#[cfg(test)]
pub(crate) mod testing {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Serve one canned reply on a loopback port; returns the port and a
    /// handle whose join yields the request the server saw.
    pub fn serve_once(reply: &'static str) -> (u16, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = vec![0u8; 1 << 16];
            let n = s.read(&mut buf).unwrap_or(0);
            s.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&buf[..n]).to_string()
        });
        (port, h)
    }

    /// A 200 with a JSON body, content-length framed.
    pub fn ok(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_unquotes() {
        assert_eq!(json_quote("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
        let p = r#"{"model":"m","response":"line one\nsaid \"hi\" é 🚀 end","done":true}"#;
        assert_eq!(
            json_string_field(p, "response").unwrap(),
            "line one\nsaid \"hi\" é 🚀 end"
        );
        assert_eq!(json_string_field(p, "model").unwrap(), "m");
        assert!(json_string_field(p, "missing").is_none());
    }

    #[test]
    fn chunked_bodies_are_reassembled() {
        assert_eq!(
            dechunk("5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"),
            "hello world"
        );
    }
}
