//! An owned anonymous Chromium session. No user profiles or cookies are read.
use crate::{Error, Result, http::Http};
use serde_json::{Value, json};
use std::{
    net::TcpStream,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};

pub struct Browser {
    child: Child,
    profile: tempfile::TempDir,
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    sequence: u64,
    timeout: Duration,
}
impl Browser {
    pub fn new(path: Option<PathBuf>, timeout: Duration, proxy: Option<&str>) -> Result<Self> {
        let path = path.or_else(find_browser).ok_or_else(|| {
            Error::new(
                "browser_missing",
                "未找到 Chrome/Edge，请指定 --browser-path 或 LCSC_BROWSER_PATH",
            )
        })?;
        let profile = tempfile::tempdir()?;
        let mut command = Command::new(path);
        if let Some(proxy) = proxy {
            let parsed = url::Url::parse(proxy).map_err(|e| Error::invalid(e.to_string()))?;
            if !parsed.username().is_empty() || parsed.password().is_some() {
                return Err(Error::new(
                    "browser_proxy_unsupported",
                    "浏览器回退不支持带账号密码的代理，请使用无需认证的本地代理",
                ));
            }
            command.arg(format!("--proxy-server={proxy}"));
        } else {
            command.arg("--no-proxy-server");
        }
        command
            .args([
                "--headless=new",
                "--no-first-run",
                "--no-default-browser-check",
                "--remote-debugging-port=0",
                "--remote-debugging-address=127.0.0.1",
            ])
            .arg(format!("--user-data-dir={}", profile.path().display()))
            .arg("about:blank")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let child = command.spawn()?;
        let mut browser = Self {
            child,
            profile,
            socket: None,
            sequence: 0,
            timeout,
        };
        let deadline = Instant::now() + timeout;
        let port = loop {
            if let Ok(text) =
                std::fs::read_to_string(browser.profile.path().join("DevToolsActivePort"))
                && let Some(port) = text.lines().next().and_then(|s| s.parse::<u16>().ok())
            {
                break port;
            }
            if browser.child.try_wait()?.is_some() || Instant::now() >= deadline {
                return Err(Error::new("browser_failed", "浏览器未能启动调试会话"));
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let http = Http::new(timeout, None)?;
        let targets = http.json(&format!("http://127.0.0.1:{port}/json/list"), None)?;
        let ws = targets
            .as_array()
            .and_then(|a| a.iter().find(|t| t["type"] == "page"))
            .and_then(|t| t["webSocketDebuggerUrl"].as_str())
            .ok_or_else(|| Error::new("browser_failed", "浏览器未提供页面调试接口"))?;
        let (mut socket, _) = tungstenite::connect(ws).map_err(browser_error)?;
        if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream.set_read_timeout(Some(timeout))?;
            stream.set_write_timeout(Some(timeout))?;
        }
        browser.socket = Some(socket);
        browser.call("Page.enable", json!({}))?;
        Ok(browser)
    }
    fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        self.sequence += 1;
        let id = self.sequence;
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| Error::new("browser_failed", "浏览器连接已关闭"))?;
        socket
            .send(Message::Text(
                json!({"id":id,"method":method,"params":params})
                    .to_string()
                    .into(),
            ))
            .map_err(browser_error)?;
        let deadline = Instant::now() + self.timeout;
        while Instant::now() < deadline {
            let message = socket.read().map_err(browser_error)?;
            if let Message::Text(text) = message {
                let value: Value = serde_json::from_str(&text)?;
                if value["id"] == id {
                    if !value["error"].is_null() {
                        return Err(Error::new("browser_failed", value["error"].to_string()));
                    }
                    return Ok(value["result"].clone());
                }
            }
        }
        Err(Error::new("browser_timeout", "浏览器操作超时"))
    }
    pub fn evaluate(&mut self, expression: &str) -> Result<Value> {
        let result = self.call("Runtime.evaluate", json!({"expression":expression,"awaitPromise":true,"returnByValue":true,"timeout":self.timeout.as_millis() as u64}))?;
        if !result["exceptionDetails"].is_null() {
            return Err(Error::new(
                "browser_failed",
                result["exceptionDetails"].to_string(),
            ));
        }
        Ok(result["result"]["value"].clone())
    }
    pub fn page(&mut self, url: &str) -> Result<Value> {
        let navigated = self.call("Page.navigate", json!({"url":url}))?;
        if let Some(error) = navigated["errorText"].as_str() {
            return Err(Error::new("browser_failed", error));
        }
        let deadline = Instant::now() + self.timeout;
        loop {
            let expression = format!(
                "(()=>{{if(location.href!=={})return null;const e=document.getElementById('__NEXT_DATA__');return e?JSON.parse(e.textContent).props.pageProps:null}})()",
                serde_json::to_string(url)?
            );
            let result = self.evaluate(&expression)?;
            if !result.is_null() {
                return Ok(result);
            }
            if Instant::now() >= deadline {
                return Err(Error::new(
                    "page_challenge",
                    "页面未提供商品数据，请在浏览器中检查商品链接及验证码",
                ));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    pub fn api(&mut self, path: &str, body: &Value) -> Result<Value> {
        let origin = self.evaluate("location.origin")?;
        if origin != "https://so.szlcsc.com" {
            self.page("https://so.szlcsc.com/global.html?k=LM358")?;
        }
        let expression = format!(
            "(async()=>{{const r=await fetch({},{{method:'POST',headers:{{'content-type':'application/json'}},body:JSON.stringify({}),signal:AbortSignal.timeout({})}});return {{status:r.status,body:await r.text()}}}})()",
            serde_json::to_string(path)?,
            body,
            self.timeout.as_millis()
        );
        let response = self.evaluate(&expression)?;
        if response["status"] != 200 {
            return Err(Error::new(
                "http_error",
                format!("浏览器查询 HTTP {}", response["status"]),
            ));
        }
        serde_json::from_str(response["body"].as_str().unwrap_or(""))
            .map_err(|_| Error::new("page_challenge", "浏览器查询返回非 JSON 页面"))
    }
}
impl Drop for Browser {
    fn drop(&mut self) {
        if let Some(socket) = self.socket.as_mut() {
            let _ = socket.send(Message::Text(
                "{\"id\":999999,\"method\":\"Browser.close\"}".into(),
            ));
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn browser_error(e: tungstenite::Error) -> Error {
    Error::new("browser_failed", e.to_string())
}
fn find_browser() -> Option<PathBuf> {
    let mut paths = vec![];
    if let Some(path) = std::env::var_os("LCSC_BROWSER_PATH") {
        paths.push(PathBuf::from(path));
    }
    for variable in ["PROGRAMFILES(X86)", "PROGRAMFILES", "LOCALAPPDATA"] {
        if let Some(root) = std::env::var_os(variable) {
            paths.push(PathBuf::from(&root).join("Microsoft/Edge/Application/msedge.exe"));
            paths.push(PathBuf::from(root).join("Google/Chrome/Application/chrome.exe"));
        }
    }
    paths.extend(
        [
            "/usr/bin/google-chrome",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ]
        .map(PathBuf::from),
    );
    paths.into_iter().find(|p| p.is_file())
}
