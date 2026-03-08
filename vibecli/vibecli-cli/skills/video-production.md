---
triggers: ["video production", "video editing", "Premiere Pro", "Final Cut", "DaVinci Resolve", "After Effects", "motion graphics"]
tools_allowed: ["read_file", "write_file", "bash"]
category: creative
---

# Video Production

When working with video production:

1. Plan pre-production thoroughly by writing a script or outline first, creating storyboards for complex sequences, building a detailed shot list organized by location to minimize setup changes, and scouting locations for lighting conditions, power availability, and ambient noise.
2. Apply shooting fundamentals consistently: follow the rule of thirds for framing, use three-point lighting (key, fill, back) as a baseline, record audio with dedicated microphones (lavalier for interviews, shotgun for dialogue), shoot in the highest quality your storage and workflow can handle, and always capture B-roll.
3. Structure your editing workflow in phases: begin with an assembly edit to lay out all footage in sequence order, create a rough cut focusing on story and pacing, refine into a fine cut with precise trim points and transitions, then lock the edit before moving to color and audio finishing.
4. Apply color grading in DaVinci Resolve using a node-based workflow: start with a primary correction node for exposure and white balance, add a secondary node for specific color adjustments, use LUTs as starting points rather than final looks, match shots within scenes before applying creative grades, and use scopes (waveform, vectorscope, parade) to ensure broadcast-safe levels.
5. Mix audio with proper gain staging: normalize dialogue to -12 to -6 dBFS, set music beds 15-20 dB below dialogue, add room tone under edits to maintain consistent ambiance, use compression to even out dynamic range in speech, apply EQ to remove rumble below 80Hz on voice tracks, and target -14 LUFS for YouTube or -24 LUFS for broadcast.
6. Create motion graphics in After Effects by pre-composing complex elements, using adjustment layers for global effects, building with shape layers and masks for scalable graphics, leveraging expressions for automated animation (wiggle, loopOut, time), and organizing the project panel with folders matching the timeline structure.
7. Configure export settings based on delivery: use H.264 at 15-50 Mbps for web delivery, ProRes 422 for intermediate editing files, ProRes 4444 for graphics with alpha channels; match frame rate to source footage, export at the platform's recommended resolution, and use variable bitrate (VBR 2-pass) for the best quality-to-size ratio.
8. Design titles and typography with readability as the priority: maintain safe margins (10% action safe, 20% title safe), use sans-serif fonts at sufficient size for mobile viewing, limit font choices to two per project, ensure adequate contrast against backgrounds, and animate text entries and exits to match the pacing of the edit.
9. Execute green screen and chroma key shots by lighting the green screen evenly and separately from the subject, maintaining distance between subject and screen to minimize spill, shooting at the highest bit depth available, pulling the key in DaVinci Resolve or After Effects with edge refinement and spill suppression, and compositing with matched lighting and perspective.
10. Edit multicam footage by syncing all angles using timecode, audio waveform, or slate markers, creating a multicam clip or group in your NLE, cutting live between angles in real time, then refining individual cuts in the timeline; maintain consistent color across cameras by shooting with matching profiles.
11. Set up proxy workflows for performance by transcoding high-resolution footage (4K, 6K, 8K) to lower-resolution proxies (1080p ProRes Proxy or H.264), editing with proxies enabled, then relinking to original media before final export; Premiere Pro, Final Cut, and DaVinci Resolve all support automatic proxy generation.
12. Prepare delivery formats per platform: YouTube (16:9, 2160p preferred, H.264), Instagram feed (1:1 or 4:5, 60s max for feed), Instagram Reels/TikTok (9:16, 60-90s), LinkedIn (16:9 or 1:1, add captions), Twitter/X (16:9, under 2:20), and broadcast (ProRes or DNxHR at specified specs); always embed captions for accessibility.
