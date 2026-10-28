/** Coquille de l'application : menu latéral et zone de contenu. */

import { Suspense, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { NavLink, Outlet } from 'react-router';

import { commands, unwrap, type Profile } from '@/shared/api';
import { cn } from '@/shared/lib/utils';

import { buildMenu } from './menu';
import type { ModuleManifest } from './types';

/** Menu construit depuis le registre ; la page du module s'affiche à droite. */
export function Shell({ modules }: { modules: readonly ModuleManifest[] }) {
  const { t } = useTranslation();
  // Le menu ne dépend que du registre.
  const menu = useMemo(() => buildMenu(modules), [modules]);
  const [profile, setProfile] = useState<Profile | null>(null);

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
        {menu.map(({ moduleId, path, labelKey, icon: Icon }) => (
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
          </NavLink>
        ))}
        {/* Profil actif, poussé en bas du menu. */}
        {profile && (
          <div className="mt-auto flex items-center gap-3 rounded-xl bg-sidebar-accent p-3">
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
          </div>
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
