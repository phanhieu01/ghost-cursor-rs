const fs = require('fs');
const path = require('path');

function usage() {
  console.log('Usage: node scripts/ts-parity/compare-fixtures.js <ts-fixture.json> <rust-output.json>');
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function pointDistance(a, b) {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  return Math.hypot(dx, dy);
}

function monotonicTimestamps(points) {
  for (let i = 1; i < points.length; i++) {
    if ((points[i].timestamp ?? 0) < (points[i - 1].timestamp ?? 0)) {
      return false;
    }
  }
  return true;
}

function main() {
  const [,, tsFixturePath, rustOutputPath] = process.argv;
  if (!tsFixturePath || !rustOutputPath) {
    usage();
    process.exit(1);
  }

  const tsFixture = readJson(path.resolve(tsFixturePath));
  const rustOutput = readJson(path.resolve(rustOutputPath));

  const tsData = tsFixture.data;
  const rustData = rustOutput.data ?? rustOutput;

  if (!Array.isArray(tsData) || !Array.isArray(rustData)) {
    throw new Error('Both inputs must contain array field data or be arrays.');
  }

  const report = {
    ts_points: tsData.length,
    rust_points: rustData.length,
    point_count_delta: rustData.length - tsData.length,
    start_distance: tsData.length > 0 && rustData.length > 0 ? pointDistance(tsData[0], rustData[0]) : null,
    end_distance:
      tsData.length > 0 && rustData.length > 0
        ? pointDistance(tsData[tsData.length - 1], rustData[rustData.length - 1])
        : null,
    ts_timestamps_monotonic: monotonicTimestamps(tsData),
    rust_timestamps_monotonic: monotonicTimestamps(rustData),
  };

  console.log(JSON.stringify(report, null, 2));
}

main();
