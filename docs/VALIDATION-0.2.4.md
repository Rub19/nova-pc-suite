# Validation 0.2.4 — 26 octobre 2026

## Diagnostic du défaut

Le défaut a été reproduit avant correction par un test Rust sur `include_str!` et par ParseInput sur les fichiers lus explicitement en UTF-8 : hardware.ps1, memory.ps1 et network.ps1 étaient rejetés. ParseFile de Windows PowerShell 5.1 lisait les fichiers sans BOM en ANSI et masquait le défaut en transformant les apostrophes typographiques en caractères différents. Les littéraux concernés sont corrigés. Le nouveau contrôle CI parse les neuf scripts intégrés en UTF-8, sans les exécuter.

Le transport vérifie maintenant l’enveloppe JSON, accepte un BOM UTF-8 et conserve le statut du processus. Le diagnostic brut de stderr n’est pas montré ni exporté ; seules des références de catégorie sont retournées. Les erreurs applicatives structurées (dont AdministratorRequired) restent préservées.

## Nettoyage

La suppression utilise l’aperçu protégé existant, l’identité du fichier et des handles retenus. Un manifeste DPAPI est persisté avant les déplacements et la purge. Les fichiers modifiés ou occupés sont conservés ; les échecs partiels et tailles logiques sont rapportés. Ni parcours récursif, ni effacement des dossiers temporaires complets. Tests sur des fichiers de fixture créés sous target/cleanup-tests uniquement ; aucune suppression des fichiers utilisateur pour valider le code.

Le mode automatique est opt-in, stocké localement, et ne tourne que lorsque Nova est ouvert. Premier passage à l’activation/au lancement puis intervalle d’une heure. Désactivation : pas de nouveau passage, sans interruption forcée d’un passage déjà envoyé. Les tests DOM simulés vérifient l’absence d’appel avant activation, le cycle horaire, la reprise du choix, l’arrêt et les résultats partiels sans boîte de confirmation.

## Administrateur et fabricants

Une instance est retenue par un mutex Windows local. Le remplacement élevé est limité à une demande explicite, un jeton aléatoire de 128 bits et un événement d’accusé de réception accessible via les permissions Windows par défaut. La session élevée doit réellement posséder les droits administrateur. La fenêtre initiale est masquée pendant la demande, puis quittée seulement après le signal de préparation ; sinon elle revient. Les tests utilisent des mutex/événements de fixture uniques, sans demande UAC ni élévation automatique.

La détection des fabricants sélectionne des propriétés CIM précises et regroupe les périphériques par règles connues. Les fabricants non associés à un support intégré restent visibles ; aucune URL arbitraire n’est ouverte. Tests de données simulées pour déduplication, fabricants connus/inconnus et absence d’ouverture de navigateur.

## Vérifications

- 25 tests frontend réussis.
- 23 tests Rust réussis, dont les diagnostics réels en lecture seule via le même transport que l’application.
- Tests PowerShell de l’automatisation et des fabricants réussis ; contrôle de syntaxe UTF-8 des neuf scripts réussi.
- Compilation et tests visuels documentés à la livraison. L’UAC reste une interaction utilisateur ; son acceptation ne peut pas être simulée par ces tests.
