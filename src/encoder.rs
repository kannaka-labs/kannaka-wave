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
}

impl Encoder for OllamaEncoder {
    fn encode(&self, text: &str) -> Vector {
        match self.try_encode(text) {
            Ok(v) => v,
            Err(e) => panic!("encoder failed: {e}"),
        }
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
