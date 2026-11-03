# Nova PC Suite

Entretien et réparation de Windows 10/11 x64. Interface en français, thèmes clair et sombre selon Windows, moteur Rust/Tauri, installateur NSIS. Licence MIT.

Nova ne montre que des mesures réelles. Chaque changement est mesuré avant et après, inscrit dans le Journal, et annulable quand Windows le permet.

## Ce que fait Nova

- **Bilan** : espace disque, santé des disques, blocages de carte graphique, arrêts brutaux, écrans bleus, plantages d'applications, appareils en erreur, antivirus, pare-feu, dernière mise à jour, temps de démarrage. Pas de note inventée : la liste de ce qui est à traiter, et ce qui va bien.
- **Pilotes** : appareils en erreur expliqués, pilotes proposés par Windows Update, sites officiels des fabricants détectés, inventaire filtrable.
- **Mises à jour** : Windows Update, logiciels via WinGet, historique, redémarrage différé et annulable.
- **Nettoyage** : temporaires, cache Windows Update, optimisation de la distribution, miniatures, shaders, caches des navigateurs, rapports de plantage, corbeille. Taille réelle avant, espace libre avant et après.
- **Démarrage** : registre, dossiers Démarrage et tâches à l'ouverture de session. Désactivation identique à celle du Gestionnaire des tâches, annulable d'un clic. Mémoire utilisée par programme, durée des derniers démarrages.
- **Performances** : mode d'alimentation avec mesure du calcul processeur avant et après, état de la carte NVIDIA, optimisation des disques, veille prolongée, programmes qui consomment, test de connexion, cache DNS.
- **Réparer** : DISM, SFC, chkdsk, anciens composants, points de restauration.
- **Journal** : toutes les modifications de Nova, avec leurs mesures et l'annulation.

Toute installation de pilote ou de mise à jour Windows, et toute réparation, est précédée d'un point de restauration. Si Windows ne peut pas en créer, Nova s'arrête sans rien modifier.

## Ce que Nova refuse de faire

Pas de « nettoyage de RAM » : vider la mémoire de travail d'un programme le ralentit ensuite. Pas de nettoyage du registre, pas de désactivation de services, pas de logiciel de sécurité désactivé, pas d'arrêt forcé de programme, pas de note de santé, pas de promesse de gain.

## Développement

```bash
npm ci
npm test
npm run tauri:build
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Les scripts PowerShell de `src-tauri/scripts` sont intégrés au binaire et n'acceptent que les actions listées dans `policy()` de `src-tauri/src/main.rs`.

Les binaires ne sont pas signés Authenticode : Windows peut afficher un avertissement d'éditeur inconnu.
