# Nova 0.3 — Carte des capacités

| Module | Responsabilité | Dépend de |
| --- | --- | --- |
| health-guidance | Score local explicable, priorités et alertes | diagnostics existants |
| software-center | Inventaire, mises à jour et informations logiciels | health-guidance |
| driver-center | Inventaire, Windows Update et fabricants détectés | health-guidance |
| reports-recovery | Rapport local exportable et parcours de retour arrière | health-guidance |
| release-quality | Installer, CI, signature et validation Windows | tous les modules |

Ordre : `health-guidance` → `software-center` et `driver-center` → `reports-recovery` → `release-quality`.

Limites : Nova ne télécharge pas de pilotes depuis des sources inconnues, ne ferme ni ne désinstalle automatiquement des logiciels, et toute action qui modifie Windows reste confirmée.
