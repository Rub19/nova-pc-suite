# Nova PC Suite 0.2.7 preview

Installeur : `Nova-PC-Suite-0.2.7-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.7.exe` (WebView2 requis).

## Nouveautés Pilotes

- Audit en lecture seule des appareils Plug and Play déclarés par Windows : nombre d’appareils, appareils en erreur et code Windows associé, dans une liste limitée.
- Comptage des pilotes dont Windows expose une métadonnée de signature. Ce compteur est indicatif : il ne signifie pas à lui seul qu’un pilote est dangereux ou à remplacer.
- Accès guidé à Windows Update et aux supports officiels des constructeurs détectés.
- Interface adaptée aux écrans étroits.

## Sécurité et limites

- Nova ne copie pas de catalogue tiers de pilotes et ne télécharge pas de pilotes inconnus. Les installations restent limitées aux mises à jour proposées par Windows Update, avec sélection, licences et confirmation explicites.
- L’audit ne lance aucune réparation, ne ferme aucun programme et ne collecte pas les identifiants matériels, numéros de série ou adresses réseau.
- Les binaires sont une preview non signée Authenticode ; Windows peut afficher un avertissement d’éditeur inconnu.
- La validation manuelle complète sur plusieurs PC/VM Windows 10 et Windows 11 reste à réaliser avant une version stable.

## Retour arrière

En cas de problème, désinstallez Nova depuis Paramètres > Applications > Applications installées et réinstallez une release précédente depuis GitHub.
