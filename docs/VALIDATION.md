# Validation de la preview 0.1.0

## Vérifications réalisées

- Compilation TypeScript/Vite, tests unitaires de recommandations et erreurs.
- Tests Rust de l’allowlist, identifiants et transport UTF-8/PowerShell.
- Régression pour scripts dépassant la limite de ligne de commande Windows.
- Diagnostic embarqué par le transport natif réel.
- Inventaires locaux en lecture seule : snapshot, drivers, startup, security, storage, processes.
- Garde-fous WUA et licences par mocks : aucune installation réelle.
- Build de l’exécutable et de l’installateur NSIS x64.
- Installation silencieuse locale dans un répertoire de test dédié, contrôle d’intégrité (hors marqueur de type de bundle Tauri), désinstallation réussie et exécutable de test retiré.
- Vérification visuelle et confirmation/annulation dans l’interface.
- Recherche WUA réelle depuis l’exécutable : deux pilotes applicables retournés, aucun téléchargement ni installation lancé.
- Affichage du diagnostic réel dans la fenêtre native, y compris le redémarrage en attente.
- Audit npm : aucune vulnérabilité connue lors du build.

## À valider sur machines dédiées avant une version stable

- Installation, mise à niveau et désinstallation sur VM Windows 10 et 11 propres, avec et sans WebView2.
- UAC accepté/refusé, politiques entreprise, deux instances concurrentes, fermeture forcée.
- Installation réelle des pilotes, licences, échecs partiels, réseau et redémarrage sur machines dédiées.
- Restauration active/inactive et limite de fréquence Windows.
- DISM/SFC sur VM avec inspection CBS.log ; TRIM compatible/incompatible.
- Ollama avec modèle réel et contrôle des ressources.

Aucune maintenance n’a été appliquée au PC de développement. La preview ne doit pas être présentée comme validée sur tous les matériels Windows.

## Retour arrière

Release associée à un tag et SHA-256 ; CI Windows avec lockfiles. En cas de défaut, retirer la release de la distribution et publier un correctif sous un nouveau tag. Ne pas remplacer silencieusement les binaires. Les changements de pilotes/système s’annulent avec les mécanismes Windows, pas en désinstallant Nova.
