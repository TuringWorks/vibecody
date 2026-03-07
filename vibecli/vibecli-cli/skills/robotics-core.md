---
triggers: ["robotics", "ROS", "ROS2", "robot", "kinematics", "motion planning", "SLAM", "sensor fusion", "actuator", "inverse kinematics", "trajectory planning", "robotic arm"]
tools_allowed: ["read_file", "write_file", "bash"]
category: robotics
---

# Robotics Core Systems

When working with robotics software, ROS2, and motion control:

1. Architect ROS2 nodes following the managed lifecycle pattern (Unconfigured, Inactive, Active, Finalized) so hardware drivers and processing pipelines can be started, stopped, and reconfigured cleanly — use component nodes loaded into a single process via composition for latency-critical pipelines, and separate processes for isolation where a node crash should not take down the entire system.

2. Implement sensor fusion using Extended Kalman Filters (EKF) or Unscented Kalman Filters (UKF) for Gaussian noise models, and particle filters for non-Gaussian or multimodal distributions — fuse IMU, wheel odometry, GPS, and visual odometry with properly characterized noise covariances, and always propagate the full state covariance matrix so downstream consumers can make uncertainty-aware decisions.

3. Build SLAM systems by choosing the right algorithm for your constraints: EKF-SLAM for small environments, graph-based SLAM (g2o, GTSAM) for large-scale mapping, ORB-SLAM3 for visual/visual-inertial, or Cartographer/RTAB-Map for lidar — separate the front-end (feature extraction, data association) from the back-end (graph optimization) so each can be improved independently, and implement loop closure detection to correct accumulated drift.

4. Implement motion planning using a layered approach: global planning (RRT*, PRM, A* on a navigation grid) for finding collision-free paths, local planning (DWA, TEB, MPC) for real-time obstacle avoidance and trajectory smoothing, and a recovery behavior layer for handling stuck situations — configure costmaps with proper inflation radii and use the Nav2 framework's BT-based navigation for composable, recoverable behaviors.

5. Solve inverse kinematics using analytical solutions for common kinematic chains (6-DOF serial arms with spherical wrists) where closed-form solutions exist, and numerical solvers (KDL, TRAC-IK, or Jacobian-based iterative methods) for general cases — always validate solutions against joint limits, check for singularities (low manipulability index), and handle multiple valid solutions by selecting based on proximity to current configuration or optimization criteria.

6. Generate smooth trajectories by interpolating between waypoints using trapezoidal velocity profiles for simple moves, or cubic/quintic spline interpolation for smooth acceleration — respect joint velocity, acceleration, and jerk limits at all times, use time-optimal trajectory scaling (TOPP-RA) to minimize cycle time while staying within dynamic limits, and implement real-time trajectory modification for dynamic environments.

7. Design PID controllers with proper gain tuning (Ziegler-Nichols for initial values, then manual refinement), anti-windup on the integral term, derivative filtering to reduce noise sensitivity, and feed-forward terms for known dynamics — for higher performance, implement Model Predictive Control (MPC) that optimizes over a receding horizon, handles constraints explicitly, and accounts for system dynamics and coupling between joints.

8. Build hardware abstraction layers (HAL) using the ros2_control framework: define hardware interfaces (SystemInterface, ActuatorInterface, SensorInterface) that expose state and command interfaces, and use controllers (JointTrajectoryController, DiffDriveController) from ros2_controllers — this decouples your application logic from specific motor drivers, encoders, and communication buses so hardware can be swapped without rewriting control code.

9. Meet real-time constraints by running control loops on a PREEMPT_RT patched kernel, using memory-locked processes (mlockall), pre-allocated memory pools (avoid malloc in the control loop), and real-time-safe inter-thread communication (lock-free queues) — measure and monitor worst-case execution time (WCET) and jitter, and separate real-time (control, safety) from non-real-time (planning, perception) processing into distinct execution contexts.

10. Test in simulation before deploying to hardware using Gazebo (Classic or Ignition/Gz) for physics-based simulation with ROS2 integration, or NVIDIA Isaac Sim for photorealistic rendering and GPU-accelerated physics — maintain identical launch files and configuration for sim and real (differing only in hardware interface plugins), implement a sim-to-real transfer checklist, and run regression tests in CI using headless simulation.

11. Implement safety systems as the highest-priority layer: hardware e-stop circuits that cut power independent of software, software safety nodes that monitor joint limits, velocities, forces/torques, and workspace boundaries at the control loop rate, and collision avoidance using distance queries against the planning scene — safety nodes must be independent of the main application, use watchdog timers, and fail to a safe state (controlled stop or power cut) on any anomaly.

12. Configure communication middleware appropriately: use DDS (the default ROS2 middleware) with QoS profiles matched to your data patterns — reliable/transient-local for configuration and map data, best-effort/volatile for high-frequency sensor streams, and deadline/liveliness policies for health monitoring — for hardware communication, use EtherCAT for deterministic real-time motor control, or CAN bus (SocketCAN) for distributed sensor/actuator networks with lower bandwidth requirements.

13. Implement coordinate frame management rigorously using the TF2 library — publish transforms between all reference frames (base_link, odom, map, sensor frames, tool frames) at consistent rates, use static transforms for fixed geometry, and always transform data into the correct frame before processing — maintain a URDF/Xacro model as the single source of truth for robot geometry and use it for visualization, planning, and collision checking.
