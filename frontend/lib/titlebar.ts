/**
 * Overlay traffic-light clearance when the sidebar column is gone.
 *
 * Matches tauri `trafficLightPosition.x` (16) + light cluster (~52) + a
 * trailing gap equal to the leading inset (16). Keep layout CSS and the
 * unified-header e2e assertion on this token so they cannot drift.
 */
export const TRAFFIC_LIGHT_CLEARANCE_PX = 84;
