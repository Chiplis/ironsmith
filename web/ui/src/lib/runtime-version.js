/* global __IRONSMITH_RUNTIME_VERSION__ */
// The production bundler hashes the executable modules and facade together.
// Recovery must not silently replay an accepted transcript with different rules.
export const RUNTIME_VERSION = typeof __IRONSMITH_RUNTIME_VERSION__ === 'string'
  ? __IRONSMITH_RUNTIME_VERSION__ : 'development-unversioned';
export function assertRuntimeVersion(match) {
  if (match?.runtimeVersion && match.runtimeVersion !== RUNTIME_VERSION) {
    throw new Error('This match uses a different engine version. Open the matching application build to resume it.');
  }
}
