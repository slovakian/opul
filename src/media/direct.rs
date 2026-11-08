use super::*;

pub(super) fn run_direct_download(
    context: &WorkerContext,
    id: &str,
    item: &StoredMediaDownload,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("could not start the direct downloader: {error}"))?;
    runtime.block_on(download_direct_resource(context, id, item))
}

async fn download_direct_resource(
    context: &WorkerContext,
    id: &str,
    item: &StoredMediaDownload,
) -> Result<(), String> {
    let source = Url::parse(&item.source_url)
        .map_err(|error| format!("invalid direct download URL: {error}"))?;
    let folder = Path::new(&item.output_folder);
    let temporary = direct_temporary_path(&item.output_folder, &source, &item.title, id);
    let existing_bytes = fs::metadata(&temporary)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("could not prepare the direct downloader: {error}"))?;
    let cookie_file = item_cookie_path(context, item);
    let (mut response, final_url) = send_direct_request(
        &client,
        &source,
        existing_bytes,
        item,
        cookie_file.as_deref(),
    )
    .await?;
    response
        .error_for_status_ref()
        .map_err(|error| format!("direct download request failed: {error}"))?;
    validate_direct_content_type(&response, &final_url, item.guided_kind)?;
    let filename = direct_response_filename(&response, &final_url, &item.title);
    let resumed = existing_bytes > 0
        && response.status() == reqwest::StatusCode::PARTIAL_CONTENT
        && response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with(&format!("bytes {existing_bytes}-")));
    let starting_bytes = if resumed { existing_bytes } else { 0 };
    let total_bytes = response
        .content_length()
        .map(|length| length.saturating_add(starting_bytes))
        .unwrap_or(0);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&temporary)
        .map_err(|error| format!("could not prepare the direct download file: {error}"))?;
    let started = Instant::now();
    let mut downloaded = starting_bytes;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("direct download interrupted: {error}"))?
    {
        loop {
            let state = context
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(id)
                .map(|item| item.state);
            match state {
                Some(MediaTransferState::Paused) => {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
                Some(MediaTransferState::Cancelled) | None => {
                    drop(file);
                    let _ = fs::remove_file(&temporary);
                    return Ok(());
                }
                _ => break,
            }
        }
        file.write_all(&chunk)
            .map_err(|error| format!("could not write the direct download: {error}"))?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        let transferred = downloaded.saturating_sub(starting_bytes);
        let speed = (transferred as f64 / elapsed) as u64;
        update_direct_progress(context, id, downloaded, total_bytes, speed);

        let limit = *context
            .max_download_bps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if limit > 0 {
            let expected = Duration::from_secs_f64(transferred as f64 / limit as f64);
            if expected > started.elapsed() {
                tokio::time::sleep(expected - started.elapsed()).await;
            }
        }
    }
    file.flush()
        .map_err(|error| format!("could not finish the direct download: {error}"))?;
    drop(file);
    let destination = reserve_direct_destination(folder, &filename, &temporary)?;
    let mut items = context
        .items
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(stored) = items.get_mut(id) {
        stored.output_path = Some(destination.to_string_lossy().into_owned());
        stored.total_bytes = downloaded;
        stored.downloaded_bytes = downloaded;
        stored.progress = 1.0;
    }
    Ok(())
}

async fn send_direct_request(
    client: &reqwest::Client,
    source: &Url,
    existing_bytes: u64,
    item: &StoredMediaDownload,
    cookie_file: Option<&Path>,
) -> Result<(reqwest::Response, Url), String> {
    let mut current = source.clone();
    let mut redirect_cookies = Vec::<RedirectCookie>::new();
    for redirects in 0..=10 {
        let mut request = client.get(current.as_str());
        if existing_bytes > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={existing_bytes}-"));
        }
        if let Some(referer) = item
            .options
            .referer
            .as_deref()
            .and_then(|referer| direct_referer_for_target(referer, &current))
        {
            request = request.header(reqwest::header::REFERER, referer);
        }
        if let Some(user_agent) = item.options.user_agent.as_deref() {
            request = request.header(reqwest::header::USER_AGENT, user_agent);
        }
        let same_origin = same_origin(source, &current);
        let authorization_scope = default_cookie_path(source.path());
        for header in &item.request_headers {
            let allowed = match header.name.as_str() {
                "authorization" => {
                    same_origin && cookie_path_matches(&authorization_scope, current.path())
                }
                "origin" => same_origin,
                "accept" | "accept-language" => true,
                _ => false,
            };
            if allowed {
                request = request.header(header.name.as_str(), header.value.as_str());
            }
        }
        let mut cookies = redirect_cookie_header(&redirect_cookies, &current);
        if let Some(cookie_header) = direct_cookie_header(cookie_file, &current) {
            cookies.push(cookie_header);
        }
        if !cookies.is_empty() {
            request = request.header(reqwest::header::COOKIE, cookies.join("; "));
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("direct download request failed: {error}"))?;
        update_redirect_cookies(&mut redirect_cookies, &current, response.headers());
        if !response.status().is_redirection() {
            return Ok((response, current));
        }
        if redirects == 10 {
            return Err("the direct media URL redirected more than 10 times".to_string());
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                "the direct media URL redirected without a valid Location".to_string()
            })?;
        let next = current
            .join(location)
            .map_err(|error| format!("the direct media redirect was invalid: {error}"))?;
        if !matches!(next.scheme(), "http" | "https") {
            return Err("the direct media URL redirected to an unsupported scheme".to_string());
        }
        current = next;
    }
    unreachable!("bounded redirect loop always returns")
}

