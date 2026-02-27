export function setupKeybindings({
  onToggleHidden,
  onNavigateBack,
  onNavigateForward,
  onNewFolder,
  onFocusAddressBar
}) {
  const onKeyDown = (event) => {
    const target = event.target;
    if (
      target &&
      (target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable)
    ) {
      return;
    }

    if (event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey) {
      if (event.key.toLowerCase() === 'h') {
        event.preventDefault();
        onToggleHidden?.();
      }
    }

    if (event.ctrlKey && !event.altKey && !event.metaKey && event.shiftKey) {
      if (event.key.toLowerCase() === 'n') {
        event.preventDefault();
        onNewFolder?.();
        return;
      }
    }

    if (event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey) {
      if (event.key === 'ArrowLeft') {
        event.preventDefault();
        onNavigateBack?.();
        return;
      }
      if (event.key === 'ArrowRight') {
        event.preventDefault();
        onNavigateForward?.();
        return;
      }
    }

    if (!event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey) {
      if (event.key === 'F6') {
        event.preventDefault();
        onFocusAddressBar?.();
        return;
      }
      if (event.key === 'Backspace') {
        event.preventDefault();
        onNavigateBack?.();
      }
    }
  };

  window.addEventListener('keydown', onKeyDown);
  return () => window.removeEventListener('keydown', onKeyDown);
}
