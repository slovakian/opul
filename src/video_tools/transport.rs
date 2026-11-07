use super::install::{Failure, download_error, verification_error};
use super::*;
use reqwest::{Client, Method, Response, StatusCode};
use std::future::Future;
use std::io::{Seek, SeekFrom, Write};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub type NetworkFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Reachability and time are injected independently of HTTP and the installation root.
/// Dropping either future must cancel it, including when setup is cancelled or Opul quits.
pub trait NetworkEnvironment: Send + Sync {
    fn online(&self) -> NetworkFuture<'_, bool>;
    fn sleep(&self, duration: Duration) -> NetworkFuture<'_, ()>;
}

pub(super) struct SystemNetwork;
impl NetworkEnvironment for SystemNetwork {
    fn online(&self) -> NetworkFuture<'_, bool> {
        Box::pin(async {
            #[cfg(target_os = "macos")]
            {
                // scutil queries macOS SystemConfiguration reachability. It does not fetch a URL.
                let mut command = tokio::process::Command::new("/usr/sbin/scutil");
                command.args(["-r", "github.com"]).kill_on_drop(true);
                if let Ok(Ok(output)) =
                    tokio::time::timeout(Duration::from_secs(5), command.output()).await
                    && output.status.success()
                {
                    let flags = String::from_utf8_lossy(&output.stdout);
                    return !flags.contains("Not Reachable")
                        && !flags.contains("Connection Required");
                }
            }
            // A reachability service error is not proof that the machine is offline.
            true
        })
    }

    fn sleep(&self, duration: Duration) -> NetworkFuture<'_, ()> {
        Box::pin(tokio::time::sleep(duration))
    }
}

pub(super) struct Transfer<'a> {
    pub tools: &'a VideoTools,
    pub client: &'a Client,
    pub step: Option<(Uuid, Component)>,
}

enum Sink {
    Memory(Vec<u8>),
    File(std::fs::File),
}
impl Sink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        match self {
            Self::Memory(data) => data.extend_from_slice(bytes),
            Self::File(file) => file.write_all(bytes).map_err(download_error)?,
        }
        Ok(())
    }
    fn reset(&mut self) -> Result<(), Failure> {
        match self {
            Self::Memory(data) => data.clear(),
            Self::File(file) => {
                file.set_len(0).map_err(download_error)?;
                file.seek(SeekFrom::Start(0)).map_err(download_error)?;
            }
        }
        Ok(())
    }
}

