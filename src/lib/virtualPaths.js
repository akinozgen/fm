export const FM_SCHEME = 'fm://';
export const FM_WELCOME = 'fm://welcome';
export const FM_HOME_ALIAS = 'fm://home';
export const FM_TRASH = 'fm://trash';
export const FM_DRAFT_PREFIX = 'fm://draft/';

export function normalizePath(path) {
  return (path || '').replace(/\\/g, '/');
}

export function canonicalizePath(path) {
  const normalized = normalizePath(path);
  if (normalized === FM_HOME_ALIAS) return FM_WELCOME;
  return normalized;
}

export function isVirtualPath(path) {
  return canonicalizePath(path).startsWith(FM_SCHEME);
}

export function isWelcomePath(path) {
  const normalized = canonicalizePath(path);
  return normalized === FM_WELCOME;
}

export function isTrashPath(path) {
  return canonicalizePath(path) === FM_TRASH;
}

export function isDraftPath(path) {
  return canonicalizePath(path).startsWith(FM_DRAFT_PREFIX);
}

export function canBrowsePath(path) {
  return !isWelcomePath(path) && !isTrashPath(path);
}

export function createDraftPath(kind = 'item') {
  return `${FM_DRAFT_PREFIX}${kind}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function getVirtualPathLabel(path) {
  const normalized = canonicalizePath(path);
  if (normalized === FM_WELCOME) return 'Home';
  if (normalized === FM_TRASH) return 'Trash';
  return '';
}
