use super::*;

impl GuidedExtractionSession {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(GuidedExtractionState::default()),
        })
    }

    pub fn observe(&self, observation: GuidedObservation) -> Vec<GuidedCandidate> {
        self.observe_batch(vec![observation])
    }

    pub fn observe_batch(&self, observations: Vec<GuidedObservation>) -> Vec<GuidedCandidate> {
        let observations = observations.into_iter().take(256).collect::<Vec<_>>();
        let observed_at_ms = observations
            .iter()
            .map(|observation| observation.observed_at_ms)
            .max()
            .filter(|value| *value > 0)
            .unwrap_or_else(now_millis);
        let candidates = observations
            .iter()
            .filter_map(|observation| {
                candidate_from_observation(observation).map(|candidate| {
                    let identity = guided_candidate_identity(observation, candidate.candidate.kind);
                    (identity, candidate)
                })
            })
            .collect::<Vec<_>>();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for (identity, candidate) in candidates {
            insert_guided_candidate(&mut state, identity, candidate);
        }
        guided_candidate_snapshot(&state, observed_at_ms)
    }

    pub(crate) fn requires_video_tools(
        &self,
        candidate_id: &str,
        options: &MediaDownloadOptions,
    ) -> Result<bool, CoreError> {
        Ok(!super::open::direct_download_compatible(
            self.resolve_candidate(candidate_id)?.candidate.kind,
            options,
        ))
    }

    pub fn candidates(&self) -> Vec<GuidedCandidate> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guided_candidate_snapshot(&state, now_millis())
    }

    pub fn limit_reached(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .limit_reached
    }

    pub fn clear(&self) {
        *self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = GuidedExtractionState::default();
    }
}

impl GuidedExtractionSession {
    pub(super) fn resolve_candidate(&self, id: &str) -> Result<GuidedCandidateEntry, CoreError> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut entry = state
            .candidates
            .values()
            .find(|entry| entry.candidate.id == id)
            .cloned()
            .ok_or_else(|| CoreError::InvalidConfiguration {
                reason: "the guided media candidate is no longer available".to_string(),
            })?;
        entry.candidate.is_expired = entry
            .candidate
            .expires_at_ms
            .is_some_and(|expiry| expiry <= now_millis());
        if entry.candidate.is_expired {
            return Err(CoreError::InvalidConfiguration {
                reason: "that media URL has expired; play it again to capture a fresh URL"
                    .to_string(),
            });
        }
        Ok(entry)
    }
}

pub(super) fn insert_guided_candidate(
    state: &mut GuidedExtractionState,
    identity: String,
    candidate: GuidedCandidateEntry,
) {
    let candidate_key = state.aliases.get(&identity).cloned().or_else(|| {
        state
            .candidates
            .iter()
            .find(|(_, existing)| {
                existing.candidate.kind == candidate.candidate.kind
                    && existing.candidate.url == candidate.candidate.url
                    && same_candidate_context(existing, &candidate)
            })
            .map(|(key, _)| key.clone())
    });
    if let Some(candidate_key) = candidate_key {
        if state.aliases.len() < 4_000 {
            state.aliases.insert(identity, candidate_key.clone());
        }
        if let Some(existing) = state.candidates.get_mut(&candidate_key) {
            merge_guided_candidate(existing, candidate);
        }
    } else if state.candidates.len() < 2_000 {
        if state.aliases.len() < 4_000 {
            state.aliases.insert(identity.clone(), identity.clone());
        }
        state.candidates.insert(identity, candidate);
    } else {
        state.limit_reached = true;
        let eviction = state
            .candidates
            .iter()
            .min_by_key(|(_, entry)| (entry.candidate.score, entry.candidate.last_seen_ms))
            .map(|(key, entry)| (key.clone(), entry.candidate.score));
        if let Some((evicted_key, evicted_score)) = eviction
            && candidate.candidate.score > evicted_score
        {
            state.candidates.remove(&evicted_key);
            state.aliases.retain(|_, key| key != &evicted_key);
            if state.aliases.len() < 4_000 {
                state.aliases.insert(identity.clone(), identity.clone());
            }
            state.candidates.insert(identity, candidate);
        }
    }
}

