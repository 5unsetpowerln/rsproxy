use anyhow::{Context, Result};
use http::header::CONTENT_ENCODING;
use hyper::{Body, HeaderMap, Method, Request, Uri, Version};
use serde::{Deserialize, Serialize};
use std::str::{self, FromStr};

use crate::proxy::body::clone_body;

use super::body::{decode_body, BodyForFrontend};
use super::compression::CompressionEncoding;
use super::headers::{Header, HeaderMapUtil, VersionUtil};

#[derive(Serialize, Deserialize, Clone)]
pub struct RequestForFrontend {
    id: usize,
    headers: Vec<Header>,
    uri: String,
    method: String,
    version: String,
    body: BodyForFrontend,
}

impl RequestForFrontend {
    pub async fn from_hyper(request: Request<Body>, id: usize) -> Result<Self> {
        let (parts, body) = request.into_parts();

        let headers_vec = parts
            .headers
            .to_header_vector()
            .context("Failed to create a vector from the headers")?;

        let uri_string = parts.uri.to_string();
        let method_string = parts.method.to_string();
        let version_string = parts.version.to_string().unwrap();

        let encoding_header = parts.headers.get(CONTENT_ENCODING);
        let used_compression_encoding = CompressionEncoding::from(encoding_header)
            .context("Failed to determine the encoding type")?;
        let decoded_body_raw = decode_body(body, used_compression_encoding)
            .await
            .context("Failed to decode a body")?;
        let decoded_body = BodyForFrontend::new(decoded_body_raw);
        // let decoded_body = bytes_to_string(&decoded_body_raw).await;

        Ok(Self {
            id,
            headers: headers_vec,
            uri: uri_string,
            method: method_string,
            version: version_string,
            body: decoded_body,
        })
    }

    pub async fn to_hyper(self) -> Result<Request<Body>> {
        let headers = HeaderMap::from_header_vector(self.headers).unwrap();
        let uri = self.uri.parse::<Uri>().unwrap();
        let method = Method::from_str(&self.method).unwrap();
        let version = Version::from_str(&self.version).unwrap();

        let encoding_header = headers.get(CONTENT_ENCODING);
        let used_compression_encoding = CompressionEncoding::from(encoding_header)
            .context("Failed to determine the encoding type")?;
        let encoded_body = self
            .body
            .encode_body(used_compression_encoding)
            .context("Failed to encode a body")?;

        let mut request_builder = hyper::Request::builder();
        for (name, value) in headers {
            request_builder = request_builder.header(name.unwrap(), value);
        }
        let request = request_builder
            .uri(uri)
            .method(method)
            .version(version)
            .body(encoded_body)
            .unwrap();

        Ok(request)
    }
}

pub async fn clone_request(request: Request<Body>) -> Result<(Request<Body>, Request<Body>)> {
    let (parts, body) = request.into_parts();

    let headers = parts.headers;
    let uri = parts.uri;
    let method = parts.method;
    let version = parts.version;

    let (body1, body2) = clone_body(body).await.context("Failed to clone a body")?;

    let mut request_builder1 = Request::builder()
        .uri(&uri)
        .method(&method)
        .version(version.clone());
    let mut request_builder2 = Request::builder()
        .uri(&uri)
        .method(&method)
        .version(version);

    for (key, value) in &headers {
        request_builder1 = request_builder1.header(key, value);
    }

    for (key, value) in &headers {
        request_builder2 = request_builder2.header(key, value);
    }

    let request1 = request_builder1
        .body(body1)
        .context("Failed to build a request")?;
    let request2 = request_builder2
        .body(body2)
        .context("Failed to build a request")?;

    Ok((request1, request2))
}
