# Nova 0.2 — maintenance étendue

## Autorisation et objectifs

L’utilisateur demande l’ajout des optimisations et fonctions annoncées après la preview 0.1.1. Livrer des fonctions utilisables, un installateur et les sources publiques. Conserver le thème sombre graphite.

## Capacités et critères vérifiables

| Module | Résultat attendu | Vérification |
|---|---|---|
| Maintenance | Audit énergie/disques/démarrage, composants Windows, intégrité, scan disque, DNS, hibernation, rapports énergie/batterie | Fixtures et diagnostics réels sans mutation |
| Mesures | Référence persistante et comparaison CPU/RAM/processus/espace/démarrage | Fixtures déterministes ; variations non présentées comme gains prouvés |
| Nettoyage | Aperçu limité aux temporaires anciens, sélection, mise à l’écart récupérable, restauration sans écrasement, purge explicite | Tests fichiers isolés : liens, chemins, conflits, modifications |
| Démarrage | Entrées utilisateur gérables, sauvegarde avant désactivation et restauration explicite | Mocks registre, aucun lancement de commande Run |
| Applications | Inventaire, recherche et mises à jour WinGet par identifiant exact | Mocks ; aucune acceptation silencieuse de licence |
| Automatisation | Audit seul quotidien/hebdomadaire à 9h, session utilisateur ouverte, bilan persistant | AST/mocks tâches ; exécution headless en lecture seule |
| Distribution | Vérification de release et accès à la page officielle, nouvel installateur | Sources HTTPS fixes, SHA-256, NSIS |
| Sources pilotes | Accès aux outils officiels constructeurs | URLs fixes vérifiées ; pas de détection multi-catalogue revendiquée |

## Contraintes

- Contrôles Rust : commandes allowlistées, payload JSON borné et typé au point d’utilisation, confirmation des changements.
- Contrôleurs PowerShell embarqués ; paramètres transportés en base64, jamais interprétés comme commandes.
- Journal d’opérations local avec résultats bornés ; aucune télémétrie.
- Nettoyage : déplacer ne libère pas d’espace ; seul le résultat de purge indique des octets supprimés.
- Aucun réglage de sécurité affaibli, aucun service essentiel désactivé, aucun boost RAM artificiel.
- Rapports de comparaison : observations ponctuelles et contexte explicite.

## Dépendances externes non satisfaites automatiquement

- Aucun certificat Authenticode utilisable trouvé dans le magasin utilisateur lors de l’inspection. Signature publique à fournir séparément ; ne jamais générer une fausse confiance avec un certificat auto-signé.
- La validation Windows 10 et les essais de mutation sur plusieurs matériels demandent des machines dédiées. Les tests locaux ne constituent pas cette validation.
- Les catalogues constructeurs ne fournissent pas ici un contrat uniforme utilisable pour une installation groupée multi-fournisseurs. Les outils officiels sont proposés en complément de WUA.

## Ordre de livraison

Contrats et garde-fous → modules backend indépendants → intégration UI → tests ciblés → revue indépendante → build natif → installateur et publication preview.
