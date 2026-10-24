// Identité de l'application : nom affiché et préfixe des clés de stockage
// local (`fabweave.theme.v1`, `fabweave.section.plate`…). Remplacer `fabweave`
// par l'identifiant de l'application, en minuscules, sans espace.

export const APP_ID = 'fabweave';
export const APP_NAME = 'FabWeave';

/** Clé du stockage local, préfixée par l'application. */
export const storageKey = (key: string) => `${APP_ID}.${key}`;
