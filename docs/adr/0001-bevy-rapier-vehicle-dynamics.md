# Bevy and Rapier with a tunable vehicle controller

Use Bevy 0.20 and bevy_rapier3d 0.37 for the native macOS first build. A dynamic chassis uses four suspension raycasts and per-wheel tire forces so grip, weight transfer, braking and recoverable slides can be tuned together; the existing Rapier vehicle example is a reference rather than the handling specification. Keep simulation at 120 Hz and interpolate the rendered car and chase camera, so changing rendering performance does not change driving behavior.

The user confirmed this stack and a Forza Horizon-inspired arcade feel. Jev selected custom raycast wheels over the stock vehicle controller or kinematic movement, and a small set of modules over a monolith or a larger framework. Headless tests exercise the same Rapier simulation used by the game.
