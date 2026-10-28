import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { Cell, splitGraphemes, TypingEngine, type RawKey } from './engine/session';
import { firstVisibleLine, lineOf } from './layout';
import { TypingArea } from './TypingArea';

const settings = { fontSize: 20, visibleLines: 2, activeLine: 'top' as const };
const rules = { correction: 'free' as const, backspaceAllowed: true, lenientPunctuation: true };

function setup(text = 'la forêt', lineStarts = [0]) {
  const engine = new TypingEngine(splitGraphemes(text), rules);
  const onKey = vi.fn<(key: RawKey) => void>();
  const view = render(
    <TypingArea
      engine={engine}
      version={0}
      lineStarts={lineStarts}
      settings={settings}
      disabled={false}
      onKey={onKey}
      onColumns={() => {}}
    />,
  );
  const input = screen.getByLabelText('Zone de frappe');
  return { engine, onKey, input, view };
}

function beforeInput(target: Element, inputType: string, data: string | null, isComposing = false) {
  const event = new InputEvent('beforeinput', {
    inputType,
    data,
    isComposing,
    bubbles: true,
    cancelable: true,
  });
  fireEvent(target, event);
  return event;
}

describe('TypingArea — saisie', () => {
  it('transmet les caractères tapés via beforeinput', () => {
    const { input, onKey } = setup();
    const event = beforeInput(input, 'insertText', 'l');
    expect(onKey).toHaveBeenCalledWith({ kind: 'insert', typed: 'l' });
    expect(event.defaultPrevented).toBe(true);
  });

  it('compte une touche morte (^ puis e) comme un seul caractère « ê »', () => {
    const { input, onKey } = setup();
    // Séquence d'un navigateur qui compose : la composition n'est lue qu'à la fin.
    fireEvent.compositionStart(input, { data: '' });
    beforeInput(input, 'insertCompositionText', '^', true);
    beforeInput(input, 'insertCompositionText', 'ê', true);
    fireEvent.compositionEnd(input, { data: 'ê' });
    expect(onKey).toHaveBeenCalledTimes(1);
    expect(onKey).toHaveBeenCalledWith({ kind: 'insert', typed: 'ê' });
  });

  it('ignore une insertion signalée pendant une composition', () => {
    const { input, onKey } = setup();
    beforeInput(input, 'insertText', 'e', true);
    expect(onKey).not.toHaveBeenCalled();
  });

  it('découpe une saisie de plusieurs graphèmes', () => {
    const { input, onKey } = setup();
    beforeInput(input, 'insertText', 'êt');
    expect(onKey.mock.calls).toEqual([
      [{ kind: 'insert', typed: 'ê' }],
      [{ kind: 'insert', typed: 't' }],
    ]);
  });

  it('traduit Entrée en retour à la ligne et Retour arrière en suppression', () => {
    const { input, onKey } = setup();
    beforeInput(input, 'insertLineBreak', null);
    fireEvent.keyDown(input, { key: 'Backspace' });
    expect(onKey.mock.calls).toEqual([[{ kind: 'insert', typed: '\n' }], [{ kind: 'delete' }]]);
  });

  it('refuse le collage', () => {
    const { input, onKey } = setup();
    const event = beforeInput(input, 'insertFromPaste', 'tout le texte');
    expect(event.defaultPrevented).toBe(true);
    expect(onKey).not.toHaveBeenCalled();
  });
});

describe('TypingArea — affichage', () => {
  it('colore les caractères et garde le caractère attendu visible en cas d’erreur (F-02)', () => {
    const engine = new TypingEngine(splitGraphemes('ab c'), rules);
    engine.apply({ kind: 'insert', typed: 'a' });
    engine.apply({ kind: 'insert', typed: 'x' });
    engine.apply({ kind: 'insert', typed: 'x' });
    const { container } = render(
      <TypingArea
        engine={engine}
        version={3}
        lineStarts={[0]}
        settings={settings}
        disabled={false}
        onKey={() => {}}
        onColumns={() => {}}
      />,
    );
    const cells = container.querySelectorAll('[data-cell]');
    expect(Array.from(cells, (c) => c.textContent)).toEqual(['a', 'b', '·', 'c']);
    expect(Array.from(cells, (c) => Number(c.getAttribute('data-cell')))).toEqual([
      Cell.Correct,
      Cell.Wrong,
      Cell.Wrong,
      Cell.Pending,
    ]);
    expect(cells[3]).toHaveClass('border-b-2');
  });

  it('ne rend que les lignes visibles (F-03, NF-01)', () => {
    const text = 'un deux trois quatre';
    // Lignes : « un » / « deux » / « trois » / « quatre ».
    const { view } = setup(text, [0, 3, 8, 14]);
    expect(view.container.querySelectorAll('[data-cell]')).toHaveLength(8);
  });

  it('calcule la ligne active et la première ligne affichée', () => {
    expect(lineOf([0, 3, 8], 0)).toBe(0);
    expect(lineOf([0, 3, 8], 5)).toBe(1);
    expect(lineOf([0, 3, 8], 99)).toBe(2);
    expect(firstVisibleLine(4, 3, 'top')).toBe(4);
    expect(firstVisibleLine(4, 3, 'center')).toBe(3);
    expect(firstVisibleLine(0, 5, 'center')).toBe(0);
  });
});
