use crate::{Error, Result};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{io::Read, time::Duration};

#[derive(Clone)]
pub struct Http {
    client: Client,
}
impl Http {
    /// Direct by default: do not inherit the Windows Internet Settings proxy.
    pub fn new(timeout: Duration, proxy: Option<&str>) -> Result<Self> {
        let mut builder = Client::builder()
            .no_proxy()
            .timeout(timeout)
            .connect_timeout(timeout.min(Duration::from_secs(10)))
            .user_agent("Mozilla/5.0 (compatible; FantasyLCSC/0.1)");
        if let Some(proxy) = proxy {
            builder = builder
                .proxy(reqwest::Proxy::all(proxy).map_err(|e| Error::invalid(e.to_string()))?);
        }
        Ok(Self {
            client: builder.build().map_err(network_error)?,
        })
    }
    pub fn get(&self, url: &str) -> Result<Vec<u8>> {
        self.request(url, None, &mut |_, _| true)
    }
    pub fn json(&self, url: &str, body: Option<&Value>) -> Result<Value> {
        let data = self.request(url, body, &mut |_, _| true)?;
        serde_json::from_slice(data.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&data)).map_err(
            |_| Error::new("page_challenge", "接口返回了非 JSON 页面，可能需要站点校验").fallback(),
        )
    }
    pub fn request(
        &self,
        url: &str,
        body: Option<&Value>,
        progress: &mut dyn FnMut(u64, Option<u64>) -> bool,
    ) -> Result<Vec<u8>> {
        for attempt in 0..3 {
            if !progress(0, None) {
                return Err(Error::new("cancelled", "已取消下载"));
            }
            let result = self.once(url, body, progress);
            match result {
                Err(ref e)
                    if attempt < 2
                        && matches!(e.code.as_str(), "network_error" | "retryable_http") =>
                {
                    // Check cancellation during retry backoff as well.
                    for _ in 0..(attempt + 1) * 10 {
                        if !progress(0, None) {
                            return Err(Error::new("cancelled", "已取消下载"));
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
                _ => return result,
            }
        }
        unreachable!()
    }
    fn once(
        &self,
        url: &str,
        body: Option<&Value>,
        progress: &mut dyn FnMut(u64, Option<u64>) -> bool,
    ) -> Result<Vec<u8>> {
        let referer = if url.contains("szlcsc.com") {
            "https://so.szlcsc.com/"
        } else {
            "https://easyeda.com/"
        };
        let request = if let Some(body) = body {
            self.client.post(url).json(body)
        } else {
            self.client.get(url)
        };
        let mut response = request
            .header("Referer", referer)
            .send()
            .map_err(network_error)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) || status == 203 {
            let code = if [429, 500, 502, 503, 504].contains(&status) {
                "retryable_http"
            } else {
                "http_error"
            };
            let e = Error::new(code, format!("上游返回 HTTP {status}：{url}"));
            return Err(if [203, 401, 403, 405].contains(&status) {
                e.fallback()
            } else {
                e
            });
        }
        let total = response.content_length();
        let mut data = Vec::new();
        let mut buffer = [0; 64 * 1024];
        loop {
            if !progress(data.len() as u64, total) {
                return Err(Error::new("cancelled", "已取消下载"));
            }
            let size = response
                .read(&mut buffer)
                .map_err(|e| Error::new("network_error", e.to_string()).fallback())?;
            if size == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..size]);
            if data.len() > 128 * 1024 * 1024 {
                return Err(Error::new(
                    "response_too_large",
                    "响应超过 128 MiB，停止下载",
                ));
            }
        }
        if !progress(data.len() as u64, total) {
            return Err(Error::new("cancelled", "已取消下载"));
        }
        Ok(data)
    }
}
fn network_error(e: reqwest::Error) -> Error {
    Error::new("network_error", format!("网络请求失败：{e}")).fallback()
}
