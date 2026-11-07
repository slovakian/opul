use super::*;

pub(super) fn score_guided_observation(
    observation: &GuidedObservation,
    kind: GuidedMediaKind,
) -> (i32, String) {
    let kind_score = match kind {
        GuidedMediaKind::Stream => 900,
        GuidedMediaKind::Video => 820,
        GuidedMediaKind::Audio => 780,
        GuidedMediaKind::Subtitle => 520,
        GuidedMediaKind::Image => 220,
        GuidedMediaKind::Unknown => 0,
    };
    let source_score = match observation.source {
        GuidedObservationSource::Dom => 90,
        GuidedObservationSource::Navigation => 70,
        GuidedObservationSource::Popup => 60,
        GuidedObservationSource::Performance => 30,
    };
    let mime_score = if observation.mime_type.as_deref().is_some_and(is_media_mime) {
        45
    } else {
        0
    };
    let response_score = if observation
        .status_code
        .is_some_and(|status| (200..300).contains(&status))
    {
        20
    } else {
        0
    };
    let size_score = observation
        .content_length
        .filter(|length| *length >= 1_000_000)
        .map(|_| 15)
        .unwrap_or(0);
    let tiny_image_penalty = if kind == GuidedMediaKind::Image
        && observation.width.is_some_and(|width| width < 160)
        && observation.height.is_some_and(|height| height < 90)
    {
        -180
    } else {
        0
    };
    let reason = format!(
        "{:?} observed via {:?} in {}{}",
        kind,
        observation.source,
        if observation.is_main_frame {
            "the main frame"
        } else {
            "a child frame"
        },
        observation
            .mime_type
            .as_deref()
            .map(|mime| format!(" ({mime})"))
            .unwrap_or_default()
    );
    (
        kind_score + source_score + mime_score + response_score + size_score + tiny_image_penalty,
        reason,
    )
}

pub(super) fn expiry_from_url(url: &Url) -> Option<u64> {
    let query = url
        .query_pairs()
        .map(|(name, value)| (name.to_ascii_lowercase(), value.into_owned()))
        .collect::<HashMap<_, _>>();
    if let Some(expiry) = ["expires", "expire", "expiry", "exp"]
        .iter()
        .find_map(|name| query.get(*name).and_then(|value| value.parse::<u64>().ok()))
    {
        return Some(if expiry >= 10_000_000_000 {
            expiry
        } else {
            expiry.saturating_mul(1_000)
        });
    }
    if let Some(expiry) = query.get("hdnts").and_then(|value| {
        value.split('~').find_map(|part| {
            part.strip_prefix("exp=")
                .and_then(|value| value.parse::<u64>().ok())
        })
    }) {
        return Some(expiry.saturating_mul(1_000));
    }
    if let Some(expiry) = query.get("se").and_then(|value| parse_utc_seconds(value)) {
        return Some(expiry.saturating_mul(1_000));
    }
    for (date_key, duration_key) in [
        ("x-amz-date", "x-amz-expires"),
        ("x-goog-date", "x-goog-expires"),
    ] {
        if let Some(expiry) = query
            .get(date_key)
            .and_then(|value| parse_utc_seconds(value))
            .zip(
                query
                    .get(duration_key)
                    .and_then(|value| value.parse::<u64>().ok()),
            )
            .map(|(start, duration)| start.saturating_add(duration).saturating_mul(1_000))
        {
            return Some(expiry);
        }
    }
    None
}

pub(super) fn parse_utc_seconds(value: &str) -> Option<u64> {
    if !value.is_ascii() {
        return None;
    }
    let digits =
        if value.len() == 16 && value.as_bytes().get(8) == Some(&b'T') && value.ends_with('Z') {
            format!(
                "{}{}{}{}{}{}",
                &value[0..4],
                &value[4..6],
                &value[6..8],
                &value[9..11],
                &value[11..13],
                &value[13..15]
            )
        } else if value.len() >= 20
            && value.as_bytes().get(4) == Some(&b'-')
            && value.as_bytes().get(7) == Some(&b'-')
            && value.as_bytes().get(10) == Some(&b'T')
            && value.as_bytes().get(13) == Some(&b':')
            && value.as_bytes().get(16) == Some(&b':')
            && value.ends_with('Z')
        {
            format!(
                "{}{}{}{}{}{}",
                &value[0..4],
                &value[5..7],
                &value[8..10],
                &value[11..13],
                &value[14..16],
                &value[17..19]
            )
        } else {
            return None;
        };
    let year = digits[0..4].parse::<i64>().ok()?;
    let month = digits[4..6].parse::<i64>().ok()?;
    let day = digits[6..8].parse::<i64>().ok()?;
    let hour = digits[8..10].parse::<i64>().ok()?;
    let minute = digits[10..12].parse::<i64>().ok()?;
    let second = digits[12..14].parse::<i64>().ok()?;
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=days_in_month).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(hour * 3_600 + minute * 60 + second)?;
    u64::try_from(seconds).ok()
}

pub(super) fn sanitize_guided_headers(headers: &[GuidedHttpHeader]) -> Vec<GuidedHttpHeader> {
    headers
        .iter()
        .filter_map(|header| {
            let name = header.name.trim().to_ascii_lowercase();
            let value = header.value.trim();
            if !matches!(
                name.as_str(),
                "accept" | "accept-language" | "authorization" | "origin"
            ) || value.is_empty()
                || name.len() > 64
                || value.len() > 8_192
                || value.contains(['\r', '\n'])
            {
                return None;
            }
            Some(GuidedHttpHeader {
                name,
                value: value.to_string(),
            })
        })
        .take(16)
        .collect()
}

pub(super) fn is_media_segment(url: &Url) -> bool {
    let path = url.path().to_ascii_lowercase();
    matches!(
        Path::new(&path)
            .extension()
            .and_then(|value| value.to_str()),
        Some("ts" | "m4s" | "cmfv" | "cmfa" | "m4f")
    ) || path.ends_with("/init.mp4")
        || path.ends_with("/init.webm")
}
