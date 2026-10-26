# Spécification — bilan de santé local

## Objectif

Donner un bilan clair et explicable à partir des diagnostics déjà locaux : RAM, stockage, redémarrage Windows, sécurité et mises à jour pilotes. Le score ne modifie pas Windows ; il sert à orienter vers la bonne page.

## Critères de succès

- Score entre 0 et 100, accompagné d’un niveau et de causes lisibles.
- Aucune donnée personnelle, identifiant matériel ou télémétrie n’est exportée ou transmise.
- Chaque cause mène à un outil Nova existant.
- Le score est couvert par des tests déterministes.

## Commandes

- Tests : `npm test`
- Build : `npm run build`

## Limites

- Toujours : calcul local, texte explicable, actions Windows confirmées.
- Demander avant : nouvelle dépendance, nouvelle source externe, changement CI.
- Jamais : déclencher nettoyage, pilote, arrêt de processus ou désinstallation depuis le score.
