# ADR-0009 : Projet `.fabweave.json`, embarqué dans le G-code

Status: Accepted (choix de l'utilisateur du 17/11/2026)
Date: 2026-11-17

## Context

Le jalon v0.6 (R-007, DESK-04) demande d'enregistrer une préparation et de la rouvrir sans la ressaisir, y compris en déposant le G-code exporté. Un projet peut être ouvert sur un autre poste, longtemps après, quand le catalogue ou les profils ont changé. Le kit `fab-design-system` fixe la forme `{ format: "<app>.project", version: n, … }` et souhaite un projet embarquable dans l'export.

## Decision

1. **Tout embarqué** : le projet recopie la source du `.weave` et le TOML des deux profils, avec le nom du projet, les valeurs de tous les paramètres et la préparation (`Setup` : buse, placement, adhésion). Aucune référence à un identifiant du catalogue.
2. **Format** `{ "format": "fabweave.project", "version": 1, … }` (JSON indenté, clés dans l'ordre), lu strictement ; une version plus récente est refusée avec `project.unsupported_version`.
3. **Toujours embarqué dans le G-code** : bloc de commentaires à la fin (`; fabweave.project begin` … `end`), JSON compressé en deflate puis en base64 (lignes de 76 caractères). Dans le paquet `.gcode.3mf`, aussi rangé à part (`Metadata/fabweave_project.json`).
4. **Valeurs** : celles du calcul ; une valeur refusée (hors bornes) est gardée telle quelle, pour que le projet rouvert redonne la même erreur au lieu d'une valeur corrigée en silence.
5. **Réouverture** (interface) : un modèle ou un profil au même texte qu'un existant est repris ; sinon il est ajouté à « Mes modèles » ou aux profils de l'utilisateur, sans en remplacer un. Seules les valeurs différentes de celles du modèle sont gardées (chacune a son ↺). Une seule étape d'annulation.
6. La lecture tolère un fichier étranger : archive ou G-code abîmé, tronqué, ou piégé (décompression plafonnée à 16 Mio) donne un code stable, jamais une panique.

## Alternatives considered

- **Références seulement** (identifiants et valeurs) : fichier minuscule, mais il ne se rouvre plus quand le modèle ou un profil change ou manque. Rejeté par l'utilisateur.
- **Mixte** (références pour le livré, contenu pour le personnel) : un profil livré peut changer d'une version à l'autre et le G-code ne serait plus reproductible. Rejeté.
- **Option d'export** pour embarquer le projet : complique l'export pour quelques kilo-octets. Rejeté par l'utilisateur.

## Consequences

- Même projet → même G-code, octet pour octet (testé) ; le bloc ajouté pèse quelques kilo-octets (15 Ko pour la lampe à coussins, sur un G-code de 6,4 Mo).
- Le G-code transporte la source du modèle et des profils : à savoir avant de partager un modèle personnel.
- L'interface lit les 8 derniers Mio d'un `.gcode` pour trouver le projet, sans charger un gros fichier en entier.
