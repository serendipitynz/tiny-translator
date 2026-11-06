/**
 * Figures copied from lucide rather than depended on (snz-design decision-1): the app needs a
 * handful of them, and the package would ship every icon plus a component layer.
 *
 * Source: lucide-react v1.17.0, `dist/esm/icons/<name>.mjs` (`__iconNode`), with the shared
 * attributes from `dist/esm/defaultAttributes.mjs` — the package and version decision-1 pins, so
 * each figure below can be diffed against its source. Each entry is the `__iconNode` as published,
 * minus React's `key`; a figure is never redrawn or simplified here. lucide's LICENSE from the same
 * package sits beside this file as `LICENSE-lucide`, unaltered (snz-design doc-15 §3.2): it
 * carries a separate MIT notice for the figures lucide derives from Feather, `x` among them.
 */

export type IconName =
  | "history"
  | "settings"
  | "clipboard-paste"
  | "copy"
  | "x"
  | "arrow-left-right";

export type IconNode = readonly (readonly ["path" | "rect" | "circle", Readonly<Record<string, string>>])[];

export const ICON_SVG_ATTRS: Readonly<Record<string, string>> = {
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  "stroke-width": "2",
  "stroke-linecap": "round",
  "stroke-linejoin": "round",
};

export const ICONS: Readonly<Record<IconName, IconNode>> = {
  history: [
    ["path", { d: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" }],
    ["path", { d: "M3 3v5h5" }],
    ["path", { d: "M12 7v5l4 2" }],
  ],
  settings: [
    [
      "path",
      {
        d: "M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",
      },
    ],
    ["circle", { cx: "12", cy: "12", r: "3" }],
  ],
  "clipboard-paste": [
    ["path", { d: "M11 14h10" }],
    ["path", { d: "M16 4h2a2 2 0 0 1 2 2v1.344" }],
    ["path", { d: "m17 18 4-4-4-4" }],
    ["path", { d: "M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 1.793-1.113" }],
    ["rect", { x: "8", y: "2", width: "8", height: "4", rx: "1" }],
  ],
  copy: [
    ["rect", { width: "14", height: "14", x: "8", y: "8", rx: "2", ry: "2" }],
    ["path", { d: "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" }],
  ],
  x: [
    ["path", { d: "M18 6 6 18" }],
    ["path", { d: "m6 6 12 12" }],
  ],
  "arrow-left-right": [
    ["path", { d: "M8 3 4 7l4 4" }],
    ["path", { d: "M4 7h16" }],
    ["path", { d: "m16 21 4-4-4-4" }],
    ["path", { d: "M20 17H4" }],
  ],
};
