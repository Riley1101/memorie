use kalosm::language::{Parse, Schema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::memory::SearchResult;

#[derive(Serialize, Deserialize)]
pub struct Response<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> Response<T> {
    /// Creates a successful response with the provided data.
    /// Arguments:
    /// * `data` - The data to be included in the successful response.
    pub fn success(data: T) -> Self {
        Response {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    /// Creates an error response with the provided error message.
    /// Arguments:
    /// * `error_msg` - The error message to be included in the error response.
    pub fn error(error_msg: String) -> Self {
        Response {
            success: false,
            data: None,
            error: Some(error_msg),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub job_id: Uuid,
}

#[derive(Deserialize, Serialize, Clone, Debug, Parse, Schema)]
pub struct AutoCompleteResponse {
    suggestion: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Parse, Schema)]
pub struct GrammarCheckResponse {
    pub corrections: String,
    pub explanation: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Parse, Schema)]
pub struct ModelLoadingResponse {
    pub is_loaded: bool,
    pub message: String,
}
