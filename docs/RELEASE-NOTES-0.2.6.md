# Nova PC Suite 0.2.6 preview

Installeur : `Nova-PC-Suite-0.2.6-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.6.exe` (WebView2 requis).

## Nouveautés

- La recherche globale `Ctrl+K` est complètement utilisable au clavier : flèches haut/bas pour parcourir les rubriques, Entrée pour ouvrir celle qui est sélectionnée et Échap pour effacer la recherche.
- Un lien « Aller au contenu principal » apparaît à la navigation clavier. Il évite de traverser le menu latéral à chaque changement de page.
- Le mini-historique du score de santé peut être effacé volontairement. Cette action retire uniquement les scores et horodatages enregistrés par Nova dans le stockage local ; elle ne supprime ni diagnostics, ni fichiers, ni historique d’opérations Windows.

## Vérification et limites

- Tests d’interface, build de production, diagnostics en lecture seule et contrôles de sécurité sont exécutés en local et dans le pipeline Windows GitHub.
- Les binaires restent non signés Authenticode ; Windows peut signaler un éditeur inconnu.
- La preview n’effectue aucune optimisation, fermeture de programme ou installation de pilote sans action et confirmation explicites de l’utilisateur.
- La validation manuelle complète sur plusieurs PC/VM Windows 10 et Windows 11 reste nécessaire avant une version stable.

## Retour arrière

En cas de problème, désinstallez Nova depuis Paramètres > Applications > Applications installées et réinstallez la release précédente. Les précédentes previews restent disponibles sur GitHub.
