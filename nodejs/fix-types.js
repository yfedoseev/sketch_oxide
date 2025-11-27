#!/usr/bin/env node
/**
 * Post-build script to fix TypeScript type aliases and JavaScript exports
 *
 * NAPI-RS generates type-only exports for uppercase aliases:
 *   export type DDSketch = DdSketch
 *
 * This script converts them to runtime exports in index.d.ts:
 *   export { DdSketch as DDSketch }
 *
 * It also ensures uppercase aliases are exported in index.js:
 *   module.exports.DDSketch = DdSketch
 *
 * This allows tests to import and use these as constructors.
 */

const fs = require('fs');
const path = require('path');

const indexDtsPath = path.join(__dirname, 'index.d.ts');
const indexJsPath = path.join(__dirname, 'index.js');

try {
  // Fix TypeScript definitions
  let dtsContent = fs.readFileSync(indexDtsPath, 'utf8');

  const dtsReplacements = [
    ['export type DDSketch = DdSketch', 'export { DdSketch as DDSketch }'],
    ['export type SALSA = Salsa', 'export { Salsa as SALSA }'],
    ['export type RatelessIBLT = RatelessIblt', 'export { RatelessIblt as RatelessIBLT }'],
    ['export type GRF = Grf', 'export { Grf as GRF }'],
  ];

  let dtsChanged = false;

  for (const [oldExport, newExport] of dtsReplacements) {
    if (dtsContent.includes(oldExport)) {
      dtsContent = dtsContent.replace(oldExport, newExport);
      console.log(`✓ Fixed TypeScript: ${oldExport.split(' ')[2]} alias`);
      dtsChanged = true;
    }
  }

  if (dtsChanged) {
    fs.writeFileSync(indexDtsPath, dtsContent, 'utf8');
    console.log('✓ Successfully fixed type aliases in index.d.ts');
  }

  // Fix JavaScript runtime exports
  let jsContent = fs.readFileSync(indexJsPath, 'utf8');

  const jsReplacements = [
    ['module.exports.DdSketch = DdSketch', 'module.exports.DdSketch = DdSketch\nmodule.exports.DDSketch = DdSketch'],
    ['module.exports.Salsa = Salsa', 'module.exports.Salsa = Salsa\nmodule.exports.SALSA = Salsa'],
    ['module.exports.RatelessIblt = RatelessIblt', 'module.exports.RatelessIblt = RatelessIblt\nmodule.exports.RatelessIBLT = RatelessIblt'],
    ['module.exports.Grf = Grf', 'module.exports.Grf = Grf\nmodule.exports.GRF = Grf'],
  ];

  let jsChanged = false;

  for (const [searchText, replacement] of jsReplacements) {
    if (jsContent.includes(searchText) && !jsContent.includes(replacement)) {
      jsContent = jsContent.replace(searchText, replacement);
      let alias = '';
      if (searchText.includes('DdSketch')) alias = 'DDSketch';
      else if (searchText.includes('Salsa')) alias = 'SALSA';
      else if (searchText.includes('RatelessIblt')) alias = 'RatelessIBLT';
      else if (searchText.includes('Grf')) alias = 'GRF';
      console.log(`✓ Fixed JavaScript: ${alias} export`);
      jsChanged = true;
    }
  }

  if (jsChanged) {
    fs.writeFileSync(indexJsPath, jsContent, 'utf8');
    console.log('✓ Successfully fixed exports in index.js');
  }

  if (!dtsChanged && !jsChanged) {
    console.log('ℹ No aliases needed fixing (already in correct format)');
  }

} catch (error) {
  console.error('✗ Error fixing aliases:', error.message);
  process.exit(1);
}
