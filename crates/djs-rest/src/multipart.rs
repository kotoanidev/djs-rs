//! Request bodies: JSON, or `multipart/form-data` with files (D-087, D-093, D-095).
//!
//! A multipart body is a list of [`Bytes`] chunks — part headers, then each file's data as
//! is — so file data is never copied, not even for retries (cloning a chunk only bumps a
//! reference count).

use core::{
    convert::Infallible,
    fmt::Write as _,
    pin::Pin,
    task::{Context, Poll},
};
use std::hash::{BuildHasher, RandomState};

use bytes::Bytes;
use djs_model::rest::FileUpload;
use http::HeaderValue;
use hyper::body::{Body, Frame, SizeHint};

/// The content type of files whose type is neither given nor known.
const OCTET_STREAM: &str = "application/octet-stream";

/// A request body built once and sent as many times as the request is attempted.
#[derive(Clone, Debug, Default)]
pub(crate) struct PreparedBody {
    chunks: Vec<Bytes>,
    content_type: Option<HeaderValue>,
}

impl PreparedBody {
    /// A body of `json` alone, or — with files — a multipart body with the files as
    /// `files[n]` and `json` as `payload_json`.
    pub(crate) fn new(json: Option<Bytes>, files: &[FileUpload]) -> Self {
        if files.is_empty() {
            return Self {
                content_type: json
                    .is_some()
                    .then(|| HeaderValue::from_static("application/json")),
                chunks: json.into_iter().collect(),
            };
        }
        multipart(json, files, &boundary())
    }

    /// The `Content-Type` of the body, if it has one.
    pub(crate) const fn content_type(&self) -> Option<&HeaderValue> {
        self.content_type.as_ref()
    }

    /// A fresh body to send.
    pub(crate) fn body(&self) -> RequestBody {
        RequestBody {
            remaining: self.chunks.iter().map(|chunk| chunk.len() as u64).sum(),
            chunks: self.chunks.clone().into_iter(),
        }
    }
}

/// The body of an HTTP request: a sequence of chunks of known total length (so the
/// `Content-Length` header is set).
#[derive(Debug)]
pub(crate) struct RequestBody {
    chunks: std::vec::IntoIter<Bytes>,
    remaining: u64,
}

impl Body for RequestBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        let this = self.get_mut();
        let next = this.chunks.by_ref().find(|chunk| !chunk.is_empty());
        if let Some(chunk) = &next {
            this.remaining -= chunk.len() as u64;
        }
        Poll::Ready(next.map(|chunk| Ok(Frame::data(chunk))))
    }

    fn is_end_stream(&self) -> bool {
        self.remaining == 0
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.remaining)
    }
}

/// A random multipart boundary (128 bits from the standard library's per-process random
/// keys), which file data cannot contain except by chance.
fn boundary() -> String {
    let random = RandomState::new();
    format!(
        "djs-rs-{:016x}{:016x}",
        random.hash_one(0_u8),
        random.hash_one(1_u8)
    )
}

/// Builds a multipart body (RFC 7578): every file as `files[n]`, then `payload_json`.
fn multipart(json: Option<Bytes>, files: &[FileUpload], boundary: &str) -> PreparedBody {
    let mut chunks = Vec::with_capacity(files.len() * 2 + 3);
    // Text between data chunks: the end of the previous part and the next part's headers.
    let mut text = String::with_capacity(256);
    for (index, file) in files.iter().enumerate() {
        if index > 0 {
            text.push_str("\r\n");
        }
        // Writing to a `String` cannot fail.
        let _ = write!(
            text,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"files[{index}]\"; filename=\""
        );
        write_quoted(&mut text, &file.name);
        text.push_str("\"\r\nContent-Type: ");
        text.push_str(content_type(file));
        text.push_str("\r\n\r\n");
        chunks.push(Bytes::from(core::mem::take(&mut text)));
        chunks.push(file.data.clone());
    }
    text.push_str("\r\n");
    if let Some(json) = json {
        let _ = write!(
            text,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"payload_json\"\r\n\
             Content-Type: application/json\r\n\r\n"
        );
        chunks.push(Bytes::from(core::mem::take(&mut text)));
        chunks.push(json);
        text.push_str("\r\n");
    }
    let _ = write!(text, "--{boundary}--\r\n");
    chunks.push(Bytes::from(text));

    // The boundary is ASCII letters, digits and `-`, so the value is always valid.
    let content_type = HeaderValue::try_from(format!("multipart/form-data; boundary={boundary}"))
        .unwrap_or_else(|_| HeaderValue::from_static("multipart/form-data"));
    PreparedBody {
        chunks,
        content_type: Some(content_type),
    }
}

