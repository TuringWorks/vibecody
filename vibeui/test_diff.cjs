const assert = require('assert');

function assembleFinal(originalLines, _modifiedLines, hunks, _allDiffed) {
 const acceptedInserts = new Map();
 const deletedOrigLines = new Set();
 for (const hunk of hunks) {
 if (!hunk.accepted) continue;
 let afterOrigLine = 0;
 const insertBuffer = [];
 for (const line of hunk.lines) {
 if (line.kind === "equal") {
 if (insertBuffer.length > 0 && line.origLine != null) {
 const key = line.origLine;
 acceptedInserts.set(key, [...(acceptedInserts.get(key) ?? []), ...insertBuffer]);
 insertBuffer.length = 0;
 }
 afterOrigLine = line.origLine ?? afterOrigLine;
 } else if (line.kind === "delete") {
 if (line.origLine != null) deletedOrigLines.add(line.origLine);
 } else if (line.kind === "insert") {
 insertBuffer.push(line.text);
 }
 }
 if (insertBuffer.length > 0) {
 const key = afterOrigLine + 1;
 acceptedInserts.set(key, [...(acceptedInserts.get(key) ?? []), ...insertBuffer]);
 }
 }
 const result = [];
 for (let i = 1; i <= originalLines.length; i++) {
 const before = acceptedInserts.get(i);
 if (before) result.push(...before);
 if (!deletedOrigLines.has(i)) {
 result.push(originalLines[i - 1]);
 }
 }
 const trailing = acceptedInserts.get(originalLines.length + 1);
 if (trailing) result.push(...trailing);
 if (deletedOrigLines.size === 0 && acceptedInserts.size === 0) {
 return originalLines.join("\n");
 }
 return result.join("\n");
}

// Test with 150_000 lines
const orig = ["hello"];
const mod = new Array(150000).fill("test");
const hunks = [{
  accepted: true,
  lines: [
    { kind: 'delete', origLine: 1, text: 'hello' },
    ...mod.map(text => ({ kind: 'insert', text }))
  ]
}];

try {
  assembleFinal(orig, mod, hunks, []);
  console.log("Success");
} catch (e) {
  console.error("Crash:", e.message);
}
