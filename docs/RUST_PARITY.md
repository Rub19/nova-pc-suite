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

## Direction artistique (10 novembre 2026)

Nouvelle direction « instrument de mesure » : papier chaud en clair (`#f3f1ec`), graphite en sombre (`#111413`), un seul accent vert sarcelle (`#0e6b5c` / `#5fd4b5`), barre latérale claire avec la page active en carte, filet de séparation, titres en Segoe UI Variable Display, texte en Segoe UI Variable Text. Toutes les couleurs passent par `Theme` (plus aucune couleur en dur hors l'interrupteur). Vu en clair et en sombre sur les captures du parcours (thème choisi dans À propos).

## 20 fonctionnalités en plus (demandées le 10 novembre 2026)

`[ ]` à faire, `[x]` fait et vérifié (tests ou parcours e2e), avec la preuve dans « Fait ensuite » ou ici. Corbeille, cache DNS, temps depuis le redémarrage et santé des disques existaient déjà : remplacés par d'autres.

- [~] 1. Démarrage rapide de Windows : état et réglage (le pare-feu, prévu ici, est déjà vérifié par le bilan) : ligne « Démarrage rapide » sous Veille prolongée dans Performances : état lu dans le registre, Activer ou Désactiver en administrateur, journalisé et annulable ; pas lancé depuis Claude (modifie le PC)
- [x] 2. Doublons dans Téléchargements (même taille et même contenu) : section « Doublons dans Téléchargements » de Nettoyage : même taille puis même contenu, à partir de 1 Mo, lecture seule (tests files_are_sorted_by_kind_age_and_content, identical_files_have_the_same_fingerprint) ; lu sur ce PC en 2,5 s (5 groupes), écran pas encore vu ; vu dans le parcours e2e à 68 étapes
- [x] 3. Vieux gros fichiers dans Téléchargements : section « Vieux gros fichiers dans Téléchargements » : plus de 100 Mo et 90 jours ; lu sur ce PC (2 fichiers), écran pas encore vu ; vu dans le parcours e2e à 68 étapes
- [x] 4. Place prise par type de fichier dans le profil (vidéos, images, archives, installeurs) : section « Vos fichiers par type » : Bureau, Documents, Images, Vidéos, Musique, Téléchargements ; vu dans le parcours e2e à 68 étapes
- [~] 5. Réinitialiser la pile réseau (Winsock), en administrateur : ligne « Réseau » dans Réparer : netsh winsock reset et int ip reset en administrateur, journalisé, redémarrage demandé ; pas lancé depuis Claude (modifie le PC)
- [~] 6. Santé de la batterie (capacité d'origine et actuelle) : dans le bilan : capacité d'origine et actuelle (WMI), alerte sous 60 % (test battery_bitlocker_and_wifi_show_in_the_report) ; pas encore vu sur ce PC
- [x] 7. BitLocker : état du chiffrement de chaque disque : ligne « BitLocker » de « Ce PC » (lisible en administrateur) ; relevé e2e/pc.txt sur ce PC, 2026-11-11 : historique « 5 le 11 nov., 4 le 10 nov. » (doublons du même jour retirés, plus de double point), BitLocker « lisible en mode administrateur », tâches planifiées listées avec noms lisibles, interrupteur du nettoyage du dimanche présent
- [x] 8. Thème clair ou sombre choisi dans Nova, au lieu de suivre Windows : « Thème » dans À propos : Comme Windows, Clair, Sombre, gardé dans theme.txt ; vu dans le parcours e2e à 68 étapes
- [x] 9. Services en démarrage automatique arrêtés : section « Services arrêtés en erreur » dans Performances (démarrage automatique, arrêtés avec un code d'erreur), lue sur ce PC : aucun (test failed_services_are_read) ; vu dans le parcours e2e à 68 étapes
- [x] 10. Tâches planifiées de logiciels tiers, avec désactivation : section « Tâches planifiées des logiciels » dans Démarrage : tâches hors Microsoft lancées à heure fixe ou sur un événement, avec l'interrupteur existant (test native_startup_matches_powershell adapté) ; relevé e2e/pc.txt sur ce PC, 2026-11-11 : historique « 5 le 11 nov., 4 le 10 nov. » (doublons du même jour retirés, plus de double point), BitLocker « lisible en mode administrateur », tâches planifiées listées avec noms lisibles, interrupteur du nettoyage du dimanche présent
- [~] 11. Désinstaller un logiciel depuis la liste des logiciels installés : lien « Désinstaller » sur chaque logiciel installé : confirmation, puis Nova ouvre le désinstalleur lu dans le registre à partir d'un identifiant (jamais une commande venue de l'écran), journalisé (test uninstall_commands_are_split_like_windows) ; jamais lancé depuis Claude
- [x] 12. Historique du bilan : nombre de points à traiter dans le temps : ligne « Points à traiter au fil des analyses » sous la date du bilan, 30 analyses gardées (test the_history_keeps_thirty_scans) ; relevé e2e/pc.txt sur ce PC, 2026-11-11 : historique « 5 le 11 nov., 4 le 10 nov. » (doublons du même jour retirés, plus de double point), BitLocker « lisible en mode administrateur », tâches planifiées listées avec noms lisibles, interrupteur du nettoyage du dimanche présent
- [x] 13. Export du bilan dans un fichier texte : bouton « Enregistrer » du bilan : fichier Nova-bilan-date.txt dans Documents, montré dans l'Explorateur (test report_files_are_named_by_date) ; e2e/bilan.txt sur ce PC, 2026-11-12 : Nova-bilan-….txt écrit dans le dossier d'essai (pas d'Explorateur pendant les tests) ; au passage l'alerte « disque presque plein » ne se déclenche plus avec 139 Go libres (seuils en Go)
- [x] 14. Nettoyage planifié (chaque semaine), désactivable : interrupteur « Nettoyage automatique chaque dimanche » dans Nettoyage : tâche Windows de l'utilisateur (schtasks, sans fenêtre) qui lance nova --scheduled-clean, catégories cochées par défaut sans droits administrateur, passage visible dans le Journal, tâche retirée à la désinstallation ; sur ce PC il prendrait user-temp et thumbnails (test the_weekly_clean_only_takes_safe_ticked_categories) ; tâche jamais créée depuis Claude ; relevé e2e/pc.txt sur ce PC, 2026-11-11 : historique « 5 le 11 nov., 4 le 10 nov. » (doublons du même jour retirés, plus de double point), BitLocker « lisible en mode administrateur », tâches planifiées listées avec noms lisibles, interrupteur du nettoyage du dimanche présent
- [~] 15. Point de restauration automatique avant les actions risquées (option) : existait déjà avant les réparations de Windows et les mises à jour de Windows ; ajouté avant la réinitialisation du réseau et le changement du démarrage rapide. Toujours actif (Windows n'en crée qu'un par 24 h), pas d'option pour le couper ; jamais lancé depuis Claude
- [x] 16. Jeux : Mode Jeu de Windows et planification GPU accélérée (le choix du plan d'alimentation existait déjà) : section « Jeux » dans Performances : Mode Jeu de Windows réglable sans droits administrateur (annulable), état de la planification GPU avec lien vers Paramètres ; lu sur ce PC : Mode Jeu désactivé, planification jamais réglée (test gaming_settings_are_read) ; vu dans le parcours e2e à 68 étapes
- [x] 17. Test de vitesse du disque : « Tester la vitesse » dans Performances : 256 Mo écrits puis lus sans le cache de Windows, fichier supprimé ; mesuré sur ce PC (C: 3 323 Mo/s en écriture, 2 793 Mo/s en lecture, test the_disk_speed_test_cleans_up_after_itself) ; vu dans le parcours e2e à 68 étapes
- [~] 18. Wi-Fi : force du signal et réseaux enregistrés : ligne « Wi-Fi » de « Ce PC » : réseau, signal, réseaux enregistrés, lus par netsh sans fenêtre (test wifi_is_read_from_netsh_in_french_or_english) ; pas encore vu sur ce PC
- [x] 19. Outils Windows en un clic : section « Outils de Windows » dans Réparer (périphériques, événements, ressources, tâches, services, disques, informations système), liste fermée côté moteur (test `only_known_windows_tools_open`), vue dans le parcours ; boutons « Ouvrir » non cliqués par le parcours pour ne pas ouvrir de vraie fenêtre
- [~] 20. Espace libre conseillé avant une grosse mise à jour Windows : note dans Mises à jour quand il reste moins de 20 Go sur le disque système (test little_free_space_is_flagged_before_a_big_update) ; écran pas encore vu
