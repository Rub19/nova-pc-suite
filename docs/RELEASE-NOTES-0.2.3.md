Nova PC Suite 0.2.3 preview — fiche du PC et zone de notification

Installeur : `Nova-PC-Suite-0.2.3-setup.exe`. Exécutable autonome : `Nova-PC-Suite-0.2.3.exe` (WebView2 requis).

- Nouvel onglet « À propos de ce PC » : ordinateur, Windows, CPU, barrettes RAM, contrôleurs graphiques, carte mère, BIOS, disques, volumes/espace libre, cartes réseau, audio et batterie.
- Recherche des composants, accès aux rubriques, actualisation, copie et export d’un rapport texte complet. Les champs absents et sources indisponibles sont distingués des valeurs fausses ou nulles.
- Icône Nova dans la zone de notification Windows : clic gauche ou droit pour ouvrir le menu et accéder directement aux écrans utiles.
- État de la surveillance RAM dans le menu et bouton pour la suspendre. Option « Masquer la fenêtre » pour laisser Nova ouvert en arrière-plan ; « Quitter » arrête sa surveillance. La fermeture normale de la fenêtre continue de quitter Nova.
- Nouveaux tests du cycle RAM automatique avec mesures simulées : pression soutenue, seul processus autorisé entretenu, aucune fermeture de programme, suspension via l’événement du menu.
- Outils 0.2.2 conservés : optimisation native du disque, lenteurs au démarrage et diagnostics de veille/réveil.

Les accès du menu ouvrent les écrans ; ils ne lancent pas de maintenance ni d’arrêt de programme directement. La visibilité de l’icône dans la barre principale ou derrière la flèche dépend de Windows. Nova doit rester lancé ; aucune inscription automatique au démarrage n’est ajoutée.

L’inventaire est local et ne collecte pas les numéros de série, clés produit ou adresses IP/MAC. Les données détaillées de cet inventaire ne sont pas conservées dans l’historique. Certaines valeurs SMBIOS/WMI peuvent être absentes ou inexactes ; VRAM dédiée, températures et tensions ne sont pas mesurées.

Tests et compilation réalisés sans prise de contrôle du bureau ni arrêt/entretien RAM des programmes utilisateur. Appel natif RAM vérifié sur un programme de test isolé sans fenêtre : working set d’environ 270 Mio réduit à 2,1 Mio, processus resté actif. Ses pages pourront revenir en mémoire ; ce test ne garantit pas une accélération d’autres applications. La fiche matérielle a été interrogée en lecture seule. La présentation et les clics sur l’icône nécessitent encore une validation interactive sur des postes dédiés. Preview non signée Authenticode ; auto-updater signé et validation complète Windows 10/11 toujours à compléter.
