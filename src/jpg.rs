//! HTML のページを 1 枚ずつ JPG にする。docv/service/service.md の「JPG」、docv/system/system.md の「JPG の撮り方」。
//!
//! ローカルの Chrome をヘッドレスで起動し、DevTools Protocol で操作して撮る。

use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use serde_json::{Value, json};
use tungstenite::{Message, WebSocket};

/// 撮るときの幅（CSS ピクセル）と、JPG の画質。
const WIDTH: u32 = 1280;
const QUALITY: u32 = 80;
const TIMEOUT: Duration = Duration::from_secs(30);

/// shots の（HTML, JPG）ごとに、src の中の HTML のページ全体を JPG にし、out の中に書く。
/// パスは、HTML は src から、JPG は out からの相対パス。
pub fn capture(src: &Path, out: &Path, shots: &[(String, String)]) -> Result<()> {
    let chrome = find_chrome()
        .ok_or_else(|| anyhow!("Chrome が見つからない。環境変数 DOCV_CHROME で場所を指定する"))?;
    let profile = env::temp_dir().join(format!("docv-chrome-{}", std::process::id()));
    let mut browser = Browser::launch(&chrome, &profile)?;
    let result = (|| {
        let session = browser.open_page()?;
        for (html, path) in shots {
            let jpg = out.join(path);
            browser.load(&session, &file_url(&src.join(html)))?;
            let data = browser.screenshot(&session)?;
            if let Some(parent) = jpg.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&jpg, data).with_context(|| format!("{} を書けない", jpg.display()))?;
        }
        Ok(())
    })();
    browser.close();
    let _ = fs::remove_dir_all(&profile);
    result
}

/// Chrome を、環境変数 DOCV_CHROME、OS ごとの標準のインストール先、PATH の順に探す。
fn find_chrome() -> Option<PathBuf> {
    if let Some(path) = env::var_os("DOCV_CHROME") {
        return Some(PathBuf::from(path));
    }
    let standard: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
    } else if cfg!(windows) {
        &[
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ]
    } else {
        &["/usr/bin/google-chrome", "/usr/bin/chromium"]
    };
    if let Some(path) = standard.iter().map(PathBuf::from).find(|p| p.is_file()) {
        return Some(path);
    }
    let names = ["chrome", "chromium", "google-chrome", "chromium-browser"];
    env::split_paths(&env::var_os("PATH")?)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|p| p.is_file())
}

/// ファイルのパスを file:// の URL にする。`/` と英数字と一部の記号以外はパーセントエンコードする。
fn file_url(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    let mut url = String::from("file://");
    if !path.starts_with('/') {
        url.push('/');
    }
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' | b':' => {
                url.push(b as char)
            }
            _ => url.push_str(&format!("%{b:02X}")),
        }
    }
    url
}

struct Browser {
    child: Child,
    socket: WebSocket<TcpStream>,
    next_id: u64,
}

impl Browser {
    fn launch(chrome: &Path, profile: &Path) -> Result<Browser> {
        let mut child = Command::new(chrome)
            .args([
                "--headless=new",
                "--disable-gpu",
                "--hide-scrollbars",
                "--no-first-run",
                "--no-default-browser-check",
                "--remote-debugging-port=0",
            ])
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Chrome（{}）を起動できない", chrome.display()))?;

        // Chrome は、DevTools の接続先を標準エラーに出す。
        let stderr = child.stderr.take().expect("標準エラーをパイプにした");
        let mut url = None;
        for line in BufReader::new(stderr).lines() {
            let line = line?;
            if let Some(rest) = line.strip_prefix("DevTools listening on ") {
                url = Some(rest.trim().to_string());
                break;
            }
        }
        let Some(url) = url else {
            let _ = child.kill();
            bail!("Chrome の DevTools に接続できない");
        };

        let address = url
            .strip_prefix("ws://")
            .and_then(|rest| rest.split('/').next())
            .ok_or_else(|| anyhow!("DevTools の接続先 {url} が読めない"))?;
        let stream = TcpStream::connect(address)?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        let (socket, _) = tungstenite::client(url.as_str(), stream)
            .map_err(|e| anyhow!("DevTools に接続できない: {e}"))?;
        Ok(Browser {
            child,
            socket,
            next_id: 0,
        })
    }

