#!/usr/bin/env node
/**
 * Post-build script to fix TypeScript type aliases
 *
 * NAPI-RS generates type-only exports for uppercase aliases:
 *   export type DDSketch = DdSketch
 *
 * This script converts them to runtime exports:
 *   export { DdSketch as DDSketch }
 *
 * This allows tests to import and use these as constructors.
 */

const fs = require('fs');
const path = require('path');

const indexDtsPath = path.join(__dirname, 'index.d.ts');

try {
  let content = fs.readFileSync(indexDtsPath, 'utf8');

  const replacements = [
    ['export type DDSketch = DdSketch', 'export { DdSketch as DDSketch }'],
    ['export type SALSA = Salsa', 'export { Salsa as SALSA }'],
    ['export type RatelessIBLT = RatelessIblt', 'export { RatelessIblt as RatelessIBLT }'],
    ['export type GRF = Grf', 'export { Grf as GRF }'],
  ];

  let changed = false;

  for (const [oldExport, newExport] of replacements) {
    if (content.includes(oldExport)) {
      content = content.replace(oldExport, newExport);
      console.log(`✓ Fixed: ${oldExport.split(' ')[2]} alias`);
      changed = true;
    }
  }

  if (changed) {
    fs.writeFileSync(indexDtsPath, content, 'utf8');
    console.log('✓ Successfully fixed type aliases in index.d.ts');
  } else {
    console.log('ℹ No type aliases needed fixing (already in correct format)');
  }

} catch (error) {
  console.error('✗ Error fixing type aliases:', error.message);
  process.exit(1);
}
