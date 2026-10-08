import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const paths = ['api/openapi.json', 'web/src/api/schema.d.ts'];
const before = paths.map((path) => readFileSync(path));
const result = spawnSync('mise', ['run', 'generate'], { stdio: 'inherit' });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
const changed = paths.filter(
  (path, i) => !before[i].equals(readFileSync(path)),
);
if (changed.length) {
  console.error(
    `Contract artifacts are stale: ${changed.join(', ')}. Commit the regenerated files.`,
  );
  process.exit(1);
}
