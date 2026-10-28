/** Page Réglages. */

import { useTranslation } from 'react-i18next';

import { useModules } from '@/app/use-modules';
import { Card, CardTitle } from '@/shared/ui/card';

/** Agrège les panneaux de réglages déclarés par chaque module dans le registre. */
export default function SettingsPage() {
  const { t } = useTranslation();
  // Seuls les modules qui déclarent un panneau apparaissent.
  const panels = useModules().flatMap((m) =>
    m.settingsPanel ? [{ moduleId: m.id, ...m.settingsPanel }] : [],
  );

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <h1 className="text-2xl font-semibold">{t('settings:title')}</h1>
      {panels.map(({ moduleId, titleKey, component: Panel }) => (
        <Card key={moduleId}>
          <CardTitle>{t(`${moduleId}:${titleKey}`)}</CardTitle>
          <Panel />
        </Card>
      ))}
    </div>
  );
}
