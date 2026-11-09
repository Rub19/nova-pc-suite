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
- Version 0.4.0. Dernier installeur : `native\release\Nova-PC-Suite_0.4.0_x64-setup.exe`, 16,1 Mo, SHA-256 `70750DA87FF9F5AE9A7E9564358693E24D9CF388EB862909F28E1B5FDFD2295B`. Installé et désinstallé en silencieux dans un dossier de test, exe identique.

## Pas vérifié
- Installation réelle, retrait réel de la tâche planifiée, icônes dans le menu Démarrer et la barre des tâches.
- Pas de fichier de parité détaillé Tauri contre natif.

## Reste à faire
1. Essais de Rub19 : lancer `native\release\Nova-PC-Suite_0.4.0_x64-setup.exe`, puis les 8 écrans à la souris.
2. Fichier de parité écran par écran, motion et nouvelles fonctions.
3. Après son OK : retirer Tauri sur la branche, adapter `.github` et la publication.

## Commandes (dans `native/`)
```
cargo test --workspace -j 3
powershell -NoProfile -ExecutionPolicy Bypass -File release.ps1
```
