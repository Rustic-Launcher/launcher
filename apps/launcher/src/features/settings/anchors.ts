/**
 * Rows of the settings sections other pages link to: `/settings?section=…&focus=<anchor>` scrolls
 * the row into view (the row's element id is the anchor).
 */
export const SETTINGS_ANCHORS = ['rustic-server'] as const;
export type SettingsAnchor = (typeof SETTINGS_ANCHORS)[number];

/** The Rustic Server row of "Game & instances", which `server.no_rustic_server` errors point to. */
export const RUSTIC_SERVER_ANCHOR: SettingsAnchor = 'rustic-server';
