---
triggers: ["autonomous vehicle", "self-driving", "ADAS", "lidar", "perception pipeline", "path planning", "autonomous drone", "V2X", "HD map", "behavioral planning", "sensor calibration"]
tools_allowed: ["read_file", "write_file", "bash"]
category: robotics
---

# Autonomous Vehicle and Drone Systems

When working with autonomous vehicles, ADAS, and drone software:

1. Build the perception pipeline as a modular fusion architecture: run independent detection networks per sensor modality (lidar 3D object detection via PointPillars/CenterPoint, camera 2D/3D detection via YOLO/BEVFormer, radar target tracking), then fuse detections in a common BEV (bird's-eye view) coordinate frame using a multi-object tracker (e.g., Hungarian assignment + Kalman filter or learned association) — output a unified object list with position, velocity, dimensions, classification, and per-object uncertainty estimates.

2. Integrate HD maps as a structured prior for perception and planning: load map tiles in a standard format (OpenDRIVE, Lanelet2, or proprietary protobuf), align the vehicle's localization to the map coordinate frame, and use lane geometry, traffic sign/signal positions, speed limits, and routing topology to constrain behavioral decisions — version maps rigorously and handle graceful degradation when map data is stale or unavailable.

3. Implement behavioral planning using a hierarchical architecture: a route planner selects the road-level path, a behavioral layer makes tactical decisions (lane follow, lane change, intersection handling, yielding) using state machines or decision trees, and a motion planner generates the kinematically feasible trajectory — define clear interfaces between layers, and make the behavioral layer's decisions explainable and auditable for safety validation.

4. Design path planning algorithms that respect vehicle dynamics: use lattice planners, optimization-based planners (e.g., IPOPT with vehicle kinematic/dynamic models), or sampling-based approaches (CL-RRT) that generate dynamically feasible trajectories — always incorporate velocity profiles, curvature constraints (minimum turning radius), acceleration/deceleration limits, and comfort metrics (jerk bounds) into the planning cost function.

5. Implement robust localization by fusing GPS/GNSS (RTK for centimeter accuracy when available), INS (IMU + wheel odometry for dead reckoning during GPS outages), lidar-based localization (point cloud matching against HD map or prior scans via NDT/ICP), and visual localization (landmark matching) — use a tightly coupled filter that produces a continuous, smooth pose estimate with uncertainty bounds, and detect and handle localization failures (e.g., tunnel entry/exit, GPS multipath in urban canyons).

6. Implement V2X (Vehicle-to-Everything) communication using the ETSI ITS-G5 or C-V2X (PC5/Uu) stack: broadcast and receive CAM (Cooperative Awareness Messages) and DENM (Decentralized Environmental Notification Messages), integrate SPaT (Signal Phase and Timing) data from infrastructure for traffic light handling, and use CPM (Collective Perception Messages) to extend perception beyond line-of-sight — always validate and rate-limit incoming V2X data as it is an untrusted input source.

7. Comply with functional safety standards by applying ISO 26262 for road vehicles (determine ASIL levels for each function, perform HARA, implement safety mechanisms with appropriate diagnostic coverage) and ISO 21448 (SOTIF) for identifying and mitigating performance limitations and foreseeable misuse — maintain a safety case that links hazards to requirements to implementation to verification evidence, and use runtime safety monitors that can override the autonomy stack and execute minimum risk maneuvers.

8. Build a simulation-based testing infrastructure using simulators (CARLA, LGSVL/SVL, NVIDIA DRIVE Sim, AirSim for drones) with scenario description languages (OpenSCENARIO 2.0) to define reproducible test cases — run thousands of parameterized scenarios in CI covering nominal driving, edge cases (cut-ins, jaywalkers, adverse weather), and adversarial scenarios, and track pass rates, regression metrics, and coverage of the ODD (Operational Design Domain).

9. Design sensor calibration pipelines that compute and maintain extrinsic transforms (rotation + translation) between all sensors (lidar-to-camera, camera-to-camera, lidar-to-IMU, radar-to-lidar) and intrinsic parameters (camera focal length, distortion, lidar beam angles) — implement both offline calibration (using calibration targets in controlled environments) and online calibration monitoring that detects and flags calibration drift from vibration or thermal effects during operation.

10. Architect OTA (Over-the-Air) update systems with dual-bank (A/B) partitioning for atomic rollback, cryptographic signing and verification of all update packages, staged rollout with canary deployments and automatic rollback on anomaly detection — separate safety-critical components (perception, planning, control) from non-critical ones (infotainment, logging) with different update cadences and approval gates, and ensure the vehicle remains in a safe state throughout the update process.

11. Build comprehensive data recording and replay systems that capture all sensor data (lidar point clouds, camera frames, radar returns, GPS/IMU, CAN bus), perception outputs, planning decisions, and vehicle state at full rate with precise timestamps — use efficient serialization (protobuf, MCap/MCAP format) and storage (compressed, indexed for random access), and implement deterministic replay so any recorded scenario can be re-processed through updated perception/planning stacks for regression testing.

12. Manage edge cases systematically: maintain a taxonomy of known difficult scenarios (construction zones, emergency vehicles, unusual road users, sensor degradation), implement specific detection and handling logic for each category, track encounter frequency and system performance per category in operational data, and use discovered edge cases to generate new simulation scenarios and expand the training/validation dataset — treat edge case management as a continuous process, not a one-time exercise.

13. Implement a minimum risk condition (MRC) framework that defines safe fallback behaviors for each type of system failure or ODD exit: controlled stop in lane, pull-over-and-stop, reduce speed operation, or hand-off to remote operator — the MRC planner must operate independently from the primary autonomy stack with its own simplified perception and planning, and must be validated to a higher safety integrity level than the nominal system.
