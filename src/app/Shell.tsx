/** Coquille de l'application : menu latéral et zone de contenu. */

import { Suspense, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, NavLink, Outlet } from 'react-router';

import { commands, unwrap, type Profile } from '@/shared/api';
import { cn } from '@/shared/lib/utils';

import { buildMenu, type MenuEntry } from './menu';
import type { ModuleManifest } from './types';

/** Pastille d'une entrée de menu : le hook du module donne le nombre affiché. */
function MenuBadge({ useBadge }: { useBadge: NonNullable<MenuEntry['useBadge']> }) {
  const count = useBadge();
  // Rien à signaler : pas de pastille.
  if (!count) return null;
  return (
    <span className="ml-auto min-w-6 rounded-full bg-accent px-2 py-0.5 text-center text-xs font-bold text-accent-foreground tabular-nums">
      {count}
    </span>
  );
}

/** Menu construit depuis le registre ; la page du module s'affiche à droite. */
export function Shell({ modules }: { modules: readonly ModuleManifest[] }) {
  const { t } = useTranslation();
  // Le menu ne dépend que du registre.
  const menu = useMemo(() => buildMenu(modules), [modules]);
  const [profile, setProfile] = useState<Profile | null>(null);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [open, setOpen] = useState(false);

  // Profil actif, affiché en bas du menu ; une erreur laisse simplement l'emplacement vide.
  useEffect(() => {
    let active = true;
    unwrap(commands.currentProfile())
      .then((p) => active && setProfile(p))
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="flex h-screen bg-background text-foreground">
      <nav
        aria-label={t('common:nav.main')}
        className="flex w-60 shrink-0 flex-col gap-1 bg-sidebar p-4 text-sidebar-foreground"
      >
        {/* Logo : carré bleu « D_ » et nom de l'application. */}
        <p className="flex items-center gap-3 px-2 pt-2 pb-6 text-xl font-bold">
          <span
            aria-hidden
            className="grid size-9 place-items-center rounded-lg bg-primary font-mono text-sm text-primary-foreground"
          >
            D_
          </span>
          {t('common:appName')}
        </p>
        {menu.map(({ moduleId, path, labelKey, icon: Icon, useBadge }) => (
          <NavLink
            key={moduleId}
            to={path}
            // L'accueil n'est actif que sur « / » exactement, pas sur toutes les routes.
            end={path === '/'}
            className={({ isActive }) =>
              cn(
                'flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm outline-none',
                'hover:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-ring',
                isActive
                  ? 'bg-sidebar-accent font-semibold text-sidebar-foreground'
                  : 'text-sidebar-muted',
              )
            }
          >
            <Icon aria-hidden className="size-5" />
            {t(`${moduleId}:${labelKey}`)}
            {useBadge && <MenuBadge useBadge={useBadge} />}
          </NavLink>
        ))}
        {/* Profil actif, poussé en bas du menu ; un clic ouvre le changement de profil (F-50). */}
        {profile && open && (
          <div
            role="menu"
            aria-label={t('common:profile.switch')}
            className="mt-auto flex flex-col gap-1 rounded-xl bg-sidebar-accent p-2 text-sm"
          >
            {profiles.map((p) => (
              <button
                key={p.id}
                type="button"
                role="menuitemradio"
                aria-checked={p.id === profile.id}
                onClick={() => {
                  setOpen(false);
                  if (p.id === profile.id) return;
                  // Le nouveau profil est retenu, puis tout repart de l'accueil.
                  void unwrap(commands.switchProfile(p.id)).then(() => {
                    window.location.hash = '#/';
                    window.location.reload();
                  });
                }}
                className={cn(
                  'rounded-lg px-3 py-2 text-left outline-none hover:bg-sidebar focus-visible:ring-2 focus-visible:ring-ring',
                  p.id === profile.id && 'font-semibold',
                )}
              >
                {p.name}
              </button>
            ))}
            <Link
              to="/settings"
              role="menuitem"
              onClick={() => setOpen(false)}
              className="rounded-lg px-3 py-2 text-sidebar-muted outline-none hover:bg-sidebar focus-visible:ring-2 focus-visible:ring-ring"
            >
              {t('common:profile.manage')}
            </Link>
          </div>
        )}
        {profile && (
          <button
            type="button"
            aria-haspopup="menu"
            aria-expanded={open}
            aria-label={t('common:profile.switchFrom', { name: profile.name })}
            onClick={() => {
              // La liste est relue à chaque ouverture : un profil a pu être créé.
              if (!open) {
                void unwrap(commands.listProfiles())
                  .then(setProfiles)
                  .catch(() => undefined);
              }
              setOpen((o) => !o);
            }}
            className={cn(
              'flex items-center gap-3 rounded-xl bg-sidebar-accent p-3 text-left outline-none hover:brightness-110 focus-visible:ring-2 focus-visible:ring-ring',
              !open && 'mt-auto',
            )}
          >
            <span
              aria-hidden
              className="grid size-10 shrink-0 place-items-center rounded-full bg-accent font-bold text-accent-foreground"
            >
              {profile.name.charAt(0).toUpperCase()}
            </span>
            <div className="min-w-0">
              <p className="truncate text-sm font-semibold">{profile.name}</p>
              <p className="truncate text-xs text-sidebar-muted">
                {t('common:profile.details', {
                  layout: profile.keyboardLayout.toUpperCase(),
                  language: t('common:languageName'),
                })}
              </p>
            </div>
          </button>
        )}
      </nav>
      <main className="flex-1 overflow-auto px-12 py-10">
        {/* Les pages sont chargées à la demande : message d'attente entre-temps. */}
        <Suspense fallback={<p>{t('common:loading')}</p>}>
          <Outlet />
        </Suspense>
      </main>
    </div>
  );
}
