use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub category: PromptCategory,
    pub system_prompt: String,
    pub user_prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptCategory {
    Summarize,
    Translate,
    ErrorAnalysis,
    ExtractTable,
    ExtractTodo,
    CodeAnalysis,
    UiReview,
    Custom,
}

impl PromptTemplate {
    pub fn builtin_templates() -> Vec<Self> {
        vec![
            Self {
                id: "summarize".to_string(),
                name: "总结截图内容".to_string(),
                category: PromptCategory::Summarize,
                system_prompt: "你是一个图片内容分析助手。请简洁准确地总结图片中的主要内容。".to_string(),
                user_prompt: "请总结这张截图的内容。".to_string(),
            },
            Self {
                id: "translate".to_string(),
                name: "翻译截图内容".to_string(),
                category: PromptCategory::Translate,
                system_prompt: "你是一个翻译助手。请将图片中的文字翻译为中文，保持原文格式。".to_string(),
                user_prompt: "请翻译这张截图中的文字内容。".to_string(),
            },
            Self {
                id: "error_analysis".to_string(),
                name: "分析报错信息".to_string(),
                category: PromptCategory::ErrorAnalysis,
                system_prompt: "你是一个资深软件工程师。请分析截图中的错误信息，给出错误原因、排查步骤和修复建议。".to_string(),
                user_prompt: "请分析这张截图中的报错信息，给出原因分析和排查步骤。".to_string(),
            },
            Self {
                id: "extract_table".to_string(),
                name: "提取表格".to_string(),
                category: PromptCategory::ExtractTable,
                system_prompt: "你是一个数据提取助手。请从图片中提取表格数据，以 Markdown 表格格式输出。".to_string(),
                user_prompt: "请提取这张截图中的表格数据。".to_string(),
            },
        ]
    }
}