pub(super) fn merge_guided_candidate(
    existing: &mut GuidedCandidateEntry,
    mut candidate: GuidedCandidateEntry,
) {
    let id = existing.candidate.id.clone();
    let first_seen_ms = existing
        .candidate
        .first_seen_ms
        .min(candidate.candidate.first_seen_ms);
    let last_seen_ms = existing
        .candidate
        .last_seen_ms
        .max(candidate.candidate.last_seen_ms);
    let occurrences = existing.candidate.occurrences.saturating_add(1);
    if existing.candidate.url != candidate.candidate.url
        && candidate.candidate.last_seen_ms < existing.candidate.last_seen_ms
    {
        existing.candidate.occurrences = occurrences;
        return;
    }
    if existing.candidate.url == candidate.candidate.url
        && candidate.candidate.score < existing.candidate.score
    {
        existing.candidate.last_seen_ms = last_seen_ms;
        existing.candidate.occurrences = occurrences;
        if existing.headers.is_empty() && !candidate.headers.is_empty() {
            existing.headers = candidate.headers;
        }
        if existing.candidate.mime_type.is_none() {
            existing.candidate.mime_type = candidate.candidate.mime_type;
        }
        if existing.candidate.frame_url.is_none() {
            existing.candidate.frame_url = candidate.candidate.frame_url;
            existing.candidate.referer = candidate.candidate.referer;
        }
        return;
    }
    let same_resource_origin = Url::parse(&existing.candidate.url)
        .ok()
        .zip(Url::parse(&candidate.candidate.url).ok())
        .is_some_and(|(existing_url, candidate_url)| same_origin(&existing_url, &candidate_url));
    if candidate.headers.is_empty() && same_resource_origin {
        candidate.headers = existing.headers.clone();
    }
    if candidate.candidate.mime_type.is_none() {
        candidate.candidate.mime_type = existing.candidate.mime_type.clone();
    }
    if candidate.candidate.frame_url.is_none() && existing.candidate.frame_url.is_some() {
        candidate.candidate.frame_url = existing.candidate.frame_url.clone();
        candidate.candidate.referer = existing.candidate.referer.clone();
    }
    candidate.candidate.id = id;
    candidate.candidate.first_seen_ms = first_seen_ms;
    candidate.candidate.last_seen_ms = last_seen_ms;
    candidate.candidate.occurrences = occurrences;
    *existing = candidate;
}

pub(super) fn same_candidate_context(
    left: &GuidedCandidateEntry,
    right: &GuidedCandidateEntry,
) -> bool {
    if left.headers.iter().any(is_sensitive_guided_header)
        || right.headers.iter().any(is_sensitive_guided_header)
    {
        return left.candidate.referer == right.candidate.referer;
    }
    true
}

pub(super) fn is_sensitive_guided_header(header: &GuidedHttpHeader) -> bool {
    matches!(header.name.as_str(), "authorization" | "origin")
}

pub(super) fn candidate_from_observation(
    observation: &GuidedObservation,
) -> Option<GuidedCandidateEntry> {
    if observation.url.len() > 16_384
        || observation.page_url.len() > 16_384
        || observation.title.len() > 4_096
        || observation.initiator.len() > 128
        || observation.status_code.is_some_and(|status| status >= 400)
    {
        return None;
    }
    let mut url = Url::parse(&observation.url).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_fragment(None);
    if observation.source == GuidedObservationSource::Performance
        && (is_media_segment(&url)
            || observation.mime_type.as_deref().is_some_and(|mime| {
                let mime = mime.split(';').next().unwrap_or_default().trim();
                ["video/mp2t", "video/iso.segment", "audio/iso.segment"]
                    .iter()
                    .any(|segment_mime| mime.eq_ignore_ascii_case(segment_mime))
            }))
    {
        return None;
    }
    let mut kind = classify_guided_resource(
        url.as_str(),
        &observation.initiator,
        observation.mime_type.as_deref(),
    );
    if kind == GuidedMediaKind::Unknown
        && observation.source == GuidedObservationSource::Navigation
        && observation
            .status_code
            .is_some_and(|status| (200..300).contains(&status))
        && observation
            .content_length
            .is_some_and(|length| length >= 256 * 1024)
        && observation.mime_type.as_deref().is_some_and(|mime| {
            matches!(
                mime.split(';').next().unwrap_or_default().trim(),
                "application/octet-stream" | "binary/octet-stream"
            )
        })
    {
        kind = GuidedMediaKind::Video;
    }
    if kind == GuidedMediaKind::Unknown {
        return None;
    }
    let observed_at_ms = if observation.observed_at_ms == 0 {
        now_millis()
    } else {
        observation.observed_at_ms
    };
    let referer = valid_http_url(observation.frame_url.as_deref())
        .or_else(|| valid_http_url(Some(&observation.page_url)))
        .unwrap_or_else(|| url.as_str().to_string());
    let title = bounded_candidate_title(&observation.title, &url);
    let expires_at_ms = expiry_from_url(&url);
    let headers = sanitize_guided_headers(&observation.headers);
    let (score, reason) = score_guided_observation(observation, kind);
    let identity = guided_candidate_identity(observation, kind);
    let id = hex::encode(Sha1::digest(identity.as_bytes()));
    Some(GuidedCandidateEntry {
        candidate: GuidedCandidate {
            id,
            url: url.into(),
            title,
            kind,
            referer,
            frame_url: valid_http_url(observation.frame_url.as_deref()),
            initiator: observation.initiator.chars().take(128).collect(),
            mime_type: observation
                .mime_type
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.chars().take(256).collect()),
            score,
            occurrences: 1,
            first_seen_ms: observed_at_ms,
            last_seen_ms: observed_at_ms,
            expires_at_ms,
            is_expired: expires_at_ms.is_some_and(|expiry| expiry <= observed_at_ms),
            reason,
            direct_download: matches!(
                kind,
                GuidedMediaKind::Video
                    | GuidedMediaKind::Audio
                    | GuidedMediaKind::Subtitle
                    | GuidedMediaKind::Image
            ),
        },
        headers,
    })
}

