//! Resource fetching with the plan's size caps (§11): `file://` paths,
//! `data:` URLs and `http(s)` via ureq. Per resource 20 MB, per page
//! 100 MB, at most 200 subresources.

use std::fmt;
use std::io::Read;
use std::time::Duration;

use url::Url;

/// Size and count caps for one page load.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Bytes per resource.
    pub per_resource: usize,
    /// Bytes per page (all resources together).
    pub per_page: usize,
    /// Number of subresources (the document itself is not counted).
    pub max_subresources: usize,
    /// Wall-clock timeout for one HTTP request.
    pub timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            per_resource: 20 * 1024 * 1024,
            per_page: 100 * 1024 * 1024,
            max_subresources: 200,
            timeout: Duration::from_secs(30),
        }
    }
}

/// Why a fetch failed. Mirrors the `Error.kind` list of plan §2.2.
#[derive(Debug)]
pub enum FetchError {
    /// Scheme is not `http`, `https`, `file` or `data`.
    UnsupportedScheme(String),
    /// The URL could not be parsed or resolved.
    BadUrl(String),
    /// A resource exceeded [`Limits::per_resource`] or the page total.
    TooLarge(String),
    /// More than [`Limits::max_subresources`] were requested.
    TooMany,
    /// HTTP status outside 2xx.
    Http(u16),
    /// Request timed out.
    Timeout,
    /// Anything else from the transport.
    Transport(String),
    /// Local file error.
    Io(std::io::Error),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::UnsupportedScheme(s) => write!(f, "unsupported URL scheme {s:?}"),
            FetchError::BadUrl(s) => write!(f, "bad URL: {s}"),
            FetchError::TooLarge(s) => write!(f, "resource too large: {s}"),
            FetchError::TooMany => write!(f, "too many subresources"),
            FetchError::Http(code) => write!(f, "HTTP status {code}"),
            FetchError::Timeout => write!(f, "request timed out"),
            FetchError::Transport(s) => write!(f, "transport error: {s}"),
            FetchError::Io(e) => write!(f, "i/o error: {e}"),
        }
    }
}

impl std::error::Error for FetchError {}

/// A fetched resource.
#[derive(Clone, Debug)]
pub struct Resource {
    /// URL after redirects.
    pub url: Url,
    /// Raw bytes (content-encoding already removed).
    pub bytes: Vec<u8>,
    /// `Content-Type` (or the `data:` media type), lower-cased, without
    /// parameters.
    pub mime: Option<String>,
}

/// Fetches resources for one page load, enforcing [`Limits`].
pub struct Fetcher {
    agent: ureq::Agent,
    limits: Limits,
    total: usize,
    subresources: usize,
}

impl Fetcher {
    /// A fetcher with the given limits.
    pub fn new(limits: Limits) -> Fetcher {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(10)
            .timeout_global(Some(limits.timeout))
            .user_agent("lean-browser/0.1 (document mode)")
            .build();
        Fetcher {
            agent: ureq::Agent::new_with_config(config),
            limits,
            total: 0,
            subresources: 0,
        }
    }

    /// Bytes fetched so far.
    pub fn total_bytes(&self) -> usize {
        self.total
    }

    /// Fetches the main document (not counted against the subresource cap).
    pub fn fetch_document(&mut self, url: &Url) -> Result<Resource, FetchError> {
        self.fetch_inner(url)
    }

    /// Fetches a subresource (stylesheet, image), counted against the cap.
    pub fn fetch_subresource(&mut self, url: &Url) -> Result<Resource, FetchError> {
        if self.subresources >= self.limits.max_subresources {
            return Err(FetchError::TooMany);
        }
        self.subresources += 1;
        self.fetch_inner(url)
    }

    fn fetch_inner(&mut self, url: &Url) -> Result<Resource, FetchError> {
        let budget = self
            .limits
            .per_resource
            .min(self.limits.per_page.saturating_sub(self.total));
        let res = match url.scheme() {
            "file" => fetch_file(url, budget),
            "data" => fetch_data(url, budget),
            "http" | "https" => self.fetch_http(url, budget),
            other => Err(FetchError::UnsupportedScheme(other.to_string())),
        }?;
        self.total += res.bytes.len();
        Ok(res)
    }

    fn fetch_http(&self, url: &Url, budget: usize) -> Result<Resource, FetchError> {
        let mut response = self
            .agent
            .get(url.as_str())
            .call()
            .map_err(map_ureq_error)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(FetchError::Http(status));
        }
        let final_url = {
            use ureq::ResponseExt;
            Url::parse(&response.get_uri().to_string()).unwrap_or_else(|_| url.clone())
        };
        let mime = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(normalize_mime);
        // Read one byte past the budget so an over-limit body is detected.
        let bytes = response
            .body_mut()
            .with_config()
            .limit(budget as u64 + 1)
            .read_to_vec()
            .map_err(map_ureq_error)?;
        if bytes.len() > budget {
            return Err(FetchError::TooLarge(url.to_string()));
        }
        Ok(Resource {
            url: final_url,
            bytes,
            mime,
        })
    }
}

fn map_ureq_error(e: ureq::Error) -> FetchError {
    match e {
        ureq::Error::StatusCode(c) => FetchError::Http(c),
        ureq::Error::Timeout(_) => FetchError::Timeout,
        ureq::Error::BodyExceedsLimit(_) => FetchError::TooLarge("body exceeds limit".into()),
        other => FetchError::Transport(other.to_string()),
    }
}

