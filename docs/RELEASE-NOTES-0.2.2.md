Nova PC Suite 0.2.2 preview — stockage, démarrage et énergie

Installeur : `Nova-PC-Suite-0.2.2-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.2.exe` (WebView2 requis).

Six nouveaux outils dans Maintenance :

- Analyser le volume système avec les statistiques Windows.
- Optimiser le volume système en laissant Windows choisir l’opération adaptée au support, à sa priorité par défaut. Confirmation et droits administrateur requis. Aucune défragmentation forcée sur SSD.
- Repérer les lenteurs du démarrage dans les événements Windows récents, avec tableau des applications, pilotes et services signalés, durées et dates.
- Identifier les applications ou pilotes qui empêchent la veille.
- Consulter les modes de veille disponibles et les raisons de leur indisponibilité.
- Consulter la dernière source de réveil et les minuteurs de réveil actifs.

Filtre Stockage ajouté à la maintenance. Les diagnostics d’énergie ne désactivent pas les applications, périphériques ou minuteurs. Les événements de démarrage ne constituent pas une preuve de causalité et ne déclenchent aucune désactivation.

L’outil de stockage cible uniquement le volume système fixe NTFS déclaré sain par Windows. Sur un autre format, un volume non sain ou un support non pris en charge, Nova refuse cette opération. Windows peut déterminer qu’aucune action n’est nécessaire. Aucun gain universel de performances garanti.

Développement, tests simulés et compilation sans contrôle du bureau ni maintenance appliquée au PC utilisateur. Preview non signée Authenticode ; validation sur postes Windows 10/11 dédiés et auto-updater signé toujours à compléter. Les fonctions précédentes sont conservées.
