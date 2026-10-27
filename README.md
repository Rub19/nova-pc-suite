# Nova PC Suite

> Current preview: **0.2.14**. The notification-area menu provides lightweight RAM measurement, opt-in memory supervision, and safe temporary-cleanup shortcuts. See [release notes](docs/RELEASE-NOTES-0.2.14.md).

Application Windows 10/11 x64 de diagnostic, pilotes et maintenance. Interface française, thèmes clair/sombre, moteur Rust/Tauri et installateur NSIS. Licence MIT.

## Télécharger

[Releases GitHub](https://github.com/Rub19/nova-pc-suite/releases) — **0.2.14 preview**, contrôles rapides et entretien RAM plus léger.

- `Nova-PC-Suite-0.2.14-setup.exe` : véritable installateur avec désinstallation Windows.
- `Nova-PC-Suite-0.2.14.exe` : exécutable autonome, WebView2 requis.
- `SHA256SUMS.txt` : empreintes de contrôle.

Binaires non signés Authenticode : Windows peut afficher un avertissement d’éditeur inconnu. La preview doit encore être validée sur plusieurs PC et VM Windows 10 avant une version stable.

## Fonctions

- Diagnostic CPU, mémoire, système, matériel, uptime et redémarrage en attente.
- Inventaire réel des pilotes : version, fabricant, date et signature.
- Recherche des pilotes applicables via Windows Update Agent, y compris les pilotes optionnels proposés par le service configuré.
- Installation groupée de la liste confirmée : nouvelle vérification des identifiants, lecture et acceptation explicite des licences nécessaires, téléchargement, résultat par pilote et indication de redémarrage.
- Création et vérification d’un point de restauration.
- Réparation Windows par DISM puis SFC ; codes de sortie affichés.
- ReTrim du SSD système si pris en charge.
- Profils Équilibré et Performances élevées si disponibles.
- Volumes, espace libre, processus, mémoire consommée, applications au démarrage, inventaire antivirus et profils pare-feu.
- Accès aux outils de nettoyage, démarrage et sécurité Windows.
- Recommandations locales ; IA optionnelle via Ollama avec le premier modèle installé sur le PC.
- Historique local des 100 dernières opérations et export JSON ; recherche et pagination des tableaux.

### Nouveautés 0.2.13

- Installateur Nova redessiné : identité graphite/bleu, visuel personnalisé d’accueil et de fin, icônes cohérentes et raccourcis classés dans le dossier Démarrer Nova PC Suite.
- Même installateur par utilisateur, avec désinstallation Windows ; aucun privilège administrateur n’est imposé pour l’installation elle-même.
- [Notes de version et limites](docs/RELEASE-NOTES-0.2.13.md).

### Nouveautés 0.2.7

- Audit Pilotes local : appareils en erreur signalés par Windows, métadonnées de signature et marche à suivre vers Windows Update ou le fabricant.
- L’audit est en lecture seule et limite les appareils affichés ; il ne télécharge, n’installe ni ne remplace un pilote.
- [Notes de version et limites](docs/RELEASE-NOTES-0.2.7.md).

### Nouveautés 0.2.6

- Recherche globale `Ctrl+K` avec navigation ↑/↓, Entrée et sélection annoncée aux technologies d’assistance.
- Lien d’évitement vers le contenu principal, visible à la navigation clavier.
- Effacement de l’historique local du score de santé, sans supprimer de diagnostics ni modifier Windows.
- [Notes de version et limites](docs/RELEASE-NOTES-0.2.6.md).

### Nouveautés 0.2.5

- Navigation regroupée, repliable et mémorisée, recherche globale `Ctrl+K`, états de chargement lisibles et cartes plus cohérentes.
- Score de santé local explicable, tendance locale limitée et rapports `.txt`/`.json` sans processus, numéros de série ni adresses réseau.
- Assistant de première utilisation, suggestions locales non automatiques et parcours de mesure mémoire plus clair.
- Centre logiciel avec résultats WinGet détaillés ; pilotes classés (graphique, réseau, audio, stockage, chipset et périphériques) et filtrables.
- L’inventaire matériel lourd est chargé seulement à l’ouverture de « À propos de ce PC » pour accélérer le démarrage.
- [Notes de version et limites](docs/RELEASE-NOTES-0.2.5.md).

### Nouveautés 0.2.4

- Correction des erreurs de scripts UTF-8 sur la fiche PC, la mémoire et le réseau ; tests sur les scripts réellement embarqués.
- Nettoyage automatique opt-in, choix conservé : au lancement puis toutes les heures lorsque Nova est ouvert. Suppression définitive des fichiers éligibles de plus de sept jours, à la racine des temporaires utilisateur, maximum 1 000 par passage. Dossiers, liens et fichiers occupés/modifiés conservés.
- Suppression manuelle directe depuis l’aperçu et boutons Tout sélectionner/Tout désélectionner. La quarantaine reste disponible.
- Instance unique et transition vers le mode administrateur, avec restauration de la fenêtre si l’UAC est annulé ou si la transition échoue.
- Détection automatique des fabricants et composants, supports officiels disponibles et liste des autres constructeurs identifiés. Aucun catalogue tiers agrégé ni installation constructeur automatique.
- [Validation et limites](docs/VALIDATION-0.2.4.md).

### Nouveautés 0.2.3

- **À propos de ce PC** : 12 rubriques de composants et système, unités lisibles, recherche, copie et export texte. Inventaire en lecture seule, données manquantes explicites, sans numéros de série, clés produit ou adresses IP/MAC. Certains capteurs et la VRAM dédiée ne sont pas mesurés.
- **Zone de notification** : icône Nova avec menu sur clic gauche/droit, accès aux écrans, statut RAM, suspension de la surveillance et masquage de la fenêtre. Quitter Nova arrête les futurs cycles de surveillance. La fermeture normale continue de quitter l’application.
- Les accès rapides ne lancent aucune optimisation directement. Windows gère le placement de l’icône. Pas d’ajout automatique de Nova au démarrage.
- Voir [validation et limites](docs/VALIDATION-0.2.3.md).

### Nouveautés 0.2.2

- **Stockage** : analyse du volume système NTFS puis optimisation confirmée avec le choix natif Windows selon le support. Volume fixe déclaré sain uniquement ; aucun autre volume ciblé et aucun mode de défragmentation imposé.
- **Démarrage** : tableau des événements de ralentissement des sept derniers jours, limité à 30 événements. Aucun pilote/service désactivé automatiquement ; un événement ne prouve pas à lui seul la cause d’une lenteur.
- **Énergie** : programmes/pilotes qui empêchent la veille, modes de veille disponibles et causes de réveil/minuteurs. Diagnostics sans modifier leurs paramètres.
- Filtre Stockage et résultats locaux. Voir [validation et limites](docs/VALIDATION-0.2.2.md).

### Nouveautés 0.2.1

- Fermeture normale d’une application sélectionnée après confirmation, avec contrôle du propriétaire et de l’heure de création. L’application peut demander d’enregistrer ou refuser. Aucun arrêt forcé ni fermeture automatique.
- Recherche par nom/PID, tri par RAM/mémoire privée/nom, filtre des applications gérables et compteur de sélection mémoire.
- Recherche, filtres d’état et compteurs du démarrage, boutons Désactiver/Réactiver intégrés avec sauvegarde.
- Liste mémoire limitée aux 30 processus de plus de 100 Mio les plus consommateurs ; seuls les processus autorisés sont modifiables.

### Nouveautés 0.2.0

- **Maintenance** : audit des disques, du démarrage mesuré, de la mémoire virtuelle et du profil énergie ; analyse/nettoyage du magasin de composants Windows, vérification DISM/SFC sans réparation, analyse CHKDSK en ligne, cache DNS, profil économe, activation/désactivation de l’hibernation, rapports énergie/batterie.
- **Avant/après** : référence persistante puis comparaison CPU, mémoire disponible, nombre de processus, espace disque et dernier démarrage mesuré. Les variations dépendent de la charge et ne démontrent pas à elles seules un gain.
- **Nettoyage récupérable** : sélection de fichiers de plus de 7 jours, uniquement à la racine des temporaires utilisateur ; 1 000 propositions maximum, sans parcours récursif. Mise à l’écart, restauration sans écrasement et purge irréversible distincte. Le déplacement ne libère pas d’espace. Les octets de purge sont des tailles logiques, pas une mesure des clusters physiques récupérés.
- **Démarrage** : gestion réversible des valeurs utilisateur `HKCU Run`, sauvegardes chiffrées DPAPI et exclusion des protections identifiées. Les autres sources restent en lecture seule. Les dossiers de démarrage et les tâches des logiciels ne sont pas modifiés.
- **Applications** : inventaire, recherche de mises à jour WinGet et mise à jour d’un identifiant exact confirmé. Les accords de sources et licences ne sont pas acceptés silencieusement.
- **Réseau** : interfaces, DNS, passerelles, proxy, indices de VPN, compteurs cumulés, connexions TCP/processus, ports en écoute. Tests courts de latence/pertes et résolution DNS. Aucune interception du trafic ; aucun verdict automatique de fuite DNS ou d’exfiltration. Les sondes contactent la passerelle, `1.1.1.1` et le DNS configuré pour `example.com`, à la demande.
- **Mémoire intelligente** : surveillance optionnelle tant que Nova reste ouvert, entretien des seuls processus sélectionnés après pression soutenue ≥90 %, pagination faible et délai de 10 minutes. Aucun processus au premier plan n’est entretenu. La réduction de working set peut provoquer des rechargements ; ce n’est ni une correction de fuite mémoire ni une garantie de performance.
- **Planification** : audit en lecture seule quotidien ou hebdomadaire à 9 h, utilisateur connecté ; dernier bilan consultable, notifications Windows optionnelles selon disponibilité. Conserver l’exécutable au même emplacement. Le désinstallateur tente de retirer uniquement la tâche Nova de cet utilisateur ; on peut aussi la désactiver depuis Nova.
- **Distribution** : vérification de version via GitHub et ouverture de la page officielle. La mise à jour de Nova reste manuelle. L’auto-updater signé n’est pas implémenté ; sa clé de signature est distincte du certificat Authenticode Windows.
- **Constructeurs** : raccourcis officiels Intel, NVIDIA, AMD, Dell, Lenovo, HP, ASUS et MSI en complément de WUA. Ce sont des accès aux supports, pas un moteur d’installation multi-catalogue.

Le détail et les limites de validation figurent dans [docs/VALIDATION-0.2.md](docs/VALIDATION-0.2.md).

Un résultat Windows Update vide ne signifie pas qu’aucune version constructeur plus récente n’existe : les catalogues NVIDIA/AMD/Intel/OEM ne sont pas agrégés. Les autres sources de démarrage restent consultables via les paramètres Windows. Aucun gain universel de performances n’est garanti.

## Utiliser Nova

1. Installez puis ouvrez Nova. Le diagnostic initial est en lecture seule.
2. Lancez l’inventaire et la recherche de pilotes.
3. Pour une maintenance, utilisez « Passer en administrateur » puis créez un point de restauration. Validez l’UAC Windows : Nova remplace la session courante et reprend son onglet. Si la demande est annulée ou échoue, la fenêtre initiale revient. Relancez la recherche de pilotes dans la session élevée.
4. Vérifiez la liste et les licences avant « Tout installer ».
5. Consultez les résultats et redémarrez manuellement si Windows le demande.

Un mutex Windows global détenu par le contrôleur PowerShell sérialise les maintenances entre les fenêtres Nova. Une réparation ou installation n’est pas interrompue par un délai arbitraire. Fermer Nova ne garantit pas l’arrêt d’une maintenance : attendez son résultat.

## Confidentialité

Aucune télémétrie, aucun compte ni clé cloud. L’historique local contient l’action, sa date, son succès et les résultats de moins de 100 000 octets, qui peuvent inclure des noms d’applications, chemins et diagnostics locaux. Les relevés mémoire et listes de connexions TCP détaillées ne sont pas conservés dans cet historique ; les questions à l’IA et les paramètres de requête n’y sont pas enregistrés. Vérifiez un export avant de le partager. Le thème reste dans le stockage WebView local. Windows Update communique avec le service Microsoft/entreprise configuré. L’IA reçoit votre question et un résumé des mesures uniquement sur `127.0.0.1:11434`.

## Compiler

Windows x64, Node 24, npm 11, Rust stable MSVC, outils C++ Visual Studio, SDK Windows et WebView2 requis.

```powershell
npm ci --ignore-scripts
npm test
npm run build
powershell -NoProfile -File tests/system-tests.ps1
powershell -NoProfile -File tests/backend-safety-tests.ps1
powershell -NoProfile -File tests/optimize-tests.ps1
powershell -NoProfile -File tests/apps-tests.ps1
powershell -NoProfile -File tests/automation-tests.ps1
powershell -NoProfile -File tests/health-tests.ps1
cargo test --manifest-path src-tauri/Cargo.toml --release --locked
npm run tauri:build
```

Les scripts des dépendances sont désactivés dans `.npmrc`. Les paquets natifs nécessaires sont précompilés. Icônes déjà générées ; régénération : `node scripts/icons.mjs`.

Exécutable : `src-tauri/target/release/`. Installateur : `src-tauri/target/release/bundle/nsis/`. Développement natif : `npm run tauri:dev`. `npm run dev` seul fournit un aperçu de l’interface qui refuse les diagnostics natifs.

Voir [docs/VALIDATION.md](docs/VALIDATION.md) pour la portée des tests. Les tests n’installent pas de pilotes et n’appliquent pas de maintenance au PC hôte.

Sources : [Windows Update Search](https://learn.microsoft.com/en-us/windows/win32/api/wuapi/nf-wuapi-iupdatesearcher-search), [installateur Windows Tauri](https://v2.tauri.app/distribute/windows-installer/).
