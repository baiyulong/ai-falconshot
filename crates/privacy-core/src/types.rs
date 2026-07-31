use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensitiveType {
    Phone,
    Email,
    IdCard,
    BankCard,
    IpAddress,
    Username,
    Hostname,
    Token,
    ApiKey,
    QrCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveMatch {
    pub sensitive_type: SensitiveType,
    pub start: usize,
    pub end: usize,
    pub matched_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyMode {
    Normal,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub mode: PrivacyMode,
    pub allow_cloud_upload: bool,
    pub save_history: bool,
    pub encrypt_history: bool,
    pub auto_redact_before_share: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            mode: PrivacyMode::Normal,
            allow_cloud_upload: false,
            save_history: true,
            encrypt_history: false,
            auto_redact_before_share: false,
        }
    }
}
