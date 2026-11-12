import { isTauri } from "@tauri-apps/api/core";

/**
 * Overlay traffic-light clearance for the Mac chrome row.
 *
 * Matches tauri `trafficLightPosition.x` (16) + light cluster (~52) + a
 * trailing gap equal to the leading inset (16). Keep layout CSS and the
 * unified-header e2e assertion on this token so they cannot drift.
 *
 * Used by the content header when the sidebar column is gone, and by the
 * Opul logo row when the docked sidebar shares the Overlay chrome axis.
 */
export const TRAFFIC_LIGHT_CLEARANCE_PX = 84;

/** Native Overlay traffic lights are present in the Tauri Mac shell. */
export const hasMacOverlayTrafficLights = (): boolean =>
  isTauri() && /Mac/u.test(navigator.userAgent);
