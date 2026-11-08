(async () => {
  const request = __REQUEST__;
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const dialogs = () => Array.from(document.querySelectorAll('[role="dialog"]')).filter((element) =>
    element.getClientRects().length && getComputedStyle(element).visibility !== "hidden" &&
    !element.closest('[inert], [aria-hidden="true"]'));
  const activeDialog = () => dialogs().at(-1);
  const wait = async (read) => {
    for (let attempt = 0; attempt < 300; attempt += 1) {
      const result = read();
      if (result) return result;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`UI condition timed out: ${request.target}`);
  };
  try {
    if (request.action === "switch") {
      const element = await wait(() => document.getElementById(request.target)
        ?.closest('[data-slot="field"]')?.querySelector('[role="switch"]'));
      if (element.getAttribute("aria-checked") !== String(request.enabled)) element.click();
      await wait(() => element.getAttribute("aria-checked") === String(request.enabled));
    } else if (request.action === "dismissed") {
      await wait(() => !dialogs().some((dialog) =>
        dialog.querySelector('[data-slot="dialog-title"]')?.textContent.trim() === request.target));
    } else if (request.action === "text") {
      await wait(() => activeDialog()?.innerText.replace(/\s+/g, " ").includes(request.target));
    } else {
      const button = await wait(() => Array.from(activeDialog()?.querySelectorAll("button") ?? []).find((element) => {
        const label = element.textContent.trim();
        return element.getClientRects().length && !element.disabled &&
          !element.closest('[inert], [aria-hidden="true"]') &&
          (request.prefix ? label.startsWith(request.target) : label === request.target);
      }));
      button.click();
    }
    await invoke("plugin:event|emit", { event: "opul-acceptance-ui", payload: {
      request_id: request.request_id, ok: true, action: request.action, target: request.target,
    }});
  } catch (error) {
    await invoke("plugin:event|emit", { event: "opul-acceptance-ui", payload: {
      request_id: request.request_id, ok: false, error: String(error),
      visible_text: document.body.innerText.slice(0, 4000),
    }});
  }
})();
