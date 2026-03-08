---
triggers: ["game engine", "Unity", "Unreal", "Godot", "game development", "ECS", "game loop", "physics engine", "rendering pipeline", "shader"]
tools_allowed: ["read_file", "write_file", "bash"]
category: gaming
---

# Game Engine Development

When working with game engines, rendering, and game architecture:

1. Structure the game loop with a fixed timestep for physics and logic updates (e.g., 60 Hz) decoupled from a variable-rate render step; accumulate frame delta time and consume it in fixed increments to ensure deterministic simulation regardless of frame rate.

2. Adopt Entity Component System (ECS) architecture for data-oriented design: entities are plain IDs, components are pure data structs stored in contiguous arrays (SoA layout), and systems iterate over component archetypes to maximize cache coherence and enable trivial parallelism.

3. Integrate physics engines (PhysX, Havok, Bullet, Rapier) through an abstraction layer; run physics on the fixed timestep, interpolate rendered positions between physics steps for smooth visuals, and use collision layers/masks to minimize broadphase pair checks.

4. Organize the rendering pipeline into distinct passes: shadow map generation, G-buffer fill (deferred) or forward pass, lighting/shading, post-processing (bloom, tone mapping, FXAA/TAA), and UI overlay; use render graphs to express pass dependencies and automatically manage resource barriers.

5. Write shaders with performance in mind: minimize texture samples, use half-precision where visual quality permits, avoid dynamic branching in fragment shaders, leverage compute shaders for parallel workloads (particle updates, culling), and profile with GPU-specific tools (RenderDoc, Nsight, PIX).

6. Build a robust asset pipeline that converts source assets (FBX, PNG, WAV) into engine-optimized formats at import time; implement async streaming for large assets, use asset bundles with dependency tracking, and support hot-reload during development for rapid iteration.

7. Manage the scene graph efficiently with spatial partitioning (octree, BVH, or grid) for frustum culling, occlusion culling, and spatial queries; flatten deep hierarchies for rendering and update transforms only when dirty-flagged.

8. Implement a unified input system that abstracts platform-specific APIs into action mappings; support rebindable controls, dead zones for analog sticks, input buffering for action games, and simultaneous keyboard/mouse/gamepad with automatic device switching.

9. Design the audio system with a mixer graph: group sounds into buses (SFX, music, voice, ambient) with per-bus volume and effects; implement 3D spatialization with distance attenuation, use audio pools for frequently played sounds, and stream long audio files rather than loading them entirely.

10. Implement LOD (Level of Detail) systems that swap meshes, reduce bone counts, and simplify materials based on screen-space size or distance; use smooth transitions (dithering or cross-fade) to avoid visible popping, and integrate with the culling system to skip rendering off-screen LODs entirely.

11. Manage memory explicitly: use pool allocators for frequently created/destroyed objects (projectiles, particles), arena allocators for per-frame temporaries, and track memory budgets per subsystem; avoid runtime heap allocations in the hot path and pre-allocate to avoid fragmentation.

12. Implement a job system with a thread pool and lock-free work-stealing queues; express parallelism through dependency graphs of fine-grained jobs rather than coarse per-system threads, and ensure the main thread only blocks at well-defined sync points.

13. Use a component-based UI framework for in-game HUD and menus; batch draw calls, atlas UI textures, support resolution-independent layout with anchoring, and implement accessibility features (text scaling, colorblind modes, input remapping).

14. Profile continuously using both CPU (instruction-level sampling) and GPU (timestamp queries, pipeline statistics) profilers; establish frame-time budgets per subsystem (e.g., 4ms physics, 8ms rendering, 2ms AI) and set up automated alerts when budgets are exceeded.
