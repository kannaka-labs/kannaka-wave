//! The encoder: text into the voice's embedding space, over HTTP to an ollama
//! server, with nothing but the standard library. E-001 ran on
//! `mxbai-embed-large` at 1024 dimensions; the record says the encoder was the
//! recall floor, so which model this points at is the most consequential
//! configuration in the system.
//!
//! Vectors are unit-normalised on the way out, so the substrate's dot product
//! is cosine.

use crate::http::{json_quote, post_json, HttpError};
use crate::{Encoder, Vector};
use std::time::Duration;

/// An embedding model served by ollama's `/api/embeddings`.
#[derive(Debug, Clone)]
pub struct OllamaEncoder {
    host: String,
    port: u16,
    model: String,
    dims: usize,
    timeout: Duration,
}

/// Why an encode failed. The substrate never guesses a vector; a failure here
/// is a failure to remember, reported as such.
#[derive(Debug)]
pub enum EncodeError {
    /// The server could not be reached or the socket failed.
    Io(std::io::Error),
    /// The server answered, but not with an embedding of the configured size.
    Bad(String),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::Io(e) => write!(f, "encoder io: {e}"),
            EncodeError::Bad(s) => write!(f, "encoder response: {s}"),
        }
    }
}

impl std::error::Error for EncodeError {}

impl From<HttpError> for EncodeError {
    fn from(e: HttpError) -> Self {
        match e {
            HttpError::Io(e) => EncodeError::Io(e),
            HttpError::Bad(s) => EncodeError::Bad(s),
        }
    }
}

impl OllamaEncoder {
    /// `host:port` of an ollama server, the model name, and the dimensionality
    /// the model is known to produce. A response of any other size is an error,
    /// never silently padded or truncated.
    pub fn new(host: &str, port: u16, model: &str, dims: usize) -> Self {
        Self {
            host: host.to_string(),
            port,
            model: model.to_string(),
            dims,
            timeout: Duration::from_secs(60),
        }
    }

    /// Socket timeout for one request. Batches of long texts on a busy CPU
    /// can take minutes; the default is 60 s.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Encode, returning the error instead of panicking. `Encoder::encode`
    /// panics on failure because the trait has no error channel; production
    /// callers use this.
    pub fn try_encode(&self, text: &str) -> Result<Vector, EncodeError> {
        let body = format!(
            "{{\"model\":{},\"prompt\":{}}}",
            json_quote(&self.model),
            json_quote(text)
        );
        let payload = post_json(
            &self.host,
            self.port,
            "/api/embeddings",
            &body,
            self.timeout,
        )?;
        let v = parse_embedding(&payload)?;
        if v.len() != self.dims {
            return Err(EncodeError::Bad(format!(
                "expected {} dims, got {}",
                self.dims,
                v.len()
            )));
        }
        Ok(normalize(Vector(v)))
    }

    /// Encode many texts in one request through `/api/embed`, which returns
    /// one vector per input. Same vectors as one at a time; a tenth of the
    /// wall time on CPU. Chunk the input at a few dozen texts.
    pub fn try_encode_batch(&self, texts: &[&str]) -> Result<Vec<Vector>, EncodeError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let inputs: Vec<String> = texts.iter().map(|t| json_quote(t)).collect();
        let body = format!(
            "{{\"model\":{},\"input\":[{}]}}",
            json_quote(&self.model),
            inputs.join(",")
        );
        let payload = post_json(&self.host, self.port, "/api/embed", &body, self.timeout)?;
        let vs = parse_embeddings(&payload)?;
        if vs.len() != texts.len() {
            return Err(EncodeError::Bad(format!(
                "sent {} texts, got {} vectors",
                texts.len(),
                vs.len()
            )));
        }
        let mut out = Vec::with_capacity(vs.len());
        for v in vs {
            if v.len() != self.dims {
                return Err(EncodeError::Bad(format!(
                    "expected {} dims, got {}",
                    self.dims,
                    v.len()
                )));
            }
            out.push(normalize(Vector(v)));
        }
        Ok(out)
    }
}

impl Encoder for OllamaEncoder {
    /// The trait has no error channel, so a transport failure is retried
    /// with backoff (a busy server refuses connections for a moment under
    /// load) and only then is a panic honest: the process cannot remember.
    fn encode(&self, text: &str) -> Vector {
        let mut last = None;
        for attempt in 0..6u32 {
            match self.try_encode(text) {
                Ok(v) => return v,
                Err(e @ EncodeError::Bad(_)) => panic!("encoder failed: {e}"),
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(Duration::from_secs(2u64.pow(attempt)));
                }
            }
        }
        panic!("encoder failed after retries: {}", last.expect("an error"));
    }
    fn dims(&self) -> usize {
        self.dims
    }
}

/// Scale to unit length. A zero vector stays zero rather than becoming NaN.
pub fn normalize(mut v: Vector) -> Vector {
    let n = v.0.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in &mut v.0 {
            *x /= n;
        }
    }
    v
}

