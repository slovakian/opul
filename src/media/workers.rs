use super::*;

pub(super) fn spawn_worker(context: WorkerContext, id: String) {
    {
        let mut workers = context
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !workers.active.insert(id.clone()) {
            return;
        }
    }
    let failure_workers = Arc::clone(&context.workers);
    let failure_items = Arc::clone(&context.items);
    let failure_store_path = context.store_path.clone();
    let failure_id = id.clone();
    let worker_label = id.chars().take(8).collect::<String>();
    let result = thread::Builder::new()
        .name(format!("opul-media-{worker_label}"))
        .spawn(move || {
            run_worker(context.clone(), id.clone());
            let should_retry = {
                let mut workers = context
                    .workers
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                workers.active.remove(&id);
                workers.retry_requested.remove(&id)
            };
            if should_retry {
                restart_download(context, id);
            }
        });
    if result.is_err() {
        failure_workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .active
            .remove(&failure_id);
        let mut items = failure_items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(item) = items.get_mut(&failure_id) {
            item.state = MediaTransferState::Failed;
            item.error_message = Some("could not start a download worker".to_string());
        }
        save_downloads(&failure_store_path, &items);
    }
}

pub(super) fn restart_download(context: WorkerContext, id: String) {
    let shutdown = context
        .gate
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .shutdown;
    if shutdown {
        return;
    }
    let can_start = {
        let mut items = context
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(item) = items.get_mut(&id) else {
            return;
        };
        let can_start = item.direct_download || context.yt_dlp.is_some();
        item.state = if can_start {
            MediaTransferState::Queued
        } else {
            MediaTransferState::Failed
        };
        item.error_message =
            (!can_start).then(|| "yt-dlp is required to retry this download".to_string());
        item.progress = 0.0;
        item.download_rate = 0;
        item.eta_seconds = None;
        save_downloads(&context.store_path, &items);
        can_start
    };
    if can_start {
        spawn_worker(context, id);
    }
}

pub(super) fn run_worker(context: WorkerContext, id: String) {
    if !wait_for_slot(&context, &id) {
        return;
    }
    let result = run_download(&context, &id);
    {
        let mut items = context
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(item) = items.get_mut(&id) {
            if item.state != MediaTransferState::Cancelled {
                match &result {
                    Ok(()) => {
                        item.state = MediaTransferState::Finished;
                        item.progress = 1.0;
                        item.downloaded_bytes = item.downloaded_bytes.max(item.total_bytes);
                        item.error_message = None;
                        remove_transfer_credentials(&context, item);
                    }
                    Err(reason) => {
                        item.state = MediaTransferState::Failed;
                        item.error_message = Some(reason.clone());
                    }
                }
            }
            item.download_rate = 0;
            item.eta_seconds = None;
        }
        save_downloads(&context.store_path, &items);
    }
    context
        .processes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&id);
    let mut gate = context
        .gate
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    gate.running = gate.running.saturating_sub(1);
    drop(gate);
    context.gate.1.notify_all();
}

pub(super) fn wait_for_slot(context: &WorkerContext, id: &str) -> bool {
    loop {
        let state = context
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(id)
            .map(|item| item.state);
        match state {
            None | Some(MediaTransferState::Cancelled | MediaTransferState::Finished) => {
                return false;
            }
            Some(MediaTransferState::Paused) => {
                let gate = context
                    .gate
                    .0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let _ = context
                    .gate
                    .1
                    .wait_timeout(gate, Duration::from_millis(250));
            }
            _ => {
                let mut gate = context
                    .gate
                    .0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if gate.shutdown {
                    return false;
                }
                if gate.running < gate.maximum {
                    gate.running += 1;
                    drop(gate);
                    let mut items = context
                        .items
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if let Some(item) = items.get_mut(id) {
                        item.state = MediaTransferState::Downloading;
                        item.error_message = None;
                    }
                    save_downloads(&context.store_path, &items);
                    return true;
                }
                let _ = context
                    .gate
                    .1
                    .wait_timeout(gate, Duration::from_millis(250));
            }
        }
    }
}

