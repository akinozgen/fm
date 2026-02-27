export function resolveFileContextTarget(event, entries) {
  const row = event.target?.closest?.('.tile[data-path], .list-row[data-path]');
  if (!row) {
    return { kind: 'empty', path: null };
  }

  const path = row.getAttribute('data-path') || '';
  const entry = entries.find((item) => item.path === path);
  if (!entry) {
    return { kind: 'empty', path: null };
  }

  return {
    kind: entry.is_dir ? 'dir' : 'file',
    path: entry.path
  };
}
