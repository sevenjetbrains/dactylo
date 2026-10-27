import { Suspense, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { NavLink, Outlet } from 'react-router';

import { cn } from '@/shared/lib/utils';

import { buildMenu } from './menu';
import type { ModuleManifest } from './types';

export function Shell({ modules }: { modules: readonly ModuleManifest[] }) {
  const { t } = useTranslation();
  const menu = useMemo(() => buildMenu(modules), [modules]);

  return (
    <div className="flex h-screen bg-background text-foreground">
      <nav
        aria-label={t('common:nav.main')}
        className="flex w-56 shrink-0 flex-col gap-1 border-r border-border p-3"
      >
        <p className="px-3 pb-3 text-lg font-semibold">{t('common:appName')}</p>
        {menu.map(({ moduleId, path, labelKey, icon: Icon }) => (
          <NavLink
            key={moduleId}
            to={path}
            end={path === '/'}
            className={({ isActive }) =>
              cn(
                'flex items-center gap-2 rounded-md px-3 py-2 text-sm outline-none',
                'hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring',
                isActive && 'bg-muted font-medium',
              )
            }
          >
            <Icon aria-hidden className="size-4" />
            {t(`${moduleId}:${labelKey}`)}
          </NavLink>
        ))}
      </nav>
      <main className="flex-1 overflow-auto p-8">
        <Suspense fallback={<p>{t('common:loading')}</p>}>
          <Outlet />
        </Suspense>
      </main>
    </div>
  );
}