pub(super) fn run_download(context: &WorkerContext, id: &str) -> Result<(), String> {
    let mut item = context
        .items
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(id)
        .cloned()
        .ok_or_else(|| "download disappeared from the queue".to_string())?;
    item.request_headers = item_request_headers(context, &item);
    if item.direct_download {
        return run_direct_download(context, id, &item);
    }
    let yt_dlp = context
        .yt_dlp
        .as_ref()
        .ok_or_else(|| "yt-dlp is required for this download".to_string())?;
    let mut command = yt_dlp.command();
    command
        .arg("--newline")
        .arg("--continue")
        .arg("--no-playlist")
        .arg("--progress-template")
        .arg("download:opul-progress\t%(progress.status)s\t%(progress.downloaded_bytes)s\t%(progress.total_bytes_estimate)s\t%(progress.speed)s\t%(progress.eta)s\t%(progress.tmpfilename)s")
        .arg("--progress-template")
        .arg("postprocess:opul-postprocess\t%(progress.status)s\t%(info.filepath)s")
        .arg("--print")
        .arg("after_move:opul-file\t%(filepath)s")
        .arg("--output")
        .arg(output_template(&item))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_format_args(&mut command, item.options.format, context.ffmpeg.is_some());
    if item.options.download_subtitles {
        command.args(["--write-subs", "--write-auto-subs"]);
        let languages = if item.options.subtitle_languages.is_empty() {
            "all,-live_chat".to_string()
        } else {
            item.options.subtitle_languages.join(",")
        };
        command.arg("--sub-langs").arg(languages);
        if context.ffmpeg.is_some() {
            command.args(["--convert-subs", "srt"]);
            if item.options.embed_subtitles {
                command.arg("--embed-subs");
            }
        }
    }
    if item.options.write_thumbnail {
        command.arg("--write-thumbnail");
        if context.ffmpeg.is_some() {
            command.args(["--convert-thumbnails", "jpg"]);
        }
    }
    if item.options.write_metadata {
        command.arg("--write-info-json");
    }
    if let Some(referer) = item.options.referer.as_deref() {
        command.arg("--referer").arg(referer);
    }
    if let Some(user_agent) = item.options.user_agent.as_deref() {
        command.arg("--user-agent").arg(user_agent);
    }
    for header in &item.request_headers {
        command
            .arg("--add-header")
            .arg(format!("{}:{}", header.name, header.value));
    }
    if let Some(cookie_file) = item_cookie_path(context, &item) {
        command.arg("--cookies").arg(cookie_file);
    }
    if let Some(ffmpeg) = context.ffmpeg.as_ref() {
        command
            .arg("--ffmpeg-location")
            .arg(ffmpeg.parent().expect("bundled FFmpeg has a parent"));
    }
    let max_download_bps = *context
        .max_download_bps
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if max_download_bps > 0 {
        command
            .arg("--limit-rate")
            .arg(max_download_bps.to_string());
    }
    command.arg("--").arg(&item.source_url);

    command.process_group(0);
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not run yt-dlp: {error}"))?;
    let pid = child.id();
    context
        .processes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(id.to_string(), pid);
    let state = context
        .items
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(id)
        .map(|item| item.state);
    match state {
        Some(MediaTransferState::Paused) => signal_process(pid, "STOP"),
        None | Some(MediaTransferState::Cancelled) => signal_process(pid, "TERM"),
        _ => {}
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "yt-dlp stdout was unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "yt-dlp stderr was unavailable".to_string())?;
    let stdout_items = Arc::clone(&context.items);
    let output_id = id.to_string();
    let stdout_thread = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            apply_progress_line(&stdout_items, &output_id, &line);
        }
    });
    let stderr_thread = thread::spawn(move || {
        let mut lines = VecDeque::with_capacity(12);
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if lines.len() == 12 {
                lines.pop_front();
            }
            lines.push_back(line);
        }
        lines
    });
    let status = child
        .wait()
        .map_err(|error| format!("could not wait for yt-dlp: {error}"))?;
    let _ = stdout_thread.join();
    let stderr_lines = stderr_thread.join().unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        let message = stderr_lines
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        Err(if message.is_empty() {
            format!("yt-dlp exited with {status}")
        } else {
            message
        })
    }
}
