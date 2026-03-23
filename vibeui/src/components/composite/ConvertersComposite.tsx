import { createComposite } from "./createComposite";

export const ConvertersComposite = createComposite([
  { id: "encoding", label: "Encoding", importFn: () => import("../EncodingPanel"), exportName: "EncodingPanel" },
  { id: "numbers", label: "Numbers", importFn: () => import("../NumberBasePanel"), exportName: "NumberBasePanel" },
  { id: "colorconv", label: "Colors", importFn: () => import("../ColorConverterPanel"), exportName: "ColorConverterPanel" },
  { id: "units", label: "Units", importFn: () => import("../UnitConverterPanel"), exportName: "UnitConverterPanel" },
  { id: "unicode", label: "Unicode", importFn: () => import("../UnicodePanel"), exportName: "UnicodePanel" },
  { id: "timestamp", label: "Timestamp", importFn: () => import("../TimestampPanel"), exportName: "TimestampPanel" },
]);