/// Pull the `"embeddings":[[…],[…]]` arrays out of `/api/embed`'s reply.
fn parse_embeddings(payload: &str) -> Result<Vec<Vec<f32>>, EncodeError> {
    let key = "\"embeddings\"";
    let at = payload
        .find(key)
        .ok_or_else(|| EncodeError::Bad("no embeddings field".into()))?;
    let after = &payload[at + key.len()..];
    let open = after
        .find('[')
        .ok_or_else(|| EncodeError::Bad("embeddings is not an array".into()))?;
    let mut rest = &after[open + 1..];
    let mut out = Vec::new();
    loop {
        let t = rest.trim_start();
        let t = t.strip_prefix(',').unwrap_or(t).trim_start();
        if t.starts_with(']') || t.is_empty() {
            break;
        }
        let inner_open = t
            .find('[')
            .ok_or_else(|| EncodeError::Bad("embedding row is not an array".into()))?;
        let inner_close = t[inner_open..]
            .find(']')
            .ok_or_else(|| EncodeError::Bad("unterminated embedding row".into()))?;
        let inner = &t[inner_open + 1..inner_open + inner_close];
        let mut v = Vec::new();
        for tok in inner.split(',') {
            let s = tok.trim();
            if s.is_empty() {
                continue;
            }
            v.push(
                s.parse::<f32>()
                    .map_err(|_| EncodeError::Bad(format!("bad number {s:?}")))?,
            );
        }
        out.push(v);
        rest = &t[inner_open + inner_close + 1..];
    }
    Ok(out)
}

/// Pull the `"embedding":[…]` array out of ollama's reply.
fn parse_embedding(payload: &str) -> Result<Vec<f32>, EncodeError> {
    let key = "\"embedding\"";
    let at = payload
        .find(key)
        .ok_or_else(|| EncodeError::Bad("no embedding field".into()))?;
    let after = &payload[at + key.len()..];
    let open = after
        .find('[')
        .ok_or_else(|| EncodeError::Bad("embedding is not an array".into()))?;
    let close = after[open..]
        .find(']')
        .ok_or_else(|| EncodeError::Bad("unterminated embedding array".into()))?;
    let inner = &after[open + 1..open + close];
    let mut v = Vec::new();
    for tok in inner.split(',') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        let x: f32 = t
            .parse()
            .map_err(|_| EncodeError::Bad(format!("bad number {t:?}")))?;
        v.push(x);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::testing::{ok, serve_once};

    #[test]
    fn encodes_and_normalises_a_reply() {
        let (port, seen) = serve_once(Box::leak(
            ok("{\"embedding\":[3.0,4.0,0.0]}").into_boxed_str(),
        ));
        let e = OllamaEncoder::new("127.0.0.1", port, "m", 3);
        let v = e.try_encode("hello \"there\"\n").unwrap();
        assert!((v.0[0] - 0.6).abs() < 1e-6 && (v.0[1] - 0.8).abs() < 1e-6);
        let req = seen.join().unwrap();
        assert!(req.starts_with("POST /api/embeddings HTTP/1.1"));
        assert!(req.ends_with("{\"model\":\"m\",\"prompt\":\"hello \\\"there\\\"\\n\"}"));
    }

    #[test]
    fn a_batch_comes_back_in_order_and_normalised() {
        let (port, seen) = serve_once(Box::leak(
            ok("{\"model\":\"m\",\"embeddings\":[[3.0,4.0,0.0],[0.0,0.0,2.0]],\"total_duration\":1}")
                .into_boxed_str(),
        ));
        let e = OllamaEncoder::new("127.0.0.1", port, "m", 3);
        let vs = e.try_encode_batch(&["a", "b"]).unwrap();
        assert_eq!(vs.len(), 2);
        assert!((vs[0].0[1] - 0.8).abs() < 1e-6);
        assert_eq!(vs[1].0, vec![0.0, 0.0, 1.0]);
        let req = seen.join().unwrap();
        assert!(req.starts_with("POST /api/embed HTTP/1.1"));
        assert!(req.ends_with("{\"model\":\"m\",\"input\":[\"a\",\"b\"]}"));
        let (port, _) = serve_once(Box::leak(ok("{\"embeddings\":[[1,0,0]]}").into_boxed_str()));
        let e = OllamaEncoder::new("127.0.0.1", port, "m", 3);
        assert!(matches!(
            e.try_encode_batch(&["a", "b"]),
            Err(EncodeError::Bad(_))
        ));
        assert!(e.try_encode_batch(&[]).unwrap().is_empty());
    }

    #[test]
    fn wrong_dimensionality_is_an_error_not_a_vector() {
        let (port, _) = serve_once(Box::leak(ok("{\"embedding\":[1,2]}").into_boxed_str()));
        let e = OllamaEncoder::new("127.0.0.1", port, "m", 3);
        assert!(matches!(e.try_encode("x"), Err(EncodeError::Bad(_))));
    }

    #[test]
    fn chunked_replies_and_server_errors() {
        let (port, _) = serve_once(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
             10\r\n{\"embedding\":[1,\r\n8\r\n0,0,0]}\r\n\r\n0\r\n\r\n",
        );
        let e = OllamaEncoder::new("127.0.0.1", port, "m", 4);
        assert_eq!(e.try_encode("x").unwrap().0, vec![1.0, 0.0, 0.0, 0.0]);
        let (port, _) = serve_once("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        let e = OllamaEncoder::new("127.0.0.1", port, "m", 4);
        assert!(matches!(e.try_encode("x"), Err(EncodeError::Bad(_))));
    }
}