    /// 新しいタブを開き、操作するためのセッションを返す。
    fn open_page(&mut self) -> Result<String> {
        let target = self.call(None, "Target.createTarget", json!({ "url": "about:blank" }))?;
        let target_id = target["targetId"].as_str().unwrap_or_default().to_string();
        let attached = self.call(
            None,
            "Target.attachToTarget",
            json!({ "targetId": target_id, "flatten": true }),
        )?;
        let session = attached["sessionId"]
            .as_str()
            .ok_or_else(|| anyhow!("タブに接続できない"))?
            .to_string();
        self.call(Some(&session), "Page.enable", json!({}))?;
        Ok(session)
    }

    /// ページを開き、ページの高さに画面を合わせる。固定した位置の要素も含め、全体が一度に見える。
    fn load(&mut self, session: &str, url: &str) -> Result<()> {
        let s = Some(session);
        self.set_size(session, 800)?;
        // 印刷の表示にする。OS ごとの手順のタブを、すべて並べて写すため。
        self.call(s, "Emulation.setEmulatedMedia", json!({ "media": "print" }))?;
        self.call(s, "Page.navigate", json!({ "url": url }))?;
        self.wait_event(session, "Page.loadEventFired")?;
        let metrics = self.call(s, "Page.getLayoutMetrics", json!({}))?;
        let height = metrics["cssContentSize"]["height"]
            .as_f64()
            .unwrap_or(800.0)
            .ceil() as u32;
        self.set_size(session, height)
    }

    /// 開いたページの全体を JPG にする。
    fn screenshot(&mut self, session: &str) -> Result<Vec<u8>> {
        let shot = self.call(
            Some(session),
            "Page.captureScreenshot",
            json!({ "format": "jpeg", "quality": QUALITY }),
        )?;
        let data = shot["data"]
            .as_str()
            .ok_or_else(|| anyhow!("ページを撮れない"))?;
        Ok(base64::engine::general_purpose::STANDARD.decode(data)?)
    }

    fn set_size(&mut self, session: &str, height: u32) -> Result<()> {
        self.call(
            Some(session),
            "Emulation.setDeviceMetricsOverride",
            json!({ "width": WIDTH, "height": height.max(1), "deviceScaleFactor": 1, "mobile": false }),
        )?;
        Ok(())
    }

    /// 命令を送り、その応答の result を返す。
    fn call(&mut self, session: Option<&str>, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            message["sessionId"] = json!(session);
        }
        self.socket
            .send(Message::text(message.to_string()))
            .map_err(|e| anyhow!("DevTools に送れない: {e}"))?;
        loop {
            let reply = self.read()?;
            if reply["id"].as_u64() == Some(id) {
                if let Some(error) = reply.get("error") {
                    bail!("{method} が失敗した: {error}");
                }
                return Ok(reply["result"].clone());
            }
        }
    }

    fn wait_event(&mut self, session: &str, method: &str) -> Result<()> {
        loop {
            let message = self.read()?;
            if message["method"] == method && message["sessionId"] == session {
                return Ok(());
            }
        }
    }

    fn read(&mut self) -> Result<Value> {
        loop {
            let message = self
                .socket
                .read()
                .map_err(|e| anyhow!("DevTools から読めない: {e}"))?;
            if let Message::Text(text) = message {
                return Ok(serde_json::from_str(text.as_str())?);
            }
        }
    }

    fn close(&mut self) {
        let _ = self.call(None, "Browser.close", json!({}));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_file_urls() {
        assert_eq!(
            file_url(Path::new("/a b/ドキュメント.html")),
            "file:///a%20b/%E3%83%89%E3%82%AD%E3%83%A5%E3%83%A1%E3%83%B3%E3%83%88.html"
        );
    }
}
