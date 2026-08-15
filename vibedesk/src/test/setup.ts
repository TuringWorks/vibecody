import "@testing-library/jest-dom/vitest";
import { configure } from "@testing-library/react";

// Match the file-level timeout in vitest.config.ts; the default `waitFor`
// budget (1000ms) flakes on otherwise-correct tests under CI load.
configure({ asyncUtilTimeout: 5000 });