fn fetch_file(url: &Url, budget: usize) -> Result<Resource, FetchError> {
    let path = url
        .to_file_path()
        .map_err(|_| FetchError::BadUrl(url.to_string()))?;
    let file = std::fs::File::open(&path).map_err(FetchError::Io)?;
    let mut bytes = Vec::new();
    file.take(budget as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(FetchError::Io)?;
    if bytes.len() > budget {
        return Err(FetchError::TooLarge(url.to_string()));
    }
    let mime = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .and_then(|e| {
            Some(
                match e.as_str() {
                    "html" | "htm" | "xhtml" => "text/html",
                    "css" => "text/css",
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    "svg" => "image/svg+xml",
                    "txt" => "text/plain",
                    _ => return None,
                }
                .to_string(),
            )
        });
    Ok(Resource {
        url: url.clone(),
        bytes,
        mime,
    })
}

/// `data:[<mediatype>][;base64],<data>`
fn fetch_data(url: &Url, budget: usize) -> Result<Resource, FetchError> {
    let body = url.path();
    let (meta, data) = body
        .split_once(',')
        .ok_or_else(|| FetchError::BadUrl("data URL without a comma".into()))?;
    let (mime, is_base64) = match meta.strip_suffix(";base64") {
        Some(m) => (m, true),
        None => (meta, false),
    };
    let decoded = if is_base64 {
        base64_decode(data).ok_or_else(|| FetchError::BadUrl("invalid base64".into()))?
    } else {
        percent_decode(data)
    };
    if decoded.len() > budget {
        return Err(FetchError::TooLarge("data URL".into()));
    }
    let mime = if mime.is_empty() {
        Some("text/plain".to_string())
    } else {
        Some(normalize_mime(mime))
    };
    Ok(Resource {
        url: url.clone(),
        bytes: decoded,
        mime,
    })
}

fn normalize_mime(s: &str) -> String {
    s.split(';').next().unwrap_or("").trim().to_ascii_lowercase()
}

fn percent_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hi = (b[i + 1] as char).to_digit(16);
            let lo = (b[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// Standard base64 (with or without padding); whitespace is skipped.
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits = 0;
    for &c in s.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        acc = (acc << 6) | val(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Turns a command-line argument into a URL: absolute URLs are used as is,
/// anything else is a filesystem path.
pub fn url_from_arg(arg: &str) -> Result<Url, FetchError> {
    if let Ok(u) = Url::parse(arg) {
        if u.scheme().len() > 1 {
            return Ok(u);
        }
    }
    let path = std::path::Path::new(arg);
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(FetchError::Io)?
            .join(path)
    };
    Url::from_file_path(&abs).map_err(|_| FetchError::BadUrl(arg.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGVsbG8").unwrap(), b"hello");
        assert_eq!(base64_decode("aGVs\nbG8=").unwrap(), b"hello");
        assert!(base64_decode("a*").is_none());
    }

    #[test]
    fn data_urls() {
        let u = Url::parse("data:text/plain;base64,aGVsbG8=").unwrap();
        let r = fetch_data(&u, 1000).unwrap();
        assert_eq!(r.bytes, b"hello");
        assert_eq!(r.mime.as_deref(), Some("text/plain"));
        let u = Url::parse("data:,a%20b").unwrap();
        assert_eq!(fetch_data(&u, 1000).unwrap().bytes, b"a b");
        assert!(matches!(
            fetch_data(&u, 1),
            Err(FetchError::TooLarge(_))
        ));
    }

    #[test]
    fn file_limits() {
        let dir = std::env::temp_dir().join(format!("lean-loader-fetch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.html");
        std::fs::write(&p, b"<p>hi</p>").unwrap();
        let url = Url::from_file_path(&p).unwrap();
        let mut f = Fetcher::new(Limits {
            per_resource: 4,
            ..Limits::default()
        });
        assert!(matches!(
            f.fetch_document(&url),
            Err(FetchError::TooLarge(_))
        ));
        let mut f = Fetcher::new(Limits::default());
        let r = f.fetch_document(&url).unwrap();
        assert_eq!(r.mime.as_deref(), Some("text/html"));
        assert_eq!(r.bytes, b"<p>hi</p>");
        let mut f = Fetcher::new(Limits {
            max_subresources: 0,
            ..Limits::default()
        });
        assert!(matches!(f.fetch_subresource(&url), Err(FetchError::TooMany)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn http_fetch_over_loopback() {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut s, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let body: &[u8] = if req.starts_with("GET /big") {
                    &[b'x'; 64]
                } else {
                    b"<p>net</p>"
                };
                let status = if req.starts_with("GET /missing") {
                    "404 Not Found"
                } else {
                    "200 OK"
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                s.write_all(head.as_bytes()).unwrap();
                s.write_all(body).unwrap();
            }
        });
        let mut f = Fetcher::new(Limits {
            per_resource: 32,
            ..Limits::default()
        });
        let ok = f
            .fetch_document(&Url::parse(&format!("http://127.0.0.1:{port}/a")).unwrap())
            .unwrap();
        assert_eq!(ok.bytes, b"<p>net</p>");
        assert_eq!(ok.mime.as_deref(), Some("text/html"));
        let big = f.fetch_document(&Url::parse(&format!("http://127.0.0.1:{port}/big")).unwrap());
        assert!(matches!(big, Err(FetchError::TooLarge(_))), "{big:?}");
        server.join().unwrap();
    }

    #[test]
    fn arg_to_url() {
        assert_eq!(
            url_from_arg("https://example.test/x").unwrap().as_str(),
            "https://example.test/x"
        );
        let u = url_from_arg("some/relative.html").unwrap();
        assert_eq!(u.scheme(), "file");
        assert!(u.path().ends_with("/some/relative.html"));
    }
}
