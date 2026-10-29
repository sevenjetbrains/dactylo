/** Tests du clavier virtuel : touche suivante, modificateurs et doigt conseillé. */

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { VirtualKeyboard } from './VirtualKeyboard';

/** Touche de test. */
const key = (code: string, base: string, finger: string, shift: string | null = null) => ({
  code,
  base,
  shift,
  altgr: null,
  dead: false,
  finger,
  width: 1,
});

/** Disposition réduite : quatre touches, deux Maj. */
const layout = {
  id: 'azerty',
  rows: [
    [key('KeyA', 'q', 'leftPinky', 'Q'), key('KeyJ', 'j', 'rightIndex', 'J')],
    [key('BracketLeft', '^', 'rightPinky', '¨'), key('KeyE', 'e', 'leftMiddle', 'E')],
    [key('ShiftLeft', '⇧', 'leftPinky'), key('ShiftRight', '⇧', 'rightPinky')],
  ],
};

/** Frappes renvoyées par `keyboardKeys` pour « q », « J » et « ê ». */
const presses: Record<string, unknown[]> = {
  q: [{ code: 'KeyA', shift: false, altgr: false, caps: false }],
  J: [{ code: 'KeyJ', shift: true, altgr: false, caps: false }],
  ê: [
    { code: 'BracketLeft', shift: false, altgr: false, caps: false },
    { code: 'KeyE', shift: false, altgr: false, caps: false },
  ],
};

// Backend simulé : la disposition et les touches de chaque graphème demandé.
vi.mock('@/shared/api', () => ({
  commands: {
    keyboardLayout: () => Promise.resolve({ status: 'ok', data: layout }),
    keyboardKeys: (g: string[]) =>
      Promise.resolve({ status: 'ok', data: g.map((x) => presses[x] ?? []) }),
  },
  unwrap: async (p: Promise<{ status: string; data?: unknown }>) => (await p).data,
}));

describe('VirtualKeyboard', () => {
  it('annonce la touche suivante et le doigt conseillé', async () => {
    render(<VirtualKeyboard text={['q', 'J', 'ê']} position={0} />);
    expect(await screen.findByRole('status')).toHaveTextContent(
      'Touche suivante : Q — auriculaire gauche',
    );
  });

  it('allume la touche et la Maj de l’autre main', async () => {
    const { container } = render(<VirtualKeyboard text={['q', 'J', 'ê']} position={1} />);
    expect(await screen.findByRole('status')).toHaveTextContent('J — index droit · avec Maj');
    const lit = [...container.querySelectorAll('.bg-primary')].map((e) => e.textContent);
    expect(lit).toEqual(['J', '⇧']);
  });

  it('touche morte puis lettre pour un accent', async () => {
    render(<VirtualKeyboard text={['q', 'J', 'ê']} position={2} />);
    expect(await screen.findByRole('status')).toHaveTextContent(
      'Touche suivante : ^ puis E — auriculaire droit',
    );
  });

  it('rien d’annoncé une fois le texte fini', async () => {
    render(<VirtualKeyboard text={['q']} position={1} />);
    expect(await screen.findByRole('status')).toBeEmptyDOMElement();
  });
});
