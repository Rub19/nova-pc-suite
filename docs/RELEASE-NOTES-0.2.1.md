Nova PC Suite 0.2.1 preview — applications et confort d’utilisation

Installeur : `Nova-PC-Suite-0.2.1-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.1.exe` (WebView2 requis).

- Bouton Fermer pour demander l’arrêt normal d’une application sélectionnée, après confirmation. Validation du propriétaire et de l’heure de création ; processus système et protections exclus. L’application peut demander un enregistrement ou refuser. Aucun arrêt forcé, ni fermeture automatique.
- Recherche par nom ou PID, tri par RAM/mémoire privée/nom et filtre des applications gérables dans la surveillance mémoire.
- Recherche et filtres Activés/Désactivés/Gérables au démarrage, compteurs et boutons intégrés Désactiver/Réactiver avec sauvegarde.
- Tous les outils de la 0.2.0 : maintenance, mesures avant/après, nettoyage récupérable, WinGet, pilotes Windows Update, réseau, surveillance RAM optionnelle et analyses planifiées.

La liste mémoire reste limitée aux 30 processus de plus de 100 Mio les plus consommateurs. Les modifications du démarrage gèrent les entrées HKCU Run prises en charge ; les autres sont en lecture seule. Désactiver au démarrage ne ferme pas une application déjà ouverte.

Tests automatisés et compilation sans prise de contrôle du bureau. Aucun programme utilisateur fermé, aucun changement appliqué au démarrage du PC pendant le développement. Fermeture validée par tests de contrat et interface simulée ; validation interactive Windows 10/11 encore nécessaire.

Preview non signée Authenticode. Les mises à jour de Nova restent manuelles ; les catalogues constructeurs ne sont pas agrégés automatiquement. Les diagnostics et comparaisons ne garantissent pas un gain de performances.
