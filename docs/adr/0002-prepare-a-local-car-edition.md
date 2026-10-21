# Prepare an independent default-paint car edition

Prepare a game-owned copy of the GT4 Nissan 350Z from the sibling carmodels repository, preserving geometry and wheel transforms while retaining only default-paint materials and images. The raw files rely on optional factory-paint variants and viewer-time material repairs; direct loading would not reproduce the intended appearance in Bevy. Apply the documented paint-alpha and marker-glass preview repairs offline, retain a source hash and preparation receipt, and leave the source repository unchanged.

This first build uses one car and one paint. Runtime factory-paint support can be reconsidered when car selection becomes part of the game; the prepared GLB is self-contained and does not depend on the showroom application.

Native visual QA showed the source's opaque untextured cabin backing rendering nearly white. Its bounded cabin geometry and separate roof/texture overlays identify the preview target; the game edition records an artistic dark-glass backing material. This does not establish recovered GT4 reflection shading, and the embedded texture bytes remain unchanged.