pub(super) fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

#[derive(Clone)]
pub(super) struct RedirectCookie {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) host: String,
    pub(super) path: String,
    pub(super) secure: bool,
}

pub(super) fn update_redirect_cookies(
    cookies: &mut Vec<RedirectCookie>,
    source: &Url,
    headers: &reqwest::header::HeaderMap,
) {
    for value in headers.get_all(reqwest::header::SET_COOKIE) {
        let Some(value) = value.to_str().ok().filter(|value| value.len() <= 8_192) else {
            continue;
        };
        let mut parts = value.split(';');
        let Some((name, cookie_value)) = parts.next().and_then(|pair| pair.split_once('=')) else {
            continue;
        };
        let name = name.trim();
        let cookie_value = cookie_value.trim();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?={} \t".contains(&byte))
            || cookie_value
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte == b';')
        {
            continue;
        }
        let Some(host) = source.host_str().map(str::to_ascii_lowercase) else {
            continue;
        };
        let mut path = default_cookie_path(source.path());
        let mut secure = false;
        let mut remove = false;
        let mut rejected = false;
        for attribute in parts {
            let (attribute_name, attribute_value) = attribute
                .trim()
                .split_once('=')
                .map_or((attribute.trim(), ""), |(name, value)| {
                    (name.trim(), value.trim())
                });
            match attribute_name.to_ascii_lowercase().as_str() {
                // Redirect cookies are intentionally host-only. Correct public-suffix validation
                // for Domain cookies would require a browser cookie implementation.
                "domain" => rejected = true,
                "path" if attribute_value.starts_with('/') => path = attribute_value.to_string(),
                "secure" => secure = true,
                "max-age" => {
                    remove = attribute_value
                        .parse::<i64>()
                        .ok()
                        .is_some_and(|seconds| seconds <= 0)
                }
                _ => {}
            }
        }
        if rejected {
            continue;
        }
        cookies
            .retain(|cookie| !(cookie.name == name && cookie.host == host && cookie.path == path));
        if !remove && cookies.len() < 64 {
            cookies.push(RedirectCookie {
                name: name.to_string(),
                value: cookie_value.to_string(),
                host,
                path,
                secure,
            });
        }
    }
}

pub(super) fn default_cookie_path(request_path: &str) -> String {
    if !request_path.starts_with('/') || request_path == "/" {
        return "/".to_string();
    }
    request_path
        .rsplit_once('/')
        .map(|(directory, _)| if directory.is_empty() { "/" } else { directory })
        .unwrap_or("/")
        .to_string()
}

pub(super) fn redirect_cookie_header(cookies: &[RedirectCookie], target: &Url) -> Vec<String> {
    let Some(host) = target.host_str() else {
        return Vec::new();
    };
    let mut length = 0;
    cookies
        .iter()
        .filter(|cookie| {
            cookie.host.eq_ignore_ascii_case(host)
                && cookie_path_matches(&cookie.path, target.path())
                && (!cookie.secure || target.scheme() == "https")
        })
        .map_while(|cookie| {
            let value = format!("{}={}", cookie.name, cookie.value);
            length += value.len() + 2;
            (length <= 32 * 1024).then_some(value)
        })
        .collect()
}

pub(super) fn cookie_path_matches(cookie_path: &str, request_path: &str) -> bool {
    request_path == cookie_path
        || (request_path.starts_with(cookie_path)
            && (cookie_path.ends_with('/')
                || request_path.as_bytes().get(cookie_path.len()) == Some(&b'/')))
}

pub(super) fn direct_referer_for_target(referer: &str, target: &Url) -> Option<String> {
    let mut referer = Url::parse(referer).ok()?;
    if !matches!(referer.scheme(), "http" | "https") {
        return None;
    }
    if referer.scheme() == "https" && target.scheme() == "http" {
        return None;
    }
    referer.set_fragment(None);
    let _ = referer.set_username("");
    let _ = referer.set_password(None);
    if same_origin(&referer, target) {
        Some(referer.into())
    } else {
        Some(format!("{}/", referer.origin().ascii_serialization()))
    }
}

pub(super) fn origin_referer_for_target(referer: &str, target: &Url) -> Option<String> {
    let referer = Url::parse(referer).ok()?;
    if !matches!(referer.scheme(), "http" | "https")
        || (referer.scheme() == "https" && target.scheme() == "http")
    {
        return None;
    }
    Some(format!("{}/", referer.origin().ascii_serialization()))
}
