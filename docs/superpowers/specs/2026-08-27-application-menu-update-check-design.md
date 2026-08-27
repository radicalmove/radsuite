# RADsuite Application Menu and Manual Update Check Design

## Goal

Simplify the project header while giving users an explicit way to check for a new RADsuite release. The result must use the existing RADsuite visual language and preserve the current one-click moon control for switching appearance.

## Approved User Experience

The right side of the project header contains two compact icon buttons:

1. The existing moon-image appearance button, unchanged in purpose and kept as a separate control.
2. A hamburger button that opens the application menu.

The application menu contains, in this order:

- **Check for updates**
- **Help**
- A non-interactive footer showing the current RADsuite version

The current version badge, “Saved locally” badge, “Cloud backup off/on” badge, and standalone Help button are removed from the header.

Routine local-saving and cloud-backup state is not shown in the menu. The existing command-bridge or saving error notices remain visible when a real failure requires the user’s attention. This work does not add cloud-backup configuration.

## Menu Behaviour

The hamburger is a native button with an accessible name such as “Open application menu,” an expanded-state attribute, and a relationship to the menu element. The icon remains visually compact and follows the existing button border, radius, hover, focus, light-theme, and dark-theme styling.

The menu is positioned below and aligned to the right edge of the hamburger. It uses the existing panel colours, borders, spacing, typography, shadows, and focus treatment. It must fit narrow layouts without overflowing the viewport.

The menu closes when:

- the hamburger is activated again;
- a menu action is chosen;
- the user clicks outside it; or
- the user presses Escape.

When opened, focus moves to the first actionable item. Arrow-key menu semantics are not required; normal Tab and Shift+Tab navigation is sufficient. Focus returns to the hamburger after Escape closes the menu.

Selecting Help closes the menu and opens the existing Help modal. The moon button continues to switch appearance directly without opening the menu.

## Manual Update Check

Selecting **Check for updates** calls the existing stable-update check with its force option enabled. A forced check bypasses the stored 24-hour throttle and any previously dismissed-version state for discovery purposes. The normal startup and 24-hour automatic checks remain unchanged.

The menu action is disabled and labelled **Checking for updates…** while a check is active, preventing duplicate requests.

The user receives one of three outcomes:

- **Update available:** the existing update notice appears with **Update now** and **Later** actions.
- **Already current:** a concise transient notice states that the installed RADsuite version is up to date.
- **Check failed:** the existing visible error-notice pattern explains that RADsuite could not check for updates.

Opening an available update must not automatically install it. Installation continues through the existing signed Tauri updater flow and existing progress UI.

Automatic background checks should not show an “up to date” notice. That confirmation is only for a user-initiated forced check.

## Component Boundaries

Create a focused application-menu component rather than adding more header markup to the already large `App.svelte` file. The component owns:

- open/closed presentation;
- outside-click and Escape handling;
- menu focus behaviour;
- rendering the update, Help, and version entries; and
- emitting callbacks for update checking and Help.

`App.svelte` remains responsible for updater state, invoking the updater API, opening the Help modal, and rendering update or error notices. The existing update-state helpers continue to own throttle and dismissed-version storage rules.

## Responsive and Visual Requirements

- The project title keeps priority and may wrap naturally.
- The moon and hamburger buttons remain grouped at the header’s right edge on wide layouts.
- On the existing narrow breakpoint, the action group follows the current header stacking rules and remains easy to reach.
- The menu is at least large enough for full action labels and never extends beyond the main-workspace viewport.
- Both themes retain readable contrast, visible hover states, and visible keyboard focus.
- No new icon library is introduced. The hamburger can be rendered with simple semantic markup or the three-line character, consistent with the existing asset-light approach.

## Testing

Add tests before implementation for:

- opening and closing the application menu;
- outside-click and Escape dismissal, including focus return;
- Help callback behaviour;
- forced update-check callback behaviour and the disabled checking label;
- display of the current version inside the menu;
- a forced check bypassing the 24-hour throttle;
- an “up to date” confirmation only after a manual check;
- preservation of the existing update-available and error paths; and
- absence of the routine saving, cloud-backup, version, and Help badges from the header.

Run the complete frontend test, Svelte check, and production-build commands. Because updater logic is affected, also run the existing release/updater contract tests. Manual visual verification should cover light and dark themes, a long project title, a narrow window, pointer dismissal, and keyboard dismissal.

## Scope Boundaries

This change does not add settings, cloud-backup controls, a new update channel, automatic installation, a new theme system, or a broader header redesign. It reorganises existing controls and exposes the already implemented forced update-check capability.
