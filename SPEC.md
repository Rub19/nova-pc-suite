# Spec: Nova PC Suite

## Objectif

Construire une application Windows `.exe` claire et professionnelle qui aide un utilisateur non expert à comprendre l’état de son PC, repérer les pilotes obsolètes, récupérer de l’espace, réduire les tâches inutiles au démarrage et appliquer des optimisations prudentes.

Le produit doit privilégier la confiance : aucune modification sensible n’est silencieuse, chaque recommandation affiche son impact et son niveau de risque, les opérations importantes sont réversibles quand c’est possible, et les données restent locales par défaut.

### Utilisateurs cibles

- Utilisateur Windows qui veut entretenir son PC sans connaître les commandes système.
- Utilisateur avancé qui veut un tableau de bord synthétique et un historique des changements.
- Famille ou petite équipe qui veut une routine de maintenance simple et explicable.

### Promesse produit

> Comprendre son PC en quelques secondes, savoir quoi faire ensuite, et garder le contrôle à chaque étape.

## Périmètre fonctionnel

### Version 0.1 — fondation démontrable

- Tableau de bord local avec santé générale, CPU/RAM/disque/uptime et dernière analyse.
- Analyse système réelle, non destructive, avec états `ok`, `attention`, `critique` et `indisponible`.
- Centre des pilotes : inventaire version/fabricant/périphérique, détection des éléments à vérifier et raccourci vers les mécanismes Windows officiels.
- Centre d’optimisation : recommandations sûres, aperçu avant exécution, confirmation explicite et journal d’audit.
- Navigation, recherche globale, thème sombre clair, responsive desktop et accessibilité clavier.
- Gestion des permissions et messages d’erreur compréhensibles.

### Version 0.2 — maintenance quotidienne

- Nettoyage des fichiers temporaires et corbeille avec estimation de l’espace récupérable.
- Analyse du stockage par catégorie et par dossier.
- Gestion des applications au démarrage.
- Profils énergie et recommandations selon l’usage.
- Création et vérification d’un point de restauration avant les actions groupées.

### Version 0.3 — sécurité et confort

- Contrôles de confidentialité locaux et état des protections Windows.
- Inventaire des applications installées et désinstallation guidée.
- Planification de scans locaux.
- Notifications et historique exportable.
- Assistant Nova qui explique les résultats, sans lancer de commande seul.

### Plus tard, sous réserve de validation

- Synchronisation de préférences entre appareils.
- Connecteurs optionnels à des services d’IA externes, avec consentement et masquage des données.
- Télémetrie produit strictement opt-in et minimisée.

## Stack technique

- Tauri 2 pour l’application native et le packaging Windows.
- React + TypeScript + Vite pour l’interface.
- Rust pour les opérations système, avec commandes allowlistées et paramètres typés.
- `sysinfo` pour les métriques de base ; APIs Windows ou commandes système fixes uniquement pour les données qui ne sont pas disponibles autrement.
- Vitest/Testing Library pour la logique et les composants ; tests d’intégration Tauri ciblés ; Playwright plus tard pour les flux critiques.
- Installeur NSIS via Tauri, cible Windows 10/11 x64 en premier.

## Commandes prévues

```text
npm install
npm run dev
npm run test
npm run build
npm run tauri:dev
npm run tauri:build
cargo test --manifest-path src-tauri/Cargo.toml
```

## Structure cible

```text
src/                    # UI React/TypeScript
src/components/         # composants visuels focalisés
src/features/           # écrans et logique par capacité
src/lib/                # contrats, formatage, état partagé minimal
src-tauri/src/          # commandes système Rust
src-tauri/icons/        # icônes générées et sources SVG
tests/                  # tests de logique et composants
docs/                   # architecture, sécurité, support
tasks/                  # plan et tâches actives
```

## Principes d’interface

- Hiérarchie visuelle calme, typographie lisible, surfaces claires et accents limités.
- Densité d’information utile : pas de grille de cartes décoratives sans action.
- Couleur jamais seule pour signaler un état ; toujours un libellé et une icône.
- Contrôles accessibles au clavier et libellés explicites.
- Chaque action système montre : ce qui va changer, niveau de risque, possibilité d’annuler, et état après exécution.
- Les opérations longues utilisent un état de progression et restent annulables si la plateforme le permet.

## Modèle de confiance et sécurité

- Les données collectées sont locales par défaut et minimisées.
- Aucun shell arbitraire : les commandes sont codées en dur, avec arguments fixes ou listes allowlistées.
- Aucun nettoyage du registre, aucune suppression de pilotes, aucun arrêt de protection de sécurité en version initiale.
- Les mises à jour de pilotes privilégient les canaux Windows ou constructeur officiels ; Nova ne distribue pas de pilotes inconnus.
- Les résultats système sont traités comme des données non fiables et validés avant affichage.
- Les secrets et jetons ne sont jamais stockés dans le dépôt ou les logs.
- Une action destructive exige une confirmation claire ; les chemins et fichiers sont vérifiés contre des racines autorisées.
- L’assistant IA ne possède pas de permission d’exécution ; ses sorties sont informatives et validées avant affichage.

## Stratégie de tests

- Tests unitaires majoritaires pour scores, normalisation de versions, recommandations et garde-fous.
- Tests composants pour états de chargement, erreur, permission, succès et confirmation.
- Tests d’intégration pour les commandes Rust avec fixtures déterministes et sans dépendre d’un PC particulier.
- Tests manuels sur Windows 10/11 pour l’élévation, l’installateur, la désinstallation et les scénarios sans permission administrateur.
- Audit de dépendances avant chaque release.

## Limites

### Toujours faire

- Expliquer toute action système avant exécution.
- Préserver les données et préférer les opérations réversibles.
- Tester les nouveaux comportements et vérifier le build avant chaque commit.
- Garder un journal local lisible des actions demandées.

### Demander avant

- Ajouter une intégration cloud ou un fournisseur d’IA.
- Activer une action qui demande des droits administrateur persistants.
- Modifier le schéma de télémétrie ou les règles de confidentialité.
- Publier une release publique ou pousser vers un dépôt GitHub non encore identifié.

### Ne jamais faire

- Exécuter une commande fournie par l’utilisateur ou par un modèle sans validation et allowlist.
- Supprimer silencieusement des fichiers, pilotes ou services.
- Désactiver antivirus, pare-feu, mises à jour ou protections Windows pour “optimiser”.
- Embarquer des secrets, télécharger des binaires depuis une source inconnue, ou contourner les protections de signature.

## Critères de succès de la première tranche

- L’application démarre en mode développement et affiche un tableau de bord sans erreur console.
- Les métriques CPU/RAM/disque/uptime viennent de l’hôte local ou affichent clairement leur indisponibilité.
- Un scan peut être lancé, suivi, terminé et retrouvé dans l’historique.
- Les actions à risque sont visibles comme recommandations non exécutées par défaut.
- L’interface fonctionne au clavier, avec états de chargement/erreur/succès.
- Le build frontend et les tests passent sur une installation propre.

## Questions ouvertes

- Le dépôt GitHub doit-il être public ou privé ?
- Faut-il cibler uniquement Windows 11 ou aussi Windows 10 ?
- Souhaites-tu une IA locale uniquement, ou autorises-tu un fournisseur cloud après consentement ?
- Quel nom de marque définitif préfères-tu si « Nova » n’est qu’un nom de travail ?