impl Transfer<'_> {
    fn progress(&self, status: StepStatus, downloaded: u64, total: Option<u64>) {
        if let Some((id, component)) = self.step {
            self.tools
                .progress(id, component, status, downloaded, total);
        }
    }

    async fn wait_online(&self) -> Result<(), Failure> {
        let environment = &self.tools.sources.environment;
        if environment.online().await {
            return Ok(());
        }
        // The optional size lookup has no setup lifetime or Cancel button. Leave its fallback.
        if self.step.is_none() {
            return Err(download_error(
                "Published size is unavailable while offline.",
            ));
        }
        let previous = self.step.and_then(|(_, component)| {
            if let VideoToolsState::SettingUp { steps } = self.tools.snapshot().state {
                steps.into_iter().find(|step| step.component == component)
            } else {
                None
            }
        });
        loop {
            if let Some(step) = &previous {
                self.progress(
                    StepStatus::WaitingForInternet,
                    step.downloaded_bytes,
                    step.total_bytes,
                );
            }
            environment.sleep(Duration::from_secs(1)).await;
            if environment.online().await {
                break;
            }
        }
        if let Some(step) = previous {
            self.progress(step.status, step.downloaded_bytes, step.total_bytes);
        }
        Ok(())
    }

    async fn retry(&self, attempts: &mut usize, error: Failure) -> Result<(), Failure> {
        let environment = &self.tools.sources.environment;
        if !environment.online().await {
            self.wait_online().await?;
            return Ok(());
        }
        let Some(delay) = [2, 10, 30].get(*attempts) else {
            return Err(error);
        };
        environment.sleep(Duration::from_secs(*delay)).await;
        // If the connection disappeared during backoff, wait without spending an attempt.
        if !environment.online().await {
            self.wait_online().await?;
        } else {
            *attempts += 1;
        }
        Ok(())
    }

    async fn request(
        &self,
        method: Method,
        url: &str,
        offset: u64,
    ) -> Result<Response, (Failure, bool)> {
        let mut request = self
            .client
            .request(method, url)
            .header("Accept-Encoding", "identity");
        if offset > 0 {
            request = request.header("Range", format!("bytes={offset}-"));
        }
        let response = request.send().await.map_err(|error| {
            let transient =
                error.is_connect() || error.is_timeout() || error.is_body() || error.is_request();
            (download_error(error), transient)
        })?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let transient = status.is_server_error() || matches!(status.as_u16(), 408 | 429);
        Err((download_error(format!("{url}: HTTP {status}")), transient))
    }

    pub async fn response(&self, method: Method, url: &str) -> Result<Response, Failure> {
        let mut attempts = 0;
        loop {
            self.wait_online().await?;
            match self.request(method.clone(), url, 0).await {
                Ok(response) => return Ok(response),
                Err((error, true)) => self.retry(&mut attempts, error).await?,
                Err((error, false)) => return Err(error),
            }
        }
    }

    pub async fn bytes(&self, url: &str, limit: u64) -> Result<Vec<u8>, Failure> {
        let mut sink = Sink::Memory(Vec::new());
        self.receive(url, limit, None, &mut sink).await?;
        let Sink::Memory(bytes) = sink else {
            unreachable!()
        };
        Ok(bytes)
    }

    pub async fn download(
        &self,
        url: &str,
        path: &std::path::Path,
        expected: Option<u64>,
        limit: u64,
    ) -> Result<(), Failure> {
        let mut sink = Sink::File(std::fs::File::create(path).map_err(download_error)?);
        self.receive(url, limit, expected, &mut sink).await?;
        if let Sink::File(file) = sink {
            file.sync_all().map_err(download_error)?;
        }
        Ok(())
    }

    async fn receive(
        &self,
        url: &str,
        limit: u64,
        expected: Option<u64>,
        sink: &mut Sink,
    ) -> Result<(), Failure> {
        let report_progress = matches!(sink, Sink::File(_));
        let mut downloaded = 0_u64;
        let mut total = expected;
        let mut attempts = 0;
        loop {
            self.wait_online().await?;
            let mut response = match self.request(Method::GET, url, downloaded).await {
                Ok(response) => response,
                Err((error, true)) => {
                    self.retry(&mut attempts, error).await?;
                    continue;
                }
                Err((error, false)) => return Err(error),
            };
            let response_end = if response.status() == StatusCode::PARTIAL_CONTENT {
                let range = response
                    .headers()
                    .get("Content-Range")
                    .and_then(|value| value.to_str().ok())
                    .and_then(parse_range)
                    .filter(|(start, end, size)| {
                        *start == downloaded
                            && total.is_none_or(|total| total == *size)
                            && response
                                .content_length()
                                .is_none_or(|length| length == end - start + 1)
                    })
                    .ok_or_else(|| {
                        download_error("The server returned an invalid resume range.")
                    })?;
                total = Some(range.2);
                Some(range.1 + 1)
            } else {
                // Servers may ignore Range. Replace the prefix instead of appending a full file.
                sink.reset()?;
                downloaded = 0;
                total = response.content_length();
                None
            };
            if report_progress {
                self.progress(StepStatus::Downloading, downloaded, total);
            }
            let failure = loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        downloaded += chunk.len() as u64;
                        if downloaded > limit {
                            return Err(verification_error(
                                "Video-tools response exceeds its size limit.",
                            ));
                        }
                        if response_end.is_some_and(|end| downloaded > end) {
                            return Err(download_error(
                                "The resumed body exceeded its advertised range.",
                            ));
                        }
                        sink.write(&chunk)?;
                        if report_progress {
                            self.progress(StepStatus::Downloading, downloaded, total);
                        }
                    }
                    Ok(None) => {
                        if total.is_some_and(|total| downloaded < total) {
                            break download_error(format!(
                                "{url}: The download ended before all bytes arrived."
                            ));
                        }
                        return Ok(());
                    }
                    Err(error) => break download_error(error),
                }
            };
            self.retry(&mut attempts, failure).await?;
        }
    }
}

fn parse_range(value: &str) -> Option<(u64, u64, u64)> {
    let (range, total) = value.strip_prefix("bytes ")?.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let (start, end, total) = (start.parse().ok()?, end.parse().ok()?, total.parse().ok()?);
    (start <= end && end < total).then_some((start, end, total))
}

pub(super) fn system_network() -> Arc<dyn NetworkEnvironment> {
    Arc::new(SystemNetwork)
}
