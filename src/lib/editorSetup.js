import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLineGutter,
  highlightSpecialChars,
  drawSelection,
  dropCursor,
  rectangularSelection,
  crosshairCursor,
  highlightActiveLine,
} from '@codemirror/view';
import { EditorState, Compartment } from '@codemirror/state';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { search, highlightSelectionMatches, findNext, findPrevious } from '@codemirror/search';
import {
  syntaxHighlighting,
  defaultHighlightStyle,
  bracketMatching,
  indentOnInput,
  foldGutter,
  foldKeymap,
} from '@codemirror/language';
import { LanguageDescription } from '@codemirror/language';
import { languages } from '@codemirror/language-data';
import {
  autocompletion,
  completionKeymap,
  closeBrackets,
  closeBracketsKeymap,
} from '@codemirror/autocomplete';
import { lintKeymap } from '@codemirror/lint';

// ── Compartments (allow live reconfiguration without full editor rebuild) ──────
export const wrapCompartment = new Compartment();
export const languageCompartment = new Compartment();

// ── Custom light theme matching the app's design system ───────────────────────
export const editorLightTheme = EditorView.theme(
  {
    '&': { color: '#1c1c1e', backgroundColor: '#ffffff', height: '100%' },
    '&.cm-focused': { outline: 'none' },
    '.cm-scroller': {
      fontFamily: "'SF Mono','Fira Code','Cascadia Code',Consolas,'Courier New',monospace",
      fontSize: '13px',
      lineHeight: '1.65',
      overflow: 'auto',
    },
    '.cm-content': { caretColor: '#0a84ff', padding: '6px 0' },
    '.cm-gutters': {
      backgroundColor: '#f5f5f7',
      borderRight: '1px solid #d2d2d7',
      color: '#9e9ea0',
      userSelect: 'none',
      minWidth: '44px',
    },
    '.cm-lineNumbers .cm-gutterElement': { paddingLeft: '8px', paddingRight: '12px' },
    '.cm-foldGutter .cm-gutterElement': { paddingLeft: '2px', paddingRight: '4px' },
    '.cm-activeLineGutter': { backgroundColor: '#ebebf0', color: '#3a3a3c' },
    '.cm-activeLine': { backgroundColor: '#f5f5fa' },
    '&.cm-focused .cm-selectionBackground': { backgroundColor: '#b3d7ff' },
    '.cm-selectionBackground': { backgroundColor: '#d4e7ff' },
    '::selection': { backgroundColor: '#b3d7ff' },
    '.cm-cursor,.cm-dropCursor': { borderLeftColor: '#0a84ff', borderLeftWidth: '2px' },
    '.cm-matchingBracket': { backgroundColor: '#ffd60a33', outline: '1px solid #ffd60a88' },
    '.cm-nonmatchingBracket': { backgroundColor: '#ff375f33', outline: '1px solid #ff375f88' },
    '.cm-searchMatch': {
      backgroundColor: '#ffd60a44',
      outline: '1px solid #ffd60a',
      borderRadius: '2px',
    },
    '.cm-searchMatch.cm-searchMatch-selected': { backgroundColor: '#ffd60a' },
    '.cm-selectionMatch': { backgroundColor: '#0a84ff18' },
    '.cm-panels': { backgroundColor: '#f5f5f7', color: '#1c1c1e' },
    '.cm-panels.cm-panels-top': { borderBottom: '1px solid #d2d2d7' },
    '.cm-panels.cm-panels-bottom': { borderTop: '1px solid #d2d2d7' },
    '.cm-textfield': {
      borderRadius: '5px',
      border: '1px solid #d2d2d7',
      backgroundColor: '#ffffff',
      padding: '3px 7px',
      fontSize: '12px',
      fontFamily: 'inherit',
      outline: 'none',
      color: '#1c1c1e',
    },
    '.cm-textfield:focus': { borderColor: '#0a84ff', boxShadow: '0 0 0 2px #0a84ff33' },
    '.cm-button': {
      borderRadius: '5px',
      border: '1px solid #d2d2d7',
      backgroundColor: '#ffffff',
      color: '#1c1c1e',
      padding: '3px 10px',
      fontSize: '12px',
      fontFamily: 'inherit',
      cursor: 'default',
    },
    '.cm-button:hover': { backgroundColor: '#f0f0f5' },
    '.cm-button:focus': { borderColor: '#0a84ff', outline: 'none' },
    '.cm-panel label': { fontSize: '12px', fontFamily: 'inherit', display: 'inline-flex', alignItems: 'center', gap: '4px' },
    '.cm-panel.cm-search': {
      padding: '6px 10px',
      display: 'flex',
      flexWrap: 'wrap',
      gap: '6px',
      alignItems: 'center',
    },
    '.cm-tooltip': {
      backgroundColor: '#ffffff',
      border: '1px solid #d2d2d7',
      borderRadius: '6px',
      boxShadow: '0 4px 12px rgba(0,0,0,0.12)',
    },
    '.cm-tooltip.cm-tooltip-autocomplete>ul': {
      fontFamily: "'SF Mono','Fira Code',Consolas,monospace",
      fontSize: '12px',
    },
    '.cm-tooltip.cm-tooltip-autocomplete>ul>li[aria-selected]': {
      backgroundColor: '#0a84ff',
      color: '#ffffff',
    },
    '.cm-completionIcon': { opacity: '0.6', paddingRight: '4px' },
    '.cm-completionLabel': { fontFamily: "'SF Mono','Fira Code',Consolas,monospace" },
    '.cm-diagnosticText': { fontSize: '12px' },
  },
  { dark: false }
);

