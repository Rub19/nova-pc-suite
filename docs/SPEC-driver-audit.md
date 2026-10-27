# Spec: audit complet des pilotes

## Objectif

Donner une vue locale et lisible des pilotes et appareils Windows : familles matérielles, erreurs PnP, pilotes manquants et priorités critiques. L’audit ne modifie jamais Windows.

## Commandes

- Test front-end : `npm test -- src/lib.test.ts`
- Vérification PowerShell embarqué : `powershell -NoProfile -File tests/embedded-script-tests.ps1`
- Build : `npm run build`

## Structure

- `src-tauri/scripts/system.ps1` : inventaire PnP lecture seule.
- `src/lib.ts` : classification des familles PnP dans l’interface.
- `src/App.tsx` : synthèse de l’audit Pilotes.
- `tests/system-tests.ps1` et `src/lib.test.ts` : contrats de données et classification.

## Limites

- Toujours : exclure identifiants matériels, chemins et informations personnelles.
- Toujours : classer les erreurs code 28 comme pilotes manquants et signaler les catégories critiques.
- Jamais : télécharger, remplacer ou flasher un pilote/firmware lors de l’audit.

## Critères de succès

- L’audit retourne les compteurs par catégorie, appareils sans pilote et problèmes critiques.
- L’interface présente ces résultats sans masquer les codes Windows.
- Les tests de classification et le parseur des scripts embarqués passent.
