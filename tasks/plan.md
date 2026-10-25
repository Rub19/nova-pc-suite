# Implementation Plan: Nova PC Suite

## Vue d’ensemble

Livrer une suite Windows Tauri/React/Rust en tranches verticales, en commençant par un shell visuel réellement utilisable puis en branchant les métriques locales et les opérations système les moins risquées.

## Décisions d’architecture

- Tauri 2 est retenu pour réduire la taille de l’installateur et isoler les opérations natives dans Rust.
- Les commandes système sont exposées par des commandes Rust dédiées, sans endpoint générique de shell.
- Le frontend ne décide jamais seul d’une action privilégiée : il demande une intention typée et reçoit un résultat structuré.
- Les recommandations sont séparées des actions : une analyse peut conseiller sans modifier le PC.
- Les fichiers utilisateur et les logs restent dans les emplacements applicatifs Windows ; rien n’est écrit dans le dépôt.

## Tâches

### Phase 1 — Fondation

- [ ] Initialiser le projet Tauri + React + TypeScript, les scripts et le `.gitignore`.
- [ ] Créer le design system Nova, la navigation et le tableau de bord accessible avec données de démonstration clairement marquées.
- [ ] Ajouter les types de contrats `SystemSnapshot`, `HealthScan`, `Recommendation` et les tests de logique.

### Point de contrôle Fondation

- [ ] Le frontend démarre, le build réussit et les tests passent.
- [ ] Le clavier permet de parcourir la navigation et d’ouvrir une recommandation.

### Phase 2 — Données système réelles

- [ ] Exposer les métriques CPU/RAM/disque/uptime via une commande Rust non privilégiée.
- [ ] Brancher le dashboard sur ces données avec états loading/error/permission.
- [ ] Ajouter un scan système non destructif et son historique local.

### Point de contrôle Système

- [ ] Les données affichées sont issues de l’hôte ou explicitement indisponibles.
- [ ] Aucun appel shell arbitraire n’est possible depuis l’UI.

### Phase 3 — Pilotes et optimisations sûres

- [ ] Inventorier les pilotes via une source Windows allowlistée et normaliser les versions.
- [ ] Afficher les recommandations de vérification et les liens Windows officiels.
- [ ] Implémenter une première optimisation réversible et son aperçu avant/après.

### Phase 4 — Maintenance étendue

- [ ] Stockage, fichiers temporaires, démarrage, applications, énergie et point de restauration.
- [ ] Centre confidentialité/sécurité et export d’audit.

### Phase 5 — Assistant, packaging et release

- [ ] Ajouter un assistant informatif avec permission nulle par défaut.
- [ ] Ajouter l’icône, le manifeste, l’installateur NSIS, la désinstallation et la mise à jour.
- [ ] Mettre en place CI, signature si disponible et documentation de support.

## Risques et mitigations

| Risque | Impact | Mitigation |
|---|---:|---|
| Actions système trop agressives | Élevé | Preview, confirmation, allowlist, opérations réversibles |
| Variations entre éditions Windows | Élevé | Détection de capacité, état indisponible, fixtures et tests sur plusieurs versions |
| Pilotes tiers non fiables | Élevé | Windows Update/constructeurs officiels, pas de miroir inconnu |
| Promesse IA trop large | Moyen | Assistant explicatif sans pouvoir d’action, local par défaut |
| Build natif non reproductible | Moyen | Lockfiles, versions documentées, CI Windows |

## Questions bloquantes pour la publication

- Identité du dépôt GitHub et visibilité public/privé.
- Cible Windows 10 seule, Windows 11 seule, ou les deux.
- Signature de code et certificat de publication.