// ── Extensions for read-only panes (diff views) ───────────────────────────────
export function readonlyExtensions() {
  return [
    lineNumbers(),
    highlightSpecialChars(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    EditorView.editable.of(false),
    EditorState.readOnly.of(true),
    editorLightTheme,
  ];
}

// ── Language detection by filename using the full language-data registry ──────
export async function detectLanguage(filename) {
  const desc = LanguageDescription.matchFilename(languages, filename);
  if (!desc) return { name: 'Plain Text', extension: [] };
  try {
    const ext = await desc.load();
    return { name: desc.name, extension: [ext] };
  } catch {
    return { name: 'Plain Text', extension: [] };
  }
}

// ── Create the main editor instance ──────────────────────────────────────────
export function createEditor({ parent, doc, onUpdate, onSave, onToggleWrap, onOpenSearch, onOpenReplace }) {
  const state = EditorState.create({
    doc,
    extensions: [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightSpecialChars(),
      foldGutter(),
      history(),
      drawSelection(),
      dropCursor(),
      EditorState.allowMultipleSelections.of(true),
      indentOnInput(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      bracketMatching(),
      closeBrackets(),
      rectangularSelection(),
      crosshairCursor(),
      highlightActiveLine(),
      highlightSelectionMatches(),
      search({ top: true }),
      autocompletion(),
      wrapCompartment.of([]),
      languageCompartment.of([]),
      editorLightTheme,
      EditorView.updateListener.of(onUpdate),
      keymap.of([
        { key: 'Mod-s', run() { onSave(); return true; } },
        { key: 'Alt-z', run() { onToggleWrap(); return true; } },
        // Intercept find/replace with our custom panel
        { key: 'Mod-f', run() { onOpenSearch?.(); return true; } },
        { key: 'Mod-h', run() { onOpenReplace?.(); return true; } },
        // Find next/prev still work via Mod-g even with custom panel
        { key: 'Mod-g', run: findNext },
        { key: 'Mod-Shift-g', run: findPrevious },
        ...closeBracketsKeymap,
        ...defaultKeymap,
        ...historyKeymap,
        ...foldKeymap,
        ...completionKeymap,
        ...lintKeymap,
        indentWithTab,
      ]),
    ],
  });

  return new EditorView({ state, parent });
}
