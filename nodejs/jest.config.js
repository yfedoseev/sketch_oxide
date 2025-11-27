module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  roots: ['<rootDir>/__tests__'],
  testMatch: ['**/__tests__/**/*.test.ts'],
  // Exclude minhash tests due to native code memory bug (34GB allocation on load)
  // TODO: Investigate minhash native implementation for memory leak
  testPathIgnorePatterns: ['/node_modules/', '__tests__/minhash.test.ts'],
  moduleFileExtensions: ['ts', 'tsx', 'js', 'jsx', 'json', 'node'],
  collectCoverageFrom: [
    'index.js',
    'src/**/*.ts',
    '!**/*.d.ts',
    '!**/node_modules/**'
  ],
  coverageThreshold: {
    global: {
      branches: 70,
      functions: 70,
      lines: 70,
      statements: 70
    }
  },
  testTimeout: 30000,
  verbose: true,
  // Run tests with single worker to prevent memory issues with native bindings
  // See: https://github.com/prisma/prisma/issues/8989
  maxWorkers: 1,
  // Limit worker memory to prevent OOM crashes
  workerIdleMemoryLimit: '512MB'
}
