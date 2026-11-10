# Parité Tauri → Rust natif (Slint)

Code natif : `native/engine` (moteur sans Tauri), `native/app` (Slint, exe `nova-native`), `native/setup` (installeur).
`[v]` porté et vu marcher dans la visite hors écran (`NOVA_OFFSCREEN=1 NOVA_TOUR=<dossier>`). `[x]` porté et compilé, pas vu en vrai. `[ ]` pas porté.

Revue du 9 novembre 2026 : chaque appel au moteur des pages Tauri (`src/pages/*.tsx`) existe dans le natif, chaque titre de section et chaque bouton aussi. Rien n'a été essayé à la souris.

## Coquille (`src/App.tsx`, `src/ui.tsx`)

- [v] navigation par 8 onglets, Ctrl+1 à Ctrl+8, compteurs « à traiter »
- [v] activité en direct : processeur, mémoire, disque en courbes
- [v] mode administrateur affiché, « Relancer en administrateur »
- [v] toasts avec fermeture, fenêtre de confirmation
- [v] À propos (version, attribution Slint)
- [x] `app_ready` : propre à Tauri, remplacé par l'ouverture directe de la fenêtre

## Bilan

- [v] « Ce PC », « Ce qui va bien », points à traiter qui mènent à la bonne page
- [v] « Copier le bilan », actualiser

## Démarrage

- [v] liste des programmes au démarrage avec interrupteur, emplacement, programme absent du disque
- [x] logiciels de sécurité protégés : étiquette « Protégé : sécurité » (Tauri le disait dans une bulle au survol, Slint 1.18 n'a pas de bulle) ; pas vue, aucun logiciel de sécurité au démarrage sur ce PC
- [v] « Droits requis », « Relancer en administrateur »
- [v] « Durée des derniers démarrages »

## Nettoyage

- [v] analyse, catégories cochées, « Nettoyer », actualiser

## Performances

- [v] « Ce qui occupe le PC en ce moment », fermer un programme
- [v] « Mode d'alimentation » avec choix du plan, « Veille prolongée »
- [v] « Disques », « Connexion » avec test

## Mises à jour

- [v] « Windows » : recherche, installation, « Dernières installations de Windows »
- [v] « Logiciels » : « Mettre à jour », « Tout mettre à jour (n) »

## Pilotes

- [v] « Pilotes installés » avec filtre par nom, fabricant, version, et tout afficher
- [v] « Appareils en erreur », « Mises à jour proposées par Windows » avec sélection
- [v] « Sites des fabricants de ce PC »

## Réparer

- [v] « Vérifications », « Points de restauration », « Restaurer le système… »

## Journal

- [v] filtres Tout, Échecs, Annulables, « Copier », annuler une action

## Ajouts du natif (absents de Tauri)

- [v] espace disque, stabilité détaillée, plus gros dossiers, logiciels installés, cartes réseau
- [x] installeur natif (`native/setup`) : remplace Nova Tauri sur place, mise à jour, désinstallation avec retrait de l'audit planifié ; essayé seulement en dossier de test

## Pas vérifié

- Clics réels à la souris sur les 8 écrans, installation réelle, actions qui modifient le PC (nettoyage, mises à jour, pilotes, restauration) : jamais lancées depuis Claude.
- Mise à jour automatique (nouvelle, absente de Tauri) : Nova lit `latest.json` des versions GitHub de `Rub19/nova-pc-suite` au lancement puis toutes les 6 h, carte « Nova X disponible » dans la barre latérale, signature minisign vérifiée avant de lancer l'installeur en `/P /UPDATE /R`, état dans « À propos ». Pas encore active : la clé n'existe pas (`native\cle-maj.ps1` la crée, à lancer par Rub19, puis `update.pub` à commiter). `release.ps1` signe ensuite et écrit `latest.json`. Jamais essayée de bout en bout : il faut la clé et une version GitHub normale (les actuelles sont des préversions, que `releases/latest` ignore).
