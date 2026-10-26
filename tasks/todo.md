# Nova — Task List

## 0.2.4 — erreurs de diagnostic, nettoyage auto et instance unique

- [x] Reproduire puis corriger les scripts embarqués UTF-8 ; tests des trois diagnostics par le transport réel.
- [x] Nettoyage automatique opt-in persistant, suppression directe et tests sur fixtures.
- [x] Instance unique, transition admin avec accusé de réception et conservation de l’onglet.
- [x] Détection automatique des fabricants et composants ; supports officiels connus.
- [ ] Test visuel de la version corrigée et publication de l’installateur.

## 0.2.3 — fiche matérielle et zone de notification

- [x] Fiche PC structurée en 12 rubriques, actualisation, recherche, copie et export.
- [x] Inventaire matériel local et exclusion des identifiants non nécessaires.
- [x] Menu gauche/droit dans la zone de notification, navigation, statut RAM, suspension et masquage.
- [x] 21 tests frontend et 15 tests Rust ; tests matériels simulés et lecture réelle sans mutation.
- [x] Compilation et publication de l’installateur 0.2.3.
- [x] Vérification RAM native sur processus isolé sans fenêtre, resté actif après réduction du working set.

## 0.2.2 — stockage, démarrage et énergie

- [x] Six outils supplémentaires : analyse/optimisation du disque système, lenteurs de démarrage et trois diagnostics de veille/réveil.
- [x] Confirmation, contrôle du support et mutex pour le stockage ; tests avec commandes OS simulées.
- [x] Filtre Stockage et tableau des événements de démarrage.
- [x] Compilation native, 14 tests Rust et publication de l’installateur 0.2.2.

## 0.2 — demande utilisateur : maintenance étendue

- [x] Audits et actions de maintenance étendues avec confirmations.
- [x] Référence et comparaison des mesures persistantes.
- [x] Nettoyage sélectionné avec récupération et purge explicite.
- [x] Démarrage utilisateur réversible et gestion des applications WinGet.
- [x] Analyses planifiées en lecture seule, bilan et notifications optionnelles.
- [x] Vérification des releases Nova et sources officielles constructeurs.
- [x] Diagnostic réseau et surveillance mémoire conditionnelle opt-in.
- [x] UI graphite, tests, revue, installateur et release publique 0.2.0 ; CI Windows et installation/désinstallation réussies sur runner GitHub.

## 0.2.1 — applications et confort

- [x] Demande de fermeture normale confirmée, identité et propriétaire contrôlés, aucun arrêt forcé.
- [x] Recherche/tri RAM, mémoire privée et nom ; filtre des applications gérables.
- [x] Recherche et filtres d’état au démarrage, compteurs et commandes intégrées.
- [x] Compilation locale, 15 tests d’interface, 14 tests Rust, tests PowerShell et publication de l’installateur 0.2.1.

- [x] Initialiser Tauri + React + TypeScript.
- [x] Design Nova, navigation, thèmes et dashboard.
- [x] Contrats de réponses, recommandations et tests.
- [x] Métriques réelles CPU/RAM/OS/matériel et redémarrage.
- [x] Diagnostic et historique local exportable.
- [x] Inventaire, recherche WUA, installation groupée et licences.
- [x] Restauration vérifiée, DISM/SFC, TRIM et profils énergie.
- [x] Stockage, démarrage, processus, protections et accès paramètres Windows.
- [x] Assistant local + Ollama optionnel, installateur NSIS et guide.

## Suite du produit (hors preview livrée)

- [x] Nettoyage interne avec aperçu de fichiers et récupération.
- [x] Gestion interne des apps et du démarrage.
- [x] Planification d’audits et notifications optionnelles.
- [ ] Mises à jour automatiques signées.
- [ ] Intégrations des catalogues constructeurs.
- [ ] Validation complète en VM Windows 10/11 et signature Authenticode.

## Checkpoint

- [x] Périmètre Windows 10/11 public confirmé par l’utilisateur.
- [x] Fondation et métriques réelles vérifiées.
- [x] Revue indépendante, garde-fous et tests en lecture seule.
- [ ] Validation complète des opérations privilégiées avant version stable.
