# Passage de relais : Nova PC Suite natif (Rust + Slint)

Dépôt `C:\Claude\nova-rust`, branche `rust-natif`. Code natif dans `native/` : `engine` (moteur sans Tauri), `app` (Slint, exe `nova-native`), `setup` (installeur). Rien n'est publié, `main` n'a pas bougé, le code Tauri est toujours là.

## Règles
- Commits : `git -c user.name=Rub19 -c user.email=rub19.mailpro@gmail.com commit`, style `feat:` ou `fix:`, sans co-auteur IA, zéro commentaire dans le code, `git add` avec chemins précis.
- Pas de fusion, pas de publication, pas de suppression de Tauri sans l'OK de Rub19.
- Ne jamais lancer l'installeur pour de vrai depuis Claude (conteneur MSIX) : seulement en dossier de test.
- Compiler en priorité basse (`start /low`, `-j 3`). Aucune fenêtre visible pendant un jeu.

## Fait et vérifié
- 59 + 7 tests du moteur et de l'app, plus 2 de l'installeur.
- Visite hors écran (`NOVA_OFFSCREEN=1 NOVA_TOUR=<dossier>`) : les 8 écrans, Ctrl+1 à 8, clics sur Bilan et Journal, dialogue, À propos.
- « Plus gros dossiers » corrigé : le moteur renvoyait le résultat sans l'enveloppe `ok`, l'écran l'affichait comme une erreur.
- Installeur Rust/Slint (`native/setup`), modèle d'Ethone, Argus et Kairo : accueil ou mise à jour, progression, terminé, erreur avec Réessayer, désinstallation, raccourci bureau, lancement avec Windows, `/S`, `/P /UPDATE /R /ARGS`. Il remplace Nova Tauri sur place : même dossier `%LOCALAPPDATA%\Nova PC Suite`, même nom `nova-pc-suite.exe`, même clé `Uninstall\Nova PC Suite`. Il range l'ancien raccourci du dossier `Nova PC Suite` du menu Démarrer et la clé Tauri `Software\github\Nova PC Suite`. La désinstallation retire aussi l'audit planifié, avec les mêmes garde-fous que `scripts/uninstall-task.ps1`.
- Essayé en dossier de test (`NOVA_SETUP_TEST=<dossier>`, plus `NOVA_SETUP_SNAP` ou `NOVA_SETUP_OFFSCREEN`) sur une fausse installation Tauri 0.3.0 : passage au natif, mise à jour passive, silencieuse, désinstallation ; captures des écrans vérifiées.
- Version 0.4.0. Dernier installeur : `native\release\Nova-PC-Suite_0.4.0_x64-setup.exe`, 14,3 Mo, SHA-256 `F4C8F8BB6BE604896F9F8A9E278E6DCD29438771299E2CA09A23E3F9D1F35192` (installeur allégé le 9 novembre : app compressée en xz au lieu de Deflate, rendu logiciel de Slint seul ; installé et désinstallé en dossier de test, exe identique) (recompilé le 9 novembre avec l'étiquette « Protégé : sécurité »). Installé et désinstallé en silencieux dans un dossier de test, exe identique.

## Pas vérifié
- Installation réelle, retrait réel de la tâche planifiée, icônes dans le menu Démarrer et la barre des tâches.
- Fichier de parité : `docs/RUST_PARITY.md`, fait le 9 novembre. Tous les appels au moteur, titres et boutons de Tauri se retrouvent dans le natif ; visite hors écran refaite sur les 8 écrans.

## Reste à faire
1. Essais de Rub19 : lancer `native\release\Nova-PC-Suite_0.4.0_x64-setup.exe`, puis les 8 écrans à la souris.
2. Fait : fichier de parité. Reste : motion à relire.
3. Après son OK : retirer Tauri sur la branche, adapter `.github` et la publication.

## Commandes (dans `native/`)
```
cargo test --workspace -j 3
powershell -NoProfile -ExecutionPolicy Bypass -File release.ps1
```

## Parcours de bout en bout (événements Slint)

`powershell -File native/e2e/run.ps1` compile avec la fonction `e2e` (absente des builds installés) et rejoue `native/e2e/nova.txt` hors écran : clics par libellé accessible ou texte affiché, frappe touche par touche, raccourcis, vérifications, capture à chaque échec. Journal et captures dans `%TEMP%`. 41 étapes sans aucune fenêtre (rendu headless) : les 8 écrans, filtres, Ctrl+1 à 8, À propos, sans rien modifier sur le PC. Bugs trouvés et corrigés : raccourcis morts après une saisie puis un changement de page, version 0.3.0 dans À propos.
Limite : un élément coupé par le clip d'un parent compte encore comme visible.
