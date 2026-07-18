import { createComposite } from "./createComposite";

export const FormattersComposite = createComposite([
  { id: "regex", label: "Regex", importFn: () => import("../RegexPanel"), exportName: "RegexPanel" },
  { id: "jwt", label: "JWT", importFn: () => import("../JwtPanel"), exportName: "JwtPanel" },
  { id: "jsontools", label: "JSON", importFn: () => import("../JsonToolsPanel"), exportName: "JsonToolsPanel" },
  { id: "cron", label: "Cron", importFn: () => import("../CronPanel"), exportName: "CronPanel" },
  { id: "csv", label: "CSV", importFn: () => import("../CsvPanel"), exportName: "CsvPanel" },
  { id: "cidr", label: "CIDR", importFn: () => import("../CidrPanel"), exportName: "CidrPanel" },
  { id: "datagen", label: "Data Gen", importFn: () => import("../DataGenPanel"), exportName: "DataGenPanel" },
  { id: "utils", label: "Utils", importFn: () => import("../UtilitiesPanel"), exportName: "UtilitiesPanel" },
]);