/// Writes a file name inside a quoted header parameter, escaping `"`, CR and LF like
/// browsers do (WHATWG `multipart/form-data` encoding), so the name cannot end the parameter
/// or add header lines.
fn write_quoted(out: &mut String, name: &str) {
    for character in name.chars() {
        match character {
            '"' => out.push_str("%22"),
            '\r' => out.push_str("%0D"),
            '\n' => out.push_str("%0A"),
            _ => out.push(character),
        }
    }
}

/// The content type of a file part: the given one if it is a valid single-line value, else
/// a guess from the file name.
fn content_type(file: &FileUpload) -> &str {
    match &file.content_type {
        Some(given)
            if !given.is_empty() && given.bytes().all(|byte| (b' '..=b'~').contains(&byte)) =>
        {
            given
        }
        _ => guess_content_type(&file.name),
    }
}

/// Guesses a content type from a file name extension (D-095), case-insensitively.
fn guess_content_type(name: &str) -> &'static str {
    let Some((_, extension)) = name.rsplit_once('.') else {
        return OCTET_STREAM;
    };
    let mut lower = [0_u8; 4];
    let Some(target) = lower.get_mut(..extension.len()) else {
        return OCTET_STREAM;
    };
    target.copy_from_slice(extension.as_bytes());
    target.make_ascii_lowercase();
    match &*target {
        // Images.
        b"png" => "image/png",
        b"apng" => "image/apng",
        b"jpg" | b"jpeg" | b"jfif" => "image/jpeg",
        b"gif" => "image/gif",
        b"webp" => "image/webp",
        b"avif" => "image/avif",
        b"heic" => "image/heic",
        b"heif" => "image/heif",
        b"bmp" => "image/bmp",
        b"svg" => "image/svg+xml",
        b"tif" | b"tiff" => "image/tiff",
        b"ico" => "image/x-icon",
        // Video.
        b"mp4" | b"m4v" => "video/mp4",
        b"webm" => "video/webm",
        b"mov" => "video/quicktime",
        b"mkv" => "video/x-matroska",
        b"avi" => "video/x-msvideo",
        // Audio (`.opus` files are Opus in an Ogg container, like voice messages).
        b"mp3" => "audio/mpeg",
        b"ogg" | b"oga" | b"opus" => "audio/ogg",
        b"wav" => "audio/wav",
        b"flac" => "audio/flac",
        b"m4a" => "audio/mp4",
        b"aac" => "audio/aac",
        // Text (as UTF-8, which is what Discord's text previews expect).
        b"txt" | b"log" => "text/plain; charset=utf-8",
        b"md" => "text/markdown; charset=utf-8",
        b"csv" => "text/csv; charset=utf-8",
        b"htm" | b"html" => "text/html; charset=utf-8",
        b"css" => "text/css; charset=utf-8",
        b"js" | b"mjs" => "text/javascript; charset=utf-8",
        b"json" => "application/json",
        b"xml" => "application/xml",
        b"yml" | b"yaml" => "application/yaml",
        b"toml" => "application/toml",
        // Documents and archives.
        b"pdf" => "application/pdf",
        b"zip" => "application/zip",
        b"gz" => "application/gzip",
        b"tar" => "application/x-tar",
        b"wasm" => "application/wasm",
        _ => OCTET_STREAM,
    }
}

#[cfg(test)]
mod tests {
    use http_body_util::BodyExt;

    use super::*;

    async fn collect(body: &PreparedBody) -> (Vec<Bytes>, Vec<u8>) {
        let mut frames = Vec::new();
        let mut all = Vec::new();
        let mut body = body.body();
        while let Some(frame) = body.frame().await {
            let data = frame.unwrap().into_data().unwrap();
            all.extend_from_slice(&data);
            frames.push(data);
        }
        (frames, all)
    }

