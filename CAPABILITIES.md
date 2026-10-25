# Nova — Capability Map

Nova est une suite Windows de diagnostic, maintenance et optimisation assistée, avec des actions réversibles et explicables.

| Module | Responsabilité | Dépend de |
|---|---|---|
| `app-shell` | Fenêtre, navigation, thème, onboarding, permissions et notifications | — |
| `system-health` | Inventaire matériel/OS, score de santé, scans et historique | `app-shell` |
| `driver-center` | Inventaire des pilotes, détection des versions, recommandations de mise à jour | `system-health` |
| `safe-optimizer` | Recommandations d’optimisation, nettoyage sûr, démarrage et énergie | `system-health` |
| `storage-care` | Analyse des volumes, fichiers temporaires et espace récupérable | `system-health` |
| `privacy-security` | Contrôles locaux de confidentialité, sécurité et restauration | `system-health` |
| `apps-startup` | Applications installées, processus de démarrage et désinstallation guidée | `system-health` |
| `nova-assistant` | Explications, résumés et conseils contextualisés, sans exécution autonome | `system-health`, `safe-optimizer` |
| `release-platform` | Packaging, signature, installateur, mises à jour et publication | `app-shell` |

## Ordre de construction

`app-shell` → `system-health` → `driver-center` + `safe-optimizer` → `storage-care` + `apps-startup` → `privacy-security` → `nova-assistant` → `release-platform`

Chaque module doit rester utilisable seul, avec des résultats réels ou un état explicitement marqué comme indisponible.