pub(super) fn guided_candidate_identity(
    observation: &GuidedObservation,
    kind: GuidedMediaKind,
) -> String {
    if let Some(element_key) = observation
        .element_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let frame = observation
            .frame_url
            .as_deref()
            .unwrap_or(&observation.page_url);
        return format!(
            "element:{kind:?}:{}:{}:{}",
            frame,
            observation.initiator.to_ascii_lowercase(),
            element_key.chars().take(512).collect::<String>()
        );
    }
    if kind == GuidedMediaKind::Stream
        && let Ok(url) = Url::parse(&observation.url)
    {
        let context = guided_sensitive_context(observation);
        return format!("stream:{}{context}", normalized_stream_url(url));
    }
    let mut url = observation.url.clone();
    if let Ok(mut parsed) = Url::parse(&url) {
        parsed.set_fragment(None);
        url = parsed.into();
    }
    let context = guided_sensitive_context(observation);
    format!("url:{url}{context}")
}

pub(super) fn guided_sensitive_context(observation: &GuidedObservation) -> String {
    if observation.headers.iter().any(|header| {
        matches!(
            header.name.trim().to_ascii_lowercase().as_str(),
            "authorization" | "origin"
        )
    }) {
        format!(
            ":context:{}",
            observation
                .frame_url
                .as_deref()
                .unwrap_or(&observation.page_url)
        )
    } else {
        String::new()
    }
}

pub(super) fn normalized_stream_url(mut url: Url) -> Url {
    let mut semantic_query = url
        .query_pairs()
        .filter(|(name, _)| !is_volatile_signature_parameter(name))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    semantic_query.sort();
    url.set_fragment(None);
    url.set_query(None);
    if !semantic_query.is_empty() {
        url.query_pairs_mut().extend_pairs(semantic_query);
    }
    url
}

pub(super) fn is_volatile_signature_parameter(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    matches!(
        name.as_str(),
        "token"
            | "access_token"
            | "auth"
            | "authorization"
            | "jwt"
            | "sig"
            | "signature"
            | "policy"
            | "key-pair-id"
            | "expires"
            | "expire"
            | "expiry"
            | "exp"
            | "hdnts"
            | "st"
            | "se"
    ) || name.starts_with("x-amz-")
        || name.starts_with("x-goog-")
}

pub(super) fn guided_candidate_snapshot(
    state: &GuidedExtractionState,
    observed_at_ms: u64,
) -> Vec<GuidedCandidate> {
    let mut candidates = state
        .candidates
        .values()
        .map(|entry| entry.candidate.clone())
        .collect::<Vec<_>>();
    for candidate in &mut candidates {
        candidate.is_expired = candidate
            .expires_at_ms
            .is_some_and(|expiry| expiry <= observed_at_ms);
    }
    candidates.sort_by(|left, right| {
        left.is_expired
            .cmp(&right.is_expired)
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| right.last_seen_ms.cmp(&left.last_seen_ms))
            .then_with(|| left.url.cmp(&right.url))
    });
    candidates
}

pub(super) fn valid_http_url(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.len() > 16_384 {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_fragment(None);
    Some(url.into())
}

pub(super) fn bounded_candidate_title(title: &str, url: &Url) -> String {
    let trimmed = title.trim();
    if !trimmed.is_empty() {
        return trimmed.chars().take(512).collect();
    }
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|value| !value.is_empty())
        .or_else(|| url.host_str())
        .unwrap_or("Detected media")
        .chars()
        .take(512)
        .collect()
}
