# Nova PC Suite 0.2.8 preview

Installeur : `Nova-PC-Suite-0.2.8-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.8.exe` (WebView2 requis).

## Nouvel installateur Nova

- Accueil et écran de fin avec une illustration Nova originale graphite et bleu, au lieu du visuel NSIS générique.
- En-tête harmonisé, icônes dédiées pour l’installation et la désinstallation, et raccourcis regroupés dans le menu Démarrer « Nova PC Suite ».
- Installation par utilisateur courant : aucune demande administrateur n’est nécessaire pour installer ou désinstaller Nova. Les fonctions qui modifient Windows restent protégées par leurs confirmations et l’élévation explicite dans Nova.

## Limites de la preview

- Le style des pages et textes reste celui du moteur NSIS/Tauri ; Nova le personnalise avec les capacités officiellement supportées du bundle, sans remplacer le moteur d’installation complet.
- Les binaires ne sont pas encore signés Authenticode. Windows peut donc afficher un avertissement d’éditeur inconnu.
- La validation manuelle complète de l’interface de l’installateur sur plusieurs versions de Windows 10/11 reste nécessaire avant une version stable.
