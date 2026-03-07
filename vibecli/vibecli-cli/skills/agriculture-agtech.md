---
triggers: ["agriculture", "AgTech", "precision farming", "crop monitoring", "farm management", "irrigation", "yield prediction", "livestock", "soil sensor", "drone agriculture", "FMIS"]
tools_allowed: ["read_file", "write_file", "bash"]
category: agriculture
---

# Agriculture & AgTech Systems

When working with agriculture technology and farm management systems:

1. Design farm management information systems (FMIS) with a spatial-first data model where every operation (planting, spraying, harvesting) is linked to georeferenced field boundaries (GeoJSON/Shapefile); support multi-farm, multi-season data organization, track input costs and yields per field and crop for profitability analysis, and provide offline-capable mobile interfaces for in-field data entry where connectivity is unreliable.

2. Build precision agriculture data pipelines that ingest satellite imagery (Sentinel-2, Landsat) and drone-captured multispectral imagery, orthorectify and stitch raw images into field-level orthomosaics, compute vegetation indices (NDVI, NDRE, SAVI) per pixel, and aggregate zonal statistics per management zone; schedule processing triggered by new image availability and store results as Cloud-Optimized GeoTIFFs (COGs) for efficient tile-based retrieval.

3. Implement crop monitoring ML models that analyze time-series NDVI profiles to detect stress indicators (pest damage, nutrient deficiency, water stress) by comparing current-season curves against historical baselines and regional benchmarks; train classification models on labeled ground-truth data to distinguish stress types, generate field-level alert maps, and retrain models seasonally as new labeled data becomes available.

4. Build irrigation automation systems that integrate soil moisture sensor readings, evapotranspiration calculations (Penman-Monteith FAO-56), weather forecast data, and crop growth stage coefficients to compute daily irrigation prescriptions per zone; interface with irrigation controllers (via GPIO, Modbus, or manufacturer APIs) to actuate valves, implement safety overrides (freeze protection, rain delay), and log actual water application for compliance and efficiency tracking.

5. Develop yield prediction algorithms that combine remote sensing features (peak NDVI, canopy cover trajectory), weather variables (growing degree days, cumulative precipitation, heat stress days), soil characteristics (texture, organic matter, drainage class), and historical yield records using ensemble models (gradient boosted trees, neural networks); generate field-level and sub-field-level yield estimates at multiple points during the growing season with confidence intervals.

6. Implement livestock tracking and health monitoring by ingesting data from GPS ear tags, accelerometers, and rumination sensors via LoRaWAN or cellular IoT gateways; build activity profiles to detect anomalies (lameness from gait changes, illness from reduced feed intake, estrus from increased activity), trigger alerts to farm managers, and maintain per-animal health records including vaccinations, treatments, and weight gain trajectories.

7. Integrate soil sensor IoT networks by deploying multi-depth sensor probes (moisture, temperature, EC, pH) across management zones, collecting readings via LoRaWAN or Zigbee mesh networks to a gateway, normalizing raw sensor values using manufacturer calibration curves, storing time-series data with sensor metadata (depth, GPS coordinates, installation date), and implementing sensor health monitoring (battery level, signal strength, reading plausibility checks).

8. Build weather data integration services that aggregate observations from on-farm weather stations (Davis, METOS), public APIs (OpenWeatherMap, NOAA), and gridded forecast models (GFS, ECMWF); interpolate station data to field-level using inverse distance weighting or kriging, compute agronomic indices (growing degree days, chill hours, frost risk, spray windows based on wind/rain thresholds), and cache forecasts with appropriate refresh intervals.

9. Implement supply chain traceability (farm-to-fork) by assigning unique lot identifiers at harvest that link to field-level production records (inputs applied, harvest date, variety, certifications); propagate lot IDs through post-harvest handling (storage, processing, transport) using GS1 standards or blockchain-based provenance ledgers, enable rapid recall trace-back from retail to field in under 4 hours, and generate consumer-facing QR-code traceability pages.

10. Build equipment telematics integration by ingesting machine data (tractors, combines, sprayers) via ISOBUS/CAN bus adapters or manufacturer cloud APIs (John Deere Operations Center, CNH), capturing as-applied maps (seed population, chemical rate, fertilizer rate per GPS coordinate), calculating field efficiency metrics (productive time vs. idle/transport), tracking maintenance schedules based on engine hours, and generating fuel consumption reports.

11. Generate variable rate application (VRA) maps by combining yield maps, soil sampling results, and remote sensing vegetation indices into management zone delineations using clustering algorithms (k-means on multi-layer raster stacks); prescribe input rates (seed, fertilizer, lime) per zone based on agronomic recommendations and economic optimization, export prescriptions in ISOXML or Shapefile format compatible with precision application equipment controllers.

12. Automate compliance and subsidy reporting by tracking field-level activities (crop rotations, cover cropping, conservation practices, input applications) against regulatory requirements (EPA, USDA conservation programs, EU CAP greening rules) and organic certification standards; generate pre-filled compliance reports, flag potential violations before submission deadlines, and maintain an audit trail of all field operations with timestamp and GPS evidence.

13. Design data architectures that handle the spatial and temporal nature of agricultural data: use PostGIS for geospatial queries on field boundaries and sensor locations, time-series databases for high-frequency sensor and weather data, object storage for imagery and orthomosaics, and implement a spatial tiling scheme for efficient map rendering; ensure all data carries CRS (coordinate reference system) metadata and support reprojection for cross-dataset analysis.

14. Implement multi-tenant farm platforms with role-based access control that supports farm owner, agronomist, equipment operator, and data viewer roles; allow agronomists to access multiple farm accounts, support data sharing between cooperating farms for regional benchmarking while anonymizing individual farm identities, and provide API access for third-party integrations (grain elevators, input suppliers, crop insurance providers) with OAuth 2.0 scoped tokens.
