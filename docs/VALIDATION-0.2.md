# Validation Nova 0.2.0 preview

## Périmètre testé

- Build TypeScript/Vite et tests React en DOM simulé : confirmations, annulation, purge verrouillée et résultat d’erreur partiel.
- Règles mémoire : pression soutenue, seuil, délais, pagination, absence de consentement, PID recyclé et exclusion des processus non gérables.
- Rust : contrats allowlistés, transport PowerShell réel, diagnostic réel, audit réel, chiffrement DPAPI, rejet de chemins, fichiers changés et liens physiques, déplacement/récupération de fixtures et restauration sans écrasement.
- Contrôleur de maintenance : 62 contrôles avec commandes Windows mockées, chemins, point de comparaison, mutex et erreurs.
- Applications : mocks registre/WinGet, identifiants exacts, absence de commande Run exécutée, sauvegarde avant retrait, conflit conservé, DPAPI et inventaire local en lecture seule.
- Planification : mocks Task Scheduler, tâche utilisateur sans élévation, actions fixes, conflits de propriété, retrait de la seule tâche propriétaire à la désinstallation et préférences limitées aux fixtures.
- Réseau/mémoire : syntaxe PS5.1, données de réseau synthétiques, refus de PID invalide avant mutation ; audit réseau et relevé mémoire réels en lecture seule.
- Recherche GitHub réelle : version publiée retrouvée. Audit npm sans vulnérabilité connue au moment du build.

## Ce qui n’a pas été testé sur le PC utilisateur

À sa demande, aucune prise de contrôle du bureau pendant cette évolution. Aucune modification réelle du démarrage, tâche planifiée, installation de pilote/application, réparation, purge de temporaires utilisateur ou réduction de working set n’a été exécutée pour tester cette version. Les seules suppressions testées concernent des fichiers synthétiques générés dans les fixtures du projet.

Le rendu de la 0.2 est testé par DOM simulé et compilation ; pas par automatisation des fenêtres. La 0.1.1 a été inspectée visuellement lors du tour précédent, avant cette contrainte utilisateur.

## À terminer avant une version stable

- VM Windows 10/11 propres et plusieurs matériels, avec cas UAC accepté/refusé, politiques entreprise et stockage compatible/incompatible.
- Essais d’installation effective de pilotes et applications, licences, erreurs et redémarrages ; restaurations réelles.
- Mesures de l’effet de l’entretien working set sur la pagination et la réactivité de vrais logiciels, dans un environnement de test dédié.
- Notifications Windows/AUMID et tâches planifiées dans plusieurs éditions ; confirmer leur retrait par le hook de désinstallation sur machines dédiées.
- Certificat Authenticode public et signature du binaire avant packaging, puis de l’installateur. Le script de signature refuse un certificat absent, expiré ou dépourvu d’usage Code Signing. Aucun certificat utilisable n’a été trouvé ; la preview reste non signée.
- Moteur multi-catalogue constructeur et mécanisme de mise à jour automatique signé : non livrés, les accès officiels et la vérification de release sont disponibles.

Les essais GitHub Actions utilisent les images Windows Server des runners ; ils ne remplacent pas une certification sur les éditions clientes Windows 10 et 11.
