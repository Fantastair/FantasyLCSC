use crate::{Error, Result, http::Http};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub const SOURCE: &str = "嘉立创 EDA / EasyEDA 官方库";

pub fn normalize_parts(values: &[String]) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut parts = vec![];
    for token in values
        .iter()
        .flat_map(|s| s.split(|c: char| c.is_whitespace() || ",;，；".contains(c)))
        .filter(|s| !s.is_empty())
    {
        let part = token.to_ascii_uppercase();
        let valid = part
            .strip_prefix('C')
            .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
        if !valid {
            return Err(Error::invalid(format!("编号无效：{token}，示例 C492411")));
        }
        if seen.insert(part.clone()) {
            parts.push(part);
        }
    }
    if parts.is_empty() {
        return Err(Error::invalid("请提供至少一个 C 编号"));
    }
    Ok(parts)
}
fn object(value: &Value) -> Result<Value> {
    if let Some(s) = value.as_str() {
        let parsed: Value = serde_json::from_str(s)?;
        if parsed.is_object() {
            return Ok(parsed);
        }
    }
    if value.is_object() {
        return Ok(value.clone());
    }
    Err(Error::new(
        "schema_changed",
        "EasyEDA 元件字段不是 JSON 对象",
    ))
}
pub fn safe_title(input: &str) -> String {
    static UNSAFE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9_.-]+").unwrap());
    let title = UNSAFE
        .replace_all(input, "_")
        .trim_matches(['.', '_'])
        .chars()
        .take(100)
        .collect::<String>();
    if title.is_empty() {
        "model".into()
    } else {
        title
    }
}
pub fn component_info(part: &str, raw: &Value) -> Result<Value> {
    if raw["success"] != true {
        return Err(Error::new("not_found", format!("EasyEDA 无法解析 {part}")));
    }
    let result = object(&raw["result"])?;
    let package = if result["packageDetail"].is_null() {
        json!({})
    } else {
        object(&result["packageDetail"])?
    };
    let data = if package["dataStr"].is_null() {
        json!({})
    } else {
        object(&package["dataStr"])?
    };
    let symbol = if result["dataStr"].is_null() {
        json!({})
    } else {
        object(&result["dataStr"])?
    };
    let params = &symbol["head"]["c_para"];
    if let Some(code) = params["Supplier Part"].as_str().filter(|s| !s.is_empty())
        && !code.eq_ignore_ascii_case(part)
    {
        return Err(Error::new(
            "identity_mismatch",
            "EasyEDA 返回的 C 编号与查询不一致",
        ));
    }
    if !data["shape"].is_null() && !data["shape"].is_array() {
        return Err(Error::new("schema_changed", "EasyEDA 封装 shape 不是数组"));
    }
    static UUID: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:[0-9a-fA-F]{32}|[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12})$")
            .unwrap()
    });
    let mut uuids = vec![];
    for shape in data["shape"].as_array().into_iter().flatten() {
        if let Some(text) = shape.as_str().and_then(|s| s.strip_prefix("SVGNODE~")) {
            let Ok(node) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            let Ok(attrs) = object(&node["attrs"]) else {
                continue;
            };
            if attrs["c_etype"] == "outline3D"
                && let Some(uuid) = attrs["uuid"].as_str().filter(|s| UUID.is_match(s))
                && !uuids.iter().any(|s: &String| s.eq_ignore_ascii_case(uuid))
            {
                uuids.push(uuid.to_owned());
            }
        }
    }
    Ok(
        json!({"source":SOURCE,"source_url":"https://easyeda.com/","fetched_at":crate::fetched_at(),"code":part,"title":result.get("title").filter(|v| !v.is_null()).unwrap_or(&package["title"]),"component_uuid":result["uuid"],"package_uuid":package["uuid"],"package_title":package["title"],"model":params["Manufacturer Part"],"manufacturer":params["Manufacturer"],"package":params["package"],"assembly_class":params["JLCPCB Part Class"],"model_uuids":uuids,"lcsc":result["lcsc"],"currency":null,"description":result["description"],"thumb":result["thumb"],"raw":raw}),
    )
}
pub fn validate_step(data: &[u8]) -> Result<()> {
    static ENTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#\d+\s*=").unwrap());
    let text = String::from_utf8_lossy(data);
    let text = text.trim();
    if !text.starts_with("ISO-10303-21;")
        || !text.ends_with("END-ISO-10303-21;")
        || !text.contains("HEADER;")
        || !text.contains("DATA;")
        || !ENTITY.is_match(text)
    {
        return Err(Error::new(
            "invalid_step",
            "响应不是完整 STEP 文件；未保存。已有文件损坏时请使用 --overwrite",
        ));
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadFile {
    pub path: PathBuf,
    pub uuid: String,
    pub bytes: u64,
    pub skipped: bool,
}
#[derive(Clone, Debug)]
pub enum DownloadEvent {
    Metadata(Value),
    Progress {
        index: usize,
        count: usize,
        received: u64,
        total: Option<u64>,
    },
    Saved(DownloadFile),
}
pub struct ModelClient {
    http: Http,
    api_origin: String,
    step_origin: String,
}
impl ModelClient {
    pub fn new(http: Http) -> Self {
        Self::with_origins(
            http,
            "https://easyeda.com/api/products".into(),
            "https://modules.easyeda.com/qAxj6KHrDKw4blvCG8QJPs7Y".into(),
        )
    }
    /// Separate origins allow offline protocol tests against a loopback server.
    pub fn with_origins(http: Http, api_origin: String, step_origin: String) -> Self {
        Self {
            http,
            api_origin,
            step_origin,
        }
    }
    pub fn info(&self, part: &str) -> Result<Value> {
        let parts = normalize_parts(&[part.into()])?;
        if parts.len() != 1 {
            return Err(Error::invalid("info 需要一个编号"));
        }
        let raw = self.http.json(
            &format!("{}/{}/components", self.api_origin, parts[0]),
            None,
        )?;
        component_info(&parts[0], &raw)
    }
    pub fn download(
        &self,
        part: &str,
        output: &Path,
        overwrite: bool,
        event: &mut dyn FnMut(DownloadEvent) -> bool,
    ) -> Result<Vec<DownloadFile>> {
        let parts = normalize_parts(&[part.into()])?;
        if parts.len() != 1 {
            return Err(Error::invalid("download 需要一个编号"));
        }
        let part = &parts[0];
        // Metadata is cancellable, including during retry backoff.
        let raw = self.http.request(
            &format!("{}/{part}/components", self.api_origin),
            None,
            &mut |received, total| {
                event(DownloadEvent::Progress {
                    index: 0,
                    count: 0,
                    received,
                    total,
                })
            },
        )?;
        let raw: Value =
            serde_json::from_slice(raw.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&raw))?;
        let info = component_info(part, &raw)?;
        if !event(DownloadEvent::Metadata(info.clone())) {
            return Err(Error::new("cancelled", "已取消下载"));
        }
        let uuids = info["model_uuids"].as_array().unwrap();
        if uuids.is_empty() {
            return Err(Error::new("no_model", format!("{part} 未关联 3D 模型")));
        }
        let title = safe_title(info["title"].as_str().unwrap_or("model"));
        let mut files = vec![];
        for (i, uuid) in uuids.iter().enumerate() {
            let uuid = uuid.as_str().unwrap();
            let suffix = if uuids.len() > 1 {
                format!("_{}", i + 1)
            } else {
                String::new()
            };
            let target = output.join(format!("{part}_{title}{suffix}.step"));
            let (bytes, skipped) = if target.exists() && !overwrite {
                let bytes = std::fs::read(&target)?;
                validate_step(&bytes)?;
                (bytes.len() as u64, true)
            } else {
                let bytes = self.http.request(
                    &format!("{}/{uuid}", self.step_origin),
                    None,
                    &mut |received, total| {
                        event(DownloadEvent::Progress {
                            index: i + 1,
                            count: uuids.len(),
                            received,
                            total,
                        })
                    },
                )?;
                validate_step(&bytes)?;
                if !event(DownloadEvent::Progress {
                    index: i + 1,
                    count: uuids.len(),
                    received: bytes.len() as u64,
                    total: Some(bytes.len() as u64),
                }) {
                    return Err(Error::new("cancelled", "已取消下载"));
                }
                crate::atomic_write(&target, &bytes, overwrite)?;
                (bytes.len() as u64, false)
            };
            let file = DownloadFile {
                path: target,
                uuid: uuid.into(),
                bytes,
                skipped,
            };
            files.push(file.clone());
            if !event(DownloadEvent::Saved(file)) {
                return Err(Error::new("cancelled", "已取消下载"));
            }
        }
        Ok(files)
    }
}