    #[tokio::test]
    async fn json_only() {
        let body = PreparedBody::new(Some(Bytes::from_static(b"{}")), &[]);
        assert_eq!(body.content_type().unwrap(), "application/json");
        assert_eq!(body.body().size_hint().exact(), Some(2));
        assert_eq!(collect(&body).await.1, b"{}");

        let empty = PreparedBody::new(None, &[]);
        assert!(empty.content_type().is_none());
        assert!(empty.body().is_end_stream());
        assert_eq!(collect(&empty).await.0.len(), 0);
    }

    #[tokio::test]
    async fn multipart_layout() {
        let image = FileUpload::new("ferris.png", Bytes::from_static(b"PNGDATA"));
        let mut text = FileUpload::new("a\"b\r\n.txt", "hello");
        text.content_type = Some("text/x-custom".into());
        let files = [image, text];
        let body = multipart(
            Some(Bytes::from_static(b"{\"content\":\"hi\"}")),
            &files,
            "XYZ",
        );
        assert_eq!(
            body.content_type().unwrap(),
            "multipart/form-data; boundary=XYZ"
        );
        let (frames, all) = collect(&body).await;
        assert_eq!(
            String::from_utf8(all).unwrap(),
            "--XYZ\r\n\
             Content-Disposition: form-data; name=\"files[0]\"; filename=\"ferris.png\"\r\n\
             Content-Type: image/png\r\n\r\n\
             PNGDATA\r\n\
             --XYZ\r\n\
             Content-Disposition: form-data; name=\"files[1]\"; filename=\"a%22b%0D%0A.txt\"\r\n\
             Content-Type: text/x-custom\r\n\r\n\
             hello\r\n\
             --XYZ\r\n\
             Content-Disposition: form-data; name=\"payload_json\"\r\n\
             Content-Type: application/json\r\n\r\n\
             {\"content\":\"hi\"}\r\n\
             --XYZ--\r\n"
        );
        // The file data is sent as is: the same allocation, not a copy.
        assert_eq!(frames[1].as_ptr(), files[0].data.as_ptr());
        assert_eq!(frames[3].as_ptr(), files[1].data.as_ptr());
        // Every attempt gets the whole body again.
        assert_eq!(collect(&body).await.0.len(), frames.len());
        let length: usize = frames.iter().map(Bytes::len).sum();
        assert_eq!(body.body().size_hint().exact(), Some(length as u64));
    }

    #[tokio::test]
    async fn multipart_without_json_and_with_empty_file() {
        let files = [FileUpload::new("empty.bin", Bytes::new())];
        let body = multipart(None, &files, "B");
        assert_eq!(
            String::from_utf8(collect(&body).await.1).unwrap(),
            "--B\r\nContent-Disposition: form-data; name=\"files[0]\"; filename=\"empty.bin\"\r\n\
             Content-Type: application/octet-stream\r\n\r\n\r\n--B--\r\n"
        );
    }

    #[test]
    fn boundaries_are_random() {
        let a = boundary();
        let b = boundary();
        assert_ne!(a, b);
        assert_eq!(a.len(), "djs-rs-".len() + 32);
        assert!(
            a.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        );
    }

    #[test]
    fn content_types() {
        for (name, expected) in [
            ("ferris.PNG", "image/png"),
            ("photo.jpeg", "image/jpeg"),
            ("voice-message.ogg", "audio/ogg"),
            ("clip.webm", "video/webm"),
            ("notes.txt", "text/plain; charset=utf-8"),
            ("archive.tar.gz", "application/gzip"),
            ("no-extension", OCTET_STREAM),
            ("trailing.", OCTET_STREAM),
            ("long.extension", OCTET_STREAM),
            ("unknown.xyz", OCTET_STREAM),
            ("ünïcode.png", "image/png"),
            ("dot.ñ", OCTET_STREAM),
        ] {
            assert_eq!(guess_content_type(name), expected, "{name}");
        }

        // A given type is used unless it could break the part headers.
        let mut file = FileUpload::new("data.png", "x");
        file.content_type = Some("image/x-special".into());
        assert_eq!(content_type(&file), "image/x-special");
        file.content_type = Some("text/plain\r\nX-Evil: 1".into());
        assert_eq!(content_type(&file), "image/png");
        file.content_type = Some(String::new().into());
        assert_eq!(content_type(&file), "image/png");
    }
}
