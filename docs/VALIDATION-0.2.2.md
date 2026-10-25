# Stockage, démarrage et énergie — 0.2.2

## Contrat

Six actions supplémentaires sont explicitement autorisées côté Rust. Seule `storage-optimize` modifie le disque et exige une confirmation. `storage-analyze` ne lance que l’analyse Windows. Les deux nécessitent les droits administrateur et partagent le mutex de maintenance avec les réparations et installations. Aucun paramètre libre ni choix d’un autre volume n’est accepté. Le volume système est dérivé du dossier système Windows connu, puis doit être fixe, NTFS et déclaré sain.

`Optimize-Volume` conserve le choix de Windows selon le support, sans forcer Defrag/ReTrim ni NormalPriority. Le texte retourné est borné à 24 576 caractères. L’application reste disponible pendant l’appel natif ; ne pas interrompre une maintenance en cours.

Les événements de démarrage sont limités aux ID 101, 102, 103, aux sept derniers jours et à 30 événements. Seuls le nom affiché, l’ID, la date et les durées numériques valides sont extraits. Journal inaccessible et absence d’événements sont distingués. Aucune désactivation automatique de service ou pilote.

Les commandes d’énergie sont fixes : `/requests`, `/availablesleepstates`, `/lastwake`, `/waketimers`. Elles ne créent aucun override et ne désactivent aucun périphérique ou minuteur.

## Vérifications

- Tests PowerShell : 84 vérifications, commandes OS et accès disque simulés. Couvre les refus admin/payload, les supports non sains, le choix Windows du traitement, les arguments fixes et les erreurs du journal.
- Tests d’interface : 17 tests passent, dont confirmation obligatoire de l’optimisation, filtre Stockage et tableau des lenteurs au démarrage. DOM et bridge simulés.
- Tests Rust : 14 tests passent. Compilation Tauri/NSIS réussie ; installateur et exécutable version 0.2.2 publiés avec leurs SHA256.
- Types 101/102/103 et noms de champs vérifiés dans les métadonnées du fournisseur Windows Diagnostics-Performance installé, en lecture seule, sans lecture d’événements utilisateur.
- Aucune optimisation native, aucun réglage de veille ni manipulation du bureau sur le PC utilisateur pour valider cette version. Il reste à mesurer les effets sur des supports physiques dédiés et à valider l’interface en conditions réelles Windows 10/11. Les tests simulés ne remplacent pas cette validation.

## Références

- [Microsoft — Optimize-Volume](https://learn.microsoft.com/en-us/powershell/module/storage/optimize-volume)
- [Microsoft — Powercfg](https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/powercfg-command-line-options)
