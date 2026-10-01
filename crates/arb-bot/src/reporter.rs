use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{create_dir_all, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ActionReporter {
    json_file: Arc<Mutex<tokio::fs::File>>,
    json_path: PathBuf,
    text_file: Arc<Mutex<tokio::fs::File>>,
    text_path: PathBuf,
}

#[derive(Serialize)]
struct ReportEntry<'a, T>
where
    T: Serialize,
{
    timestamp: String,
    action: &'a str,
    details: T,
}

impl ActionReporter {
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let json_path = path.as_ref().to_path_buf();
        let text_path = json_path.with_extension("log");

        if let Some(parent) = json_path.parent() {
            create_dir_all(parent).await?;
        }

        let json_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&json_path)
            .await?;

        let text_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&text_path)
            .await?;

        Ok(Self {
            json_file: Arc::new(Mutex::new(json_file)),
            json_path,
            text_file: Arc::new(Mutex::new(text_file)),
            text_path,
        })
    }

    pub async fn record<T>(&self, action: &str, details: T) -> Result<()>
    where
        T: Serialize,
    {
        let details_value = serde_json::to_value(&details)?;
        let entry = ReportEntry {
            timestamp: Utc::now().to_rfc3339(),
            action,
            details,
        };

        let line = serde_json::to_string(&entry)?;
        let human_line = self.format_human_line(&entry.timestamp, action, &details_value);

        let mut json_file = self.json_file.lock().await;
        json_file.write_all(line.as_bytes()).await?;
        json_file.write_all(b"\n").await?;
        json_file.flush().await?;

        let mut text_file = self.text_file.lock().await;
        text_file.write_all(human_line.as_bytes()).await?;
        text_file.write_all(b"\n").await?;
        text_file.flush().await?;

        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.json_path
    }

    pub fn text_path(&self) -> &Path {
        &self.text_path
    }

    fn format_human_line(&self, timestamp: &str, action: &str, details: &Value) -> String {
        let summary = details
            .get("summary")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| self.default_summary(action, details));

        format!("[{}] {} | {}", timestamp, action, summary)
    }

    fn default_summary(&self, action: &str, details: &Value) -> String {
        let mut fields = Vec::new();

        for key in [
            "network",
            "primary_url",
            "pool_count",
            "profit_bps",
            "expected_profit",
            "elapsed_ms",
            "signature",
            "error",
        ] {
            if let Some(value) = details.get(key) {
                fields.push(format!("{}={}", key, Self::value_to_text(value)));
            }
        }

        if fields.is_empty() {
            format!("event={}", action)
        } else {
            fields.join(" | ")
        }
    }

    fn value_to_text(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(v) => v.to_string(),
            Value::Number(v) => v.to_string(),
            Value::String(v) => v.clone(),
            _ => value.to_string(),
        }
    }
}
