/** Default visible length for URLs and paths in labels and dialogs. */
export const URL_DISPLAY_MAX = 64;

/** Default visible length for filenames and download titles. */
export const FILENAME_DISPLAY_MAX = 48;

/**
 * Primary UI errors (dialogs, toasts, list cells): first line, strip huge
 * query blobs from embedded URLs, then hard-cap around 100 characters.
 */
export const ERROR_DISPLAY_MAX = 100;

const ELLIPSIS = "…";

/** Middle ellipsis so the start and end of a long token stay visible. */
export const truncateMiddle = (value: string, max: number): string => {
  if (value.length <= max) {
    return value;
  }

  if (max <= 1) {
    return ELLIPSIS;
  }

  const keep = max - ELLIPSIS.length;
  const head = Math.ceil(keep / 2);
  const tail = Math.floor(keep / 2);

  return `${value.slice(0, head)}${ELLIPSIS}${value.slice(value.length - tail)}`;
};

const HTTP_URL_PREFIX = /^https?:\/\//iu;

const looksLikeUrl = (value: string): boolean =>
  HTTP_URL_PREFIX.test(value) ||
  value.startsWith("magnet:") ||
  value.startsWith("//");

/**
 * Shorten a URL for primary UI. Drops a huge query/hash when the origin + path
 * already fits; otherwise middle-ellipsizes the whole string.
 */
export const truncateUrl = (
  url: string,
  max: number = URL_DISPLAY_MAX
): string => {
  if (url.length <= max) {
    return url;
  }

  if (HTTP_URL_PREFIX.test(url)) {
    try {
      const parsed = new URL(url);
      const base = `${parsed.origin}${parsed.pathname}`;

      if (
        (parsed.search.length > 0 || parsed.hash.length > 0) &&
        base.length + ELLIPSIS.length <= max
      ) {
        return `${base}${ELLIPSIS}`;
      }
    } catch {
      // Not a parseable absolute URL — fall through to middle ellipsis.
    }
  }

  return truncateMiddle(url, max);
};

/**
 * Shorten a filename, path, or download title. URL-shaped titles use
 * {@link truncateUrl}; everything else uses middle ellipsis.
 */
export const truncateFilename = (
  name: string,
  max: number = FILENAME_DISPLAY_MAX
): string => {
  if (looksLikeUrl(name)) {
    return truncateUrl(name, max);
  }

  if (name.length <= max) {
    return name;
  }

  const slash = Math.max(name.lastIndexOf("/"), name.lastIndexOf("\\"));

  if (slash >= 0) {
    const leaf = name.slice(slash + 1);

    if (leaf.length > 0 && leaf.length + ELLIPSIS.length <= max) {
      return `${ELLIPSIS}${leaf}`;
    }
  }

  return truncateMiddle(name, max);
};

const EMBEDDED_URL_BLOB =
  /(?<base>https?:\/\/[^\s?#]+)(?<suffix>[?#][^\s]*)/giu;

/** Collapse oversized `?…` / `#…` tails on absolute URLs embedded in text. */
const stripEmbeddedUrlBlobs = (text: string): string => {
  let result = "";
  let cursor = 0;

  for (const match of text.matchAll(EMBEDDED_URL_BLOB)) {
    const base = match.groups?.base ?? "";
    const suffix = match.groups?.suffix ?? "";
    const start = match.index;

    result += text.slice(cursor, start);

    result +=
      match[0].length <= 80 || suffix.length <= 12
        ? match[0]
        : `${base}${ELLIPSIS}`;

    cursor = start + match[0].length;
  }

  return `${result}${text.slice(cursor)}`;
};

/**
 * Short human summary for primary UI. Uses the first line, strips huge query
 * blobs from embedded URLs, then end-ellipsizes past {@link ERROR_DISPLAY_MAX}.
 */
export const truncateError = (
  message: string,
  max: number = ERROR_DISPLAY_MAX
): string => {
  const firstLine = (message.split(/\r?\n/u, 1)[0] ?? message).trim();
  const cleaned = stripEmbeddedUrlBlobs(firstLine);

  if (cleaned.length <= max) {
    return cleaned;
  }

  if (max <= 1) {
    return ELLIPSIS;
  }

  return `${cleaned.slice(0, max - ELLIPSIS.length)}${ELLIPSIS}`;
};
