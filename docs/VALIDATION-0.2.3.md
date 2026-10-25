# Validation — fiche du PC et icône Windows

`hardware/hardware-info` est une action de lecture seule, sans paramètres libres. Douze classes CIM sont interrogées avec des propriétés sélectionnées, puis projetées dans un schéma stable. Une classe indisponible ne fait pas disparaître les autres sections. Le résultat détaillé n’est pas persisté dans l’historique ; seule la trace de l’action l’est.

L’interface conserve son relevé pendant la session, affiche des rubriques avec unités, permet la recherche et produit un rapport texte complet. Elle n’utilise pas AdapterRAM pour afficher la VRAM : sa largeur de 32 bits ne permet pas de représenter fidèlement une capacité supérieure à 4 Gio. Les valeurs de rafraîchissement 0/1 sont considérées inconnues. Les valeurs false et une charge batterie à 0 % sont conservées.

L’icône utilise l’API native Tauri tray, avec menu sur clic gauche et droit. Les liens du menu sont limités à une liste de pages et n’invoquent aucune maintenance. Masquer conserve l’application ; Quitter la termine. La commande de statut RAM ne peut changer que le libellé et l’état du bouton de suspension. L’événement de suspension invalide le cycle automatique courant et arrête les prochains relevés ; il ne peut pas annuler une opération native déjà envoyée.

Vérifications :

- Inventaire matériel interrogé en lecture seule : douze classes disponibles sur la machine de développement, sans afficher leurs détails dans les journaux de travail.
- Tests matériels avec CIM simulé : propriétés projetées, champs identifiants exclus, mémoire physique/utilisable séparées, fréquence inconnue et indisponibilité partielle.
- 21 tests frontend passent, dont ouverture de l’onglet, conservation du relevé, copie du rapport et présentation des données manquantes. Le test du mode RAM automatique utilise une horloge et des mesures simulées : entretien d’un seul processus choisi, aucune fermeture, arrêt des cycles après suspension depuis le menu.
- 15 tests Rust passent : routes du menu limitées à la navigation et absence de route pour les actions de maintenance ; inventaire matériel explicitement en lecture seule.

La compilation ne valide pas les clics réels de la zone de notification ni le rendu visuel. Aucun pilotage du bureau n’a été utilisé. Validation interactive sur Windows 10/11 dédiée encore nécessaire. Aucune libération de mémoire, fermeture de programme, optimisation ou modification de démarrage réelle sur le PC utilisateur pendant ces tests. La version 0.2.2 précédente a réussi les tests Windows et l’installation/désinstallation sur runner GitHub.

Références :

- [Tauri — System Tray](https://v2.tauri.app/learn/system-tray/)
- [Microsoft — Win32_PhysicalMemory](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-physicalmemory)
- [Microsoft — Win32_VideoController](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-videocontroller)
