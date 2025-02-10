module.exports = {
  preset: 'jest-puppeteer',
  transform: {
    '^.+\\.tsx?$': 'ts-jest',
  },
  testEnvironment: 'jest-environment-puppeteer',
  testMatch: ['**/*.test.ts', '**/*.test.js', '**/*.spec.ts', '**/*.spec.js', '**/*.e2e.ts', '**/*.e2e.js'],
  moduleFileExtensions: ['ts', 'js'],
  roots: ['<rootDir>/src', '<rootDir>/test'],
  testTimeout: 300000,
};
