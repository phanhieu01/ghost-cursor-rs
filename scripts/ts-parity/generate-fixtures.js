const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

function xmur3(str) {
  let h = 1779033703 ^ str.length;
  for (let i = 0; i < str.length; i++) {
    h = Math.imul(h ^ str.charCodeAt(i), 3432918353);
    h = (h << 13) | (h >>> 19);
  }
  return function () {
    h = Math.imul(h ^ (h >>> 16), 2246822507);
    h = Math.imul(h ^ (h >>> 13), 3266489909);
    return (h ^= h >>> 16) >>> 0;
  };
}

function mulberry32(a) {
  return function () {
    let t = (a += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function rngFromSeed(seed) {
  const hash = xmur3(seed)();
  return mulberry32(hash);
}

function withSeed(seed, fn) {
  const oldRandom = Math.random;
  Math.random = rngFromSeed(seed);
  try {
    return fn();
  } finally {
    Math.random = oldRandom;
  }
}

function ensureDir(dirPath) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }
}

function main() {
  const repoRoot = path.resolve(__dirname, '..', '..');
  const upstreamRoot = path.join(repoRoot, 'vendor', 'ghost-cursor-ts');
  const fixtureDir = path.join(repoRoot, 'fixtures', 'ts-parity');

  const upstreamCommit = execSync('git rev-parse HEAD', {
    cwd: upstreamRoot,
    stdio: ['ignore', 'pipe', 'ignore'],
    encoding: 'utf8',
  }).trim();

  const upstreamPkg = JSON.parse(
    fs.readFileSync(path.join(upstreamRoot, 'package.json'), 'utf8')
  );

  const spoofModulePath = path.join(upstreamRoot, 'lib', 'spoof.js');
  if (!fs.existsSync(spoofModulePath)) {
    throw new Error(
      'Upstream build not found at vendor/ghost-cursor-ts/lib/spoof.js. Run: npx tsc -p tsconfig.build.json in vendor/ghost-cursor-ts'
    );
  }

  const { path: tsPath } = require(spoofModulePath);
  if (typeof tsPath !== 'function') {
    throw new Error('Could not find exported path() function in upstream spoof.js');
  }

  const cases = [
    {
      case_id: 'point_short_spread0',
      seed: 'gc-ts-001',
      params: {
        start: { x: 100, y: 100 },
        target: { kind: 'point', point: { x: 320, y: 280 } },
        options: { spread_override: 0, move_speed: 0.6, use_timestamps: false },
      },
    },
    {
      case_id: 'point_long_spread0',
      seed: 'gc-ts-002',
      params: {
        start: { x: 10, y: 20 },
        target: { kind: 'point', point: { x: 1200, y: 900 } },
        options: { spread_override: 0, move_speed: 0.9, use_timestamps: false },
      },
    },
    {
      case_id: 'point_timestamp_spread0',
      seed: 'gc-ts-003',
      params: {
        start: { x: 50, y: 75 },
        target: { kind: 'point', point: { x: 640, y: 480 } },
        options: { spread_override: 0, move_speed: 0.75, use_timestamps: true },
      },
    },
    {
      case_id: 'box_target_default_spread',
      seed: 'gc-ts-004',
      params: {
        start: { x: 200, y: 250 },
        target: {
          kind: 'box',
          box: { x: 700, y: 420, width: 180, height: 120 },
        },
        options: { move_speed: 0.7, use_timestamps: false },
      },
    },
    {
      case_id: 'point_spread_random',
      seed: 'gc-ts-005',
      params: {
        start: { x: 150, y: 150 },
        target: { kind: 'point', point: { x: 900, y: 500 } },
        options: { move_speed: 0.65, use_timestamps: false },
      },
    },
  ];

  ensureDir(fixtureDir);

  const index = {
    generated_at: new Date().toISOString(),
    generator: 'scripts/ts-parity/generate-fixtures.js',
    upstream: {
      repo: 'Xetera/ghost-cursor',
      commit: upstreamCommit,
      version: upstreamPkg.version,
    },
    cases: [],
  };

  for (const testCase of cases) {
    const end =
      testCase.params.target.kind === 'point'
        ? testCase.params.target.point
        : testCase.params.target.box;

    const tsOptions = {
      spreadOverride: testCase.params.options.spread_override,
      moveSpeed: testCase.params.options.move_speed,
      useTimestamps: testCase.params.options.use_timestamps,
    };

    const route = withSeed(testCase.seed, () =>
      tsPath(testCase.params.start, end, tsOptions)
    );

    const fixture = {
      metadata: {
        case_id: testCase.case_id,
        seed: testCase.seed,
        upstream_commit: upstreamCommit,
        upstream_version: upstreamPkg.version,
      },
      params: testCase.params,
      data: route,
    };

    const outPath = path.join(fixtureDir, `${testCase.case_id}.json`);
    fs.writeFileSync(outPath, JSON.stringify(fixture, null, 2), 'utf8');

    index.cases.push({
      case_id: testCase.case_id,
      file: `${testCase.case_id}.json`,
      points: route.length,
      has_timestamps:
        route.length > 0 && Object.prototype.hasOwnProperty.call(route[0], 'timestamp'),
    });
  }

  fs.writeFileSync(path.join(fixtureDir, 'index.json'), JSON.stringify(index, null, 2), 'utf8');
  console.log(`Generated ${index.cases.length} fixtures in ${fixtureDir}`);
}

main();
