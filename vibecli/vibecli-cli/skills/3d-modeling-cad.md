---
triggers: ["3D modeling", "CAD", "AutoCAD", "SolidWorks", "Blender", "Fusion 360", "3D printing", "CAD/CAM"]
tools_allowed: ["read_file", "write_file", "bash"]
category: design
---

# 3D Modeling & CAD

When working with 3D modeling and CAD systems:

1. Choose parametric modeling (SolidWorks, Fusion 360) when designs need to be modified by changing dimensions and constraints; use direct modeling for organic shapes or quick concept geometry where design intent is less important than speed.
2. Understand the difference between mesh representations (triangulated surfaces for visualization and 3D printing) and NURBS (mathematically precise curves and surfaces for engineering); convert between them only when necessary as fidelity loss occurs going from NURBS to mesh.
3. Select file formats based on purpose: STEP and IGES for cross-platform CAD exchange, STL and 3MF for 3D printing, OBJ and FBX for rendering and animation, DWG and DXF for 2D drafting, and native formats when staying within one tool.
4. Follow AutoCAD drafting best practices: use layers to organize geometry by type, set up proper dimension styles and text standards, work in model space at 1:1 scale and configure viewports in paper space, and maintain consistent naming conventions for blocks and layers.
5. Build SolidWorks assemblies with a clear hierarchy: use a skeleton or layout sketch to drive top-level dimensions, apply mates systematically (coincident, concentric, distance), minimize in-context references to prevent circular dependencies, and test with interference detection before release.
6. Adopt a Blender modeling workflow that starts with blocking out shapes using primitives, then refines with subdivision surface modifier, uses loop cuts and edge flow to control topology, and applies modifiers non-destructively before finalizing; keep quad-dominant meshes for clean subdivision.
7. Design for 3D printing by maintaining minimum wall thickness (typically 0.8-1.2mm for FDM), minimizing overhangs beyond 45 degrees or adding support structures, orienting parts to reduce support material, accounting for shrinkage and tolerances, and using chamfers instead of fillets on bottom edges.
8. Generate CNC/CAM toolpaths by first selecting appropriate stock material and workholding, then defining roughing passes with step-down and step-over, followed by finishing passes with finer step-over; verify with simulation, account for tool deflection, and post-process for the specific machine controller.
9. Integrate with BIM workflows by exporting IFC files for interoperability, maintaining proper coordinate systems and origins, embedding metadata (material, cost, manufacturer) in model properties, and using reference planes and levels consistent with the architectural model.
10. Manage version control for CAD files by using PDM/PLM systems (SolidWorks PDM, Autodesk Vault) rather than generic Git for binary files, maintaining revision histories with meaningful descriptions, implementing check-in/check-out workflows to prevent conflicts, and archiving released versions as STEP alongside native files.
11. Set up rendering and visualization with proper HDRI lighting environments, physically-based materials (PBR), appropriate camera focal lengths (50-85mm for product shots), and use ray tracing for final output while viewport shading suffices for design reviews; batch render multiple angles for documentation.
12. Apply tolerancing and GD&T (Geometric Dimensioning and Tolerancing) by specifying datums that reflect assembly and manufacturing intent, using feature control frames with appropriate geometric tolerances (flatness, parallelism, position), applying maximum material condition where it benefits assembly, and documenting tolerance stack-ups for critical dimensions.
