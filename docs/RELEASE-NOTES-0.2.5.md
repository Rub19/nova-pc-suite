# Nova PC Suite 0.2.5 preview

Installeur : `Nova-PC-Suite-0.2.5-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.5.exe` (WebView2 requis).

Cette préversion rend Nova plus rapide à ouvrir et plus simple à parcourir, sans exécuter d’optimisation ou de fermeture de programme automatiquement.

## Nouveautés

- Navigation par groupes repliables mémorisés et palette de recherche globale `Ctrl+K` pour atteindre rapidement chaque outil.
- Tableau de santé local : score explicable, actions conseillées, mini-historique limité aux scores et horodatages, rapports `.txt` et `.json` partageables. Les rapports excluent noms de processus, numéros de série et adresses réseau.
- Assistant de première utilisation et suggestions locales : elles expliquent les prochaines étapes ; aucune action de maintenance n’est lancée par elles.
- Centre logiciel amélioré : recherche et résultats WinGet lisibles avant toute mise à jour confirmée par l’utilisateur.
- Pilotes regroupés en catégories (graphique, réseau, audio, stockage, chipset, périphériques) avec filtre local. Les liens fabricants sont des supports officiels détectés ; Nova n’installe pas de catalogue tiers inconnu.
- Mémoire : état initial explicite avant la première mesure et rappel que la surveillance ne ferme aucun programme ; seules les applications autorisées par l’utilisateur peuvent être entretenues dans les conditions affichées.
- Interface : menus déroulants, cartes, libellés d’attente et thème sombre affinés.
- Performance : l’inventaire détaillé de « À propos de ce PC » n’est chargé qu’à l’ouverture de l’onglet, ce qui allège l’affichage initial.

## Installer et vérifier

1. Téléchargez l’installateur ou l’exécutable autonome et le fichier `SHA256SUMS.txt` depuis cette release.
2. Vérifiez l’empreinte SHA-256 avant l’installation si vous le pouvez.
3. L’installateur installe Nova pour l’utilisateur courant et crée le désinstalleur Windows. L’exécutable autonome demande WebView2.

## Limites connues de la preview

- Les binaires ne sont pas signés Authenticode : Windows peut afficher un avertissement d’éditeur inconnu.
- La validation manuelle complète sur plusieurs PC/VM Windows 10 et Windows 11, ainsi que le parcours UAC de bout en bout, reste à compléter avant une version stable.
- Les gains de performance ne sont pas universels. Nova présente les mesures et demande les confirmations nécessaires ; Windows reste responsable de la gestion mémoire et des pilotes.
- La mise à jour automatique de Nova n’est pas encore disponible. Les mises à jour passent par la page GitHub officielle.

## Retour arrière

En cas de problème, désinstallez cette preview depuis Paramètres > Applications > Applications installées puis réinstallez la release précédente. Les actions de maintenance restent dans l’historique local de Nova ; les éléments de démarrage gérés disposent de leurs sauvegardes dédiées.
