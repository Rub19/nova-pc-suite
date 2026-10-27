# Nova PC Suite 0.2.12 preview

Installeur : `Nova-PC-Suite-0.2.12-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.12.exe` (WebView2 requis).

## Analyse complète des pilotes

- Un seul bouton analyse l’inventaire local, les erreurs signalées par Windows et les pilotes applicables proposés par Windows Update.
- Tous les pilotes proposés sont sélectionnés pour faciliter l’installation groupée, mais l’utilisateur conserve le contrôle de la sélection, de la confirmation et de l’élévation Windows.
- Les pages constructeurs détectées restent disponibles pour les matériels OEM ou pilotes absents de Windows Update.

## Limites de la preview

- Nova ne télécharge ni ne remplace automatiquement des pilotes depuis des sites tiers.
- Les binaires ne sont pas encore signés Authenticode. Windows peut donc afficher un avertissement d’éditeur inconnu.
