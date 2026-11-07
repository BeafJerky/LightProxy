use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreError {
    pub code: String,
    pub stage: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl CoreError {
    pub fn new(code: &str, stage: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            stage: stage.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", "internal", message)
    }

    pub fn profile_not_found(id: &str) -> Self {
        Self::new("profile_not_found", "validate", format!("配置不存在：{id}"))
            .with_hint("刷新配置列表后重试")
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}] {}", self.code, self.stage, self.message)
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(value: std::io::Error) -> Self {
        Self::new("io", "storage", value.to_string())
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::new("json", "storage", value.to_string())
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
