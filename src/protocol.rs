use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
#[serde(tag = "event")]
pub enum Request {
    #[serde(rename = "init")]
    Init,
    #[serde(rename = "terminate")]
    Terminate,
    #[serde(rename = "download")]
    Download { oid: String },
    #[serde(rename = "upload")]
    Upload { oid: String, path: String },
}

impl TryFrom<&str> for Request {
    type Error = serde_json::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(value)
    }
}

#[derive(Serialize, Debug)]
pub struct ProtocolError {
    code: i8,
    message: String,
}

impl ProtocolError {
    pub fn new(code: i8, message: String) -> Self {
        Self { code, message }
    }
}

#[derive(Serialize, Debug)]
pub struct InitResponse {}

impl InitResponse {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Serialize, Debug)]
pub struct ProgressResponse {
    event: String,
    oid: String,
    #[serde(rename = "bytesSoFar")]
    bytes_so_far: usize,
    #[serde(rename = "bytesSinceLast")]
    bytes_since_last: usize,
}

impl ProgressResponse {
    pub fn new(oid: String, bytes_so_far: usize, bytes_since_last: usize) -> Self {
        Self {
            event: String::from("progress"),
            oid,
            bytes_so_far,
            bytes_since_last,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum TransferResponse {
    Ok {
        event: String,
        oid: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    Err {
        event: String,
        oid: String,
        error: ProtocolError,
    },
}

impl TransferResponse {
    pub fn new<E: std::fmt::Display>(
        oid: String,
        result: std::result::Result<Option<String>, E>,
    ) -> Self {
        match result {
            Ok(path) => Self::Ok {
                event: String::from("complete"),
                oid,
                path,
            },
            Err(e) => Self::Err {
                event: String::from("complete"),
                oid,
                error: ProtocolError::new(1, format!("{}", e)),
            },
        }
    }
}

pub trait Response: Serialize {
    fn json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl Response for InitResponse {}
impl Response for ProgressResponse {}
impl Response for TransferResponse {}
