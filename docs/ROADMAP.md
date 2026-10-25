# Priorités après la preview

## 1. Fiabilité avant version stable

- Tester Windows 10 et 11 en VM et sur plusieurs matériels.
- Vérifier l’installation de pilotes, les échecs partiels, les restaurations et l’élévation UAC sur machines dédiées.
- Ajouter une progression détaillée et des résultats persistants par étape, y compris après réouverture de Nova.
- Signer les binaires Authenticode et les futures mises à jour de Nova.

## 2. Optimisation mesurable

- Comparer les mesures avant/après dans les mêmes conditions : démarrage, charge mémoire, espace récupéré, tâches actives.
- Nettoyage interne avec aperçu exact des fichiers, estimation fiable et récupération quand possible.
- Gestion du démarrage dans Nova avec sauvegarde et réactivation des entrées modifiées.
- Ne recommander une action que lorsqu’un diagnostic en justifie l’intérêt ; distinguer entretien, réparation et gain de performances.

## 3. Pilotes et applications

- Ajouter les sources officielles constructeurs/OEM avec correspondance matérielle et vérification des signatures.
- Prévoir sélection individuelle, exclusions, détails de version et chemin de retour arrière pour les pilotes.
- Inventaire et mises à jour des applications via mécanismes officiels, avec résultats par application.

## 4. Automatisation utile

- Analyses planifiées, notifications discrètes et profils de maintenance configurables.
- Historique détaillé, export de diagnostic avec masquage des informations personnelles.
- Réglages IA : choix explicite du modèle local, état du service et limites de ressources.

## Livré en 0.1.1

Thème sombre graphite par défaut, choix clair/sombre mémorisé, palette cohérente des alertes et badges, contrastes renforcés.
