const explicitWebProtocol = /^https?:\/\//iu;
const magnetPrefix = /^magnet:\?/iu;
const torrentHash = /^urn:btih:(?:[a-f\d]{40}|[a-z2-7]{32})$/iu;

export const isDownloadLink = (text: string): boolean => {
  const link = text.trim();

  if (magnetPrefix.test(link)) {
    const parameters = new URLSearchParams(link.slice("magnet:?".length));

    return parameters.getAll("xt").some((topic) => torrentHash.test(topic));
  }

  if (!explicitWebProtocol.test(link) || /\s/u.test(link)) {
    return false;
  }

  try {
    const url = new URL(link);

    return Boolean(url.hostname);
  } catch {
    return false;
  }
};
