import assert from "node:assert/strict";
import test from "node:test";

import { renderDocsReport } from "./docs-report.mjs";

test("renderDocsReport marks the output as derived and includes risky docs", () => {
  const report = renderDocsReport({
    generatedAt: "2026-04-08T00:00:00Z",
    docs: [
      {
        path: ".omx/ledger/current-status.md",
        score: 92,
        orphan: false,
        broken: [],
        missingRefs: [],
        staleDays: 0,
        updated: 0,
      },
      {
        path: "docs/example.md",
        score: 60,
        orphan: true,
        broken: ["docs/missing.md"],
        missingRefs: ["src/missing.ts"],
        staleDays: 10,
        updated: 0,
      },
    ],
  });

  assert.match(report, /derived report/i);
  assert.match(report, /canonical planning\/bootstrap truth lives in the OMX ledger surfaces/i);
  assert.match(report, /\*\*docs\/example\.md\*\* — score 60/);
  assert.match(report, /Near-duplicate clusters/);
});
