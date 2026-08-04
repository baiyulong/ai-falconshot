pub mod mock_provider;
pub mod provider;
pub mod types;
pub mod windows_provider;

pub use mock_provider::MockOcrProvider;
pub use provider::OcrProvider;
pub use types::*;
pub use windows_provider::WindowsOcrProvider;
