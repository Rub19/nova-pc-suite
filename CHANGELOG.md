# Changelog

## 0.2.12 preview — 2026-10-27

- Nouveau bouton « Analyser tous les pilotes » : inventaire local, appareils en erreur et pilotes applicables via Windows Update sont vérifiés en un seul parcours lecture seule.
- Les pilotes détectés par Windows Update restent tous présélectionnés pour l’installation groupée, qui nécessite toujours confirmation et droits administrateur.

## 0.2.11 preview — 2026-10-26

- Correction de la carte « Nettoyage automatique des temporaires » : case, titre et explication sont réunis dans une zone cliquable lisible avec survol et focus cohérents.

## 0.2.10 preview — 2026-10-26

- Menu de tri Mémoire reconstruit avec le style sombre Nova, état sélectionné et navigation clavier, sans le rendu hétérogène du menu natif Windows.
- La surveillance mémoire démarre après une mesure même sans application autorisée : elle observe d’abord, puis entretient uniquement les applications cochées par l’utilisateur.
- Message explicite lors de l’activation et possibilité d’ajouter ou retirer une application autorisée pendant la surveillance.

## 0.2.9 preview — 2026-10-26

- Fiche « À propos de ce PC » restructurée : deux colonnes indépendantes qui évitent les grandes zones vides entre des cartes de hauteurs différentes.
- Les groupes répétés (cartes graphiques, barrettes mémoire, volumes et adaptateurs réseau) restent courts ; les éléments supplémentaires sont accessibles par un panneau repliable.
- Les champs Windows indisponibles ne sont plus affichés comme des lignes vides dans la fiche, sans rien retirer du rapport exporté.

## 0.2.8 preview — 2026-10-26

- Installateur NSIS Nova personnalisé : illustration graphite et bleu, bandeau assorti, icônes cohérentes pour installation/désinstallation et dossier Démarrer dédié.
- Les visuels sont générés puis convertis dans le dépôt par un script reproductible, exécuté avant chaque bundle.

## 0.2.7 preview — 2026-10-26

- Audit local des appareils et pilotes : erreurs PnP déclarées par Windows, compteurs de signature et liste limitée des appareils à examiner.
- Parcours explicite vers Windows Update et les supports constructeurs, sans téléchargement, remplacement ou réparation automatique de pilote.
- Panneau Pilotes responsive et tests du nouvel audit en lecture seule.

## 0.2.6 preview — 2026-10-26

- Recherche globale navigable au clavier avec les flèches et Entrée, plus annonce accessible du résultat actif.
- Lien d’évitement vers le contenu principal pour les utilisateurs au clavier et lecteurs d’écran.
- Effacement volontaire du seul historique local de score de santé, sans toucher aux diagnostics Windows ni à l’historique des opérations.

## 0.2.5 preview — 2026-10-26

- Navigation regroupée, repliable et mémorisée, avec recherche globale au clavier (`Ctrl+K`).
- Tableau de santé local avec score explicable, historique minimal local et rapports texte/JSON qui excluent processus, numéros de série et adresses réseau.
- Assistant de première utilisation, suggestions d’entretien non automatiques et états de chargement plus clairs dans le tableau de bord.
- Centre logiciel enrichi avec résultats WinGet lisibles ; catégories et filtres pour les pilotes installés.
- Mesure mémoire guidée avant affichage des processus ; inventaire matériel chargé seulement à l’ouverture de l’onglet pour alléger le démarrage.
- Menus déroulants, cartes et libellés affinés pour une interface sombre plus cohérente.

## 0.2.4 preview — 2026-10-26

- Correction des erreurs PowerShell intégrées UTF-8 qui empêchaient les diagnostics matériel, mémoire et réseau.
- Contrôle du vrai texte embarqué en CI, diagnostics natifs en lecture seule et erreurs de transport explicites.
- Nettoyage des temporaires automatique opt-in persistant et suppression manuelle directe depuis l’aperçu.
- Instance unique et remplacement de la session lors du passage administrateur, avec repli en cas d’annulation.
- Détection locale des fabricants et regroupement des composants avec supports officiels disponibles.

## 0.2.3 preview — 2026-10-25

- Onglet « À propos de ce PC », 12 rubriques matérielles/système, recherche et rapport texte copiable/exportable.
- Inventaire CIM limité aux propriétés utiles, indisponibilités partielles explicites et résultat détaillé exclu de l’historique.
- Icône Nova dans la zone de notification, menu gauche/droit, navigation et masquage de la fenêtre.
- Statut de la surveillance RAM et suspension depuis le menu ; test simulé du cycle automatique complet.

## 0.2.2 preview — 2026-10-25

- Analyse et optimisation confirmée du volume système via Windows, selon le support, sans défragmentation forcée.
- Tableau des lenteurs de démarrage signalées dans les événements récents.
- Diagnostics des blocages de veille, états disponibles, dernière source de réveil et minuteurs.
- Filtre Stockage, erreurs de diagnostic explicites et contrôles supplémentaires automatisés.

## 0.2.1 preview — 2026-10-25

- Fermeture normale confirmée des seules applications autorisées, sans arrêt forcé.
- Recherche et tri des applications par RAM/mémoire privée/nom ; filtre par processus gérable.
- Recherche et filtres d’état au démarrage, compteurs et boutons Désactiver/Réactiver.
- Tests de non-exécution avant confirmation et d’annulation, sans fermer de programme utilisateur.

## 0.2.0 preview — 2026-10-25

- Outils de maintenance Windows supplémentaires, rapports énergie/batterie et comparaison de mesures.
- Nettoyage récupérable des temporaires anciens avec purge distincte et contrôles de fichiers par handle.
- Gestion réversible du démarrage utilisateur, sauvegardes DPAPI, inventaire et mises à jour WinGet par identifiant exact.
- Diagnostic réseau, connexions par processus, DNS et sondes courtes de latence/pertes.
- Surveillance mémoire opt-in et entretien conditionnel des seuls processus autorisés.
- Audit planifié utilisateur, bilan persistant et notifications Windows optionnelles.
- Supports constructeurs officiels et vérification des versions Nova publiées.
- Sélection des pilotes WUA et résultats d’opérations enregistrés dans l’historique.

Signature publique, validation client Windows 10/11 sur plusieurs machines et auto-updater signé restent hors validation de cette preview.

## 0.1.1 preview — 2026-10-25

- Thème sombre graphite par défaut, avec conservation du choix clair/sombre.
- Couleurs adaptées pour alertes, badges, icônes, dialogues, indicateurs et boutons.
- Contrastes renforcés et bordures plus discrètes en mode sombre.

## 0.1.0 preview — 2026-10-25

- Application Windows 10/11 x64 avec icône Nova et installateur.
- Diagnostic local, pilotes, recherche et installation groupée Windows Update.
- Licences explicites, résultats par pilote et indication de redémarrage.
- Restauration, réparation Windows, TRIM, profils d’alimentation.
- Stockage, démarrage, processus, sécurité, historique et assistant Ollama.
- Commandes allowlistées, identifiants validés, confirmations et verrou global de maintenance.
- Aucune télémétrie ni clé cloud.

Preview non signée. Validation Windows 10 et tests privilégiés sur machines dédiées encore nécessaires. Mises à jour limitées aux propositions de Windows Update. Nettoyage et configuration du démarrage dans les paramètres Windows.
