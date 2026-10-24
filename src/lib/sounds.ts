// Audio playback utility for Opul sound feedback

const soundFiles: Record<string, string[]> = {
  tap: ["tap-cjmw2.wav", "tap-hde9d.wav", "tap-md6m1.wav", "tap-smmhs.wav"],
  success: [
    "success-1aydz.wav",
    "success-1k20q.wav",
    "success-2asfa.wav",
    "success-ts4pm.wav",
  ],
  error: ["error-0z458.wav", "error-5e7cf.wav"],
  warning: ["warning-35yhp.wav", "warning-7wsgz.wav"],
  notification: ["notification-654y4.wav", "notification-s101m.wav"],
};

let soundsEnabled = true;

export function setSoundsEnabled(enabled: boolean) {
  soundsEnabled = enabled;
}

export function areSoundsEnabled(): boolean {
  return soundsEnabled;
}

export function playSound(
  category: "tap" | "success" | "error" | "warning" | "notification",
) {
  if (!soundsEnabled) return;
  const files = soundFiles[category];
  if (!files || files.length === 0) return;
  const randomFile = files[Math.floor(Math.random() * files.length)];
  const folder = category.charAt(0).toUpperCase() + category.slice(1);
  const audio = new Audio(`/sounds/${folder}/${randomFile}`);
  audio.volume = 0.5;
  audio.play().catch(() => {
    // Ignore audio play errors (e.g. user interaction policy)
  });
}
