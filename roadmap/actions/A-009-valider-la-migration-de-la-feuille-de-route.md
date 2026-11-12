---
id: A-009
type: valider
titre: "Valider la migration de la feuille de route"
statut: a-faire
cree: 2026-11-12
par: agent
fait:
---

Migration de l'ancien ROADMAP.md (conservé dans docs/ROADMAP-avant-fabboard.md) vers roadmap/. 18 éléments : 4 livrés (v0.0, v0.1, v0.2, v0.4), 3 à tester (v0.3, v0.5, v0.9-beta1), 1 spécifié (v1.1), 7 idées (v0.6, v1.0, v2, DESK-05, DESK-06, audit lots 2 et 3), 3 abandonnés (la section Hors périmètre). 11 jalons dans projet.toml. 8 actions créées, d'après les points d'arrêt en attente de l'ancien fichier et la « Prochaine action » de STATUS.md.

Choix à confirmer : (1) v0.3 et v0.5 passés à « à tester » alors que l'ancien fichier les dit « en cours » : leurs étapes de code sont toutes faites, seule la validation manque (STATUS.md pour la v0.5) ; (2) v1.1 « spécifiée » : le document a but, étapes et critères, mais ne dit pas que tu as validé la spécification (l'ADR de l'étape 1 reste à valider) ; (3) lots 2 et 3 de l'audit sortis de la v0.9-beta1 en deux idées sans jalon ; (4) v0.6 laissée en idée, bien que l'application macOS soit déjà faite.

Dates dont la source n'est pas la feuille de route : aucune date de livraison ; les dates de création viennent du premier commit qui a introduit chaque jalon (git log -S) : 24/10 pour la plupart, 29/10 (v1.1), 30/10 (DESK-05 et 06), 02/11 (v0.9-beta1 et lots d'audit).

À regarder : les liens relatifs vers docs/ dans les corps recopiés ont été corrigés (../../docs/).
