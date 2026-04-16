# AI4Animation Engine - Tutoriel Complet

## 1. Installation et Lancement

### Prérequis
- Windows 10/11 avec GPU Vulkan compatible
- Rust toolchain (`rustup` installé)

### Lancement rapide
```
Double-cliquer sur run.bat
```
Ou en ligne de commande :
```
cd ai4animation-rs
cargo run --release
```

La fenetre s'ouvre avec le logo AI4Animation au centre du viewport 3D.

---

## 2. Interface

L'interface est divisee en 5 zones :

```
+-------------------------------------------------------------------+
| Menu: Fichier | Edition | Affichage | Animation | Langue | Aide   |
+----------+----------------------------------+--------------------+
| HIERARCHY|       VIEWPORT 3D                | INSPECTEUR         |
| (gauche) |  [Toolbar: Select Move Rotate    | - Nom              |
|          |   Measure IK]                    | - Position XYZ     |
| Scene    |                                  | - Rotation XYZ     |
| - Root   |  Camera [Orbit v]  Reset         | - Scale XYZ        |
|   - Hips |                                  | - Composants       |
|   - ...  |  [Boussole XYZ]     [Stats]      |                    |
+----------+----------------------------------+ RENDER SETTINGS    |
| TIMELINE: [|<][<][>||][>][>|] =====[|]=     | - Eclairage        |
| 42/300 1.40s  Speed:[1.0x] Loop Mirror      | - Ombres, SSAO...  |
+----------+----------------------------------+--------------------+
| CONSOLE: 00:05.23 i Charge: Model.glb (24 os, 1200 frames)      |
+-------------------------------------------------------------------+
```

### Zones principales

| Zone | Description |
|------|-------------|
| **Menu** | Fichier (import/export/save), Edition (undo/redo/snap), Affichage, Animation, Langue (FR/EN/ZH), Aide |
| **Hierarchie** (gauche) | Arbre des joints du squelette, recherche, selecteur de modele, suppression |
| **Viewport 3D** (centre) | Vue 3D interactive avec outils, overlays, logo au demarrage |
| **Inspecteur** (droite) | Transform editable (drag values), infos entite, render settings |
| **Timeline** (bas) | Controles de lecture, barre de progression avec keyframes, vitesse, boucle |
| **Console** (bas) | Messages avec horodatage, compteurs par niveau, filtre |

---

## 3. Importer un modele

### Methode 1: Drag-and-drop
Glissez un fichier `.glb`, `.gltf` ou `.bvh` directement sur la fenetre.

### Methode 2: Menu
`Fichier > Importer GLB/glTF` ou `Fichier > Importer BVH`

Le modele apparait dans le viewport avec son squelette visible.

---

## 4. Navigation camera

### Mode Orbite (defaut)
| Action | Controle |
|--------|----------|
| Tourner | Clic droit + glisser |
| Panoramique | Clic milieu + glisser |
| Zoom | Molette |
| Reset | Double-clic ou touche `R` |
| Focus sur modele | Touche `F` |

### Mode Libre (premiere personne)
Changer le mode camera dans le selecteur en haut a gauche du viewport.

| Action | Controle |
|--------|----------|
| Avancer/Reculer | W / S |
| Gauche/Droite | A / D |
| Monter/Descendre | E / Q |
| Regarder | Clic droit + glisser |

---

## 5. Outils de manipulation

La barre d'outils est en haut au centre du viewport.

### Select (touche 1)
- Clic gauche sur un joint = le selectionne
- Le joint selectionne est surligne en jaune dans le viewport et la hierarchie

### Move (touche 2) 
- Clic gauche sur un joint + **glisser** = deplacer le joint en 3D
- Gizmo avec fleches colorees (X=rouge, Y=vert, Z=bleu)
- Appuyer `X`, `Y` ou `Z` pour contraindre le mouvement a un axe
- Activer le snap (bouton "Snap" dans la toolbar) pour aimanter a la grille

### Rotate (touche 3)
- Clic gauche sur un joint + **glisser horizontalement** = rotation
- Anneaux de rotation XYZ affiches
- Appuyer `X`, `Y` ou `Z` pour choisir l'axe de rotation

### Measure (touche 4)
- Clic sur un premier joint = point de depart
- Clic sur un second joint = point d'arrivee + distance affichee
- La distance apparait dans la toolbar et dans la console

### IK (touche 5)
- Clic 1 = definir la racine de la chaine IK
- Clic 2 = definir le bout (end effector)
- **Glisser** = resolution IK en temps reel (algorithme FABRIK)
- La chaine IK est surlignee en magenta

### Raccourci commun
- `Escape` = retour a l'outil Select, reset des contraintes

---

## 6. Lecture d'animation

### Controles
| Action | Controle |
|--------|----------|
| Play / Pause | `Space` ou bouton ▶/⏸ |
| Debut | `Home` ou ⏮ |
| Fin | `End` ou ⏭ |
| Image precedente | `←` ou ⏪ |
| Image suivante | `→` ou ⏩ |
| Scrub | Cliquer/glisser sur la barre de timeline |
| Vitesse | Slider "Speed" (0.1x a 3.0x) |
| Boucle | Bouton "Loop" ou touche `L` |
| Miroir | Bouton "Mirror" ou touche `M` |

### Barre de timeline
- Fond sombre avec ticks de keyframes
- Tete de lecture jaune (triangle + ligne verticale)
- Barre de progression bleue
- Clic/drag pour scrubber directement

---

## 7. Inspecteur

Quand un joint est selectionne :
- **Position** : glisser les valeurs X/Y/Z pour deplacer
- **Rotation** : glisser les valeurs en degres
- **Scale** : glisser les valeurs d'echelle
- **Focus** : bouton pour centrer la camera sur le joint
- **Reset Pos** : remet la position a zero

Toutes les modifications sont enregistrees dans l'historique d'annulation.

---

## 8. Affichage

### Menu Affichage
| Option | Touche | Description |
|--------|--------|-------------|
| Squelette | - | Afficher/masquer les os et joints |
| Maillage | - | Afficher/masquer le mesh 3D |
| Velocites | - | Vecteurs de vitesse sur chaque joint |
| Grille | `G` | Grille au sol |
| Axes | - | Axes RGB a l'origine (X=rouge, Y=vert, Z=bleu) |
| Gizmo | - | Gizmo de transformation |
| Console | - | Panneau de log |
| Render Settings | - | Panneau de reglages de rendu |

---

## 9. Parametres de rendu

Panneau accessible via `Affichage > Parametres de rendu`.

### Eclairage
- **Exposition** : luminosite globale (0.1 - 3.0)
- **Intensite soleil** : force de la lumiere directionnelle
- **Direction** : yaw et pitch de la lumiere
- **Couleur soleil** : selecteur de couleur
- **Ciel / Sol / Ambiance** : intensites de l'eclairage indirect

### Ombres
- Activer/desactiver les ombres portees
- Biais d'ombre (eviter le shadow acne)

### SSAO (Ambient Occlusion)
- Rayon, intensite, biais
- Ajoute de la profondeur aux crevasses du mesh

### Bloom
- Intensite et etendue du halo lumineux

### FXAA
- Anti-aliasing post-process

### Grille
- Taille et nombre de divisions

### Reset
- Bouton "Reinitialiser" pour revenir aux valeurs par defaut

---

## 10. Sauvegarde et Export

### Sauvegarder un projet
`Fichier > Sauvegarder projet` ou `Ctrl+S`
- Format `.a4a` (JSON)
- Sauvegarde : camera, affichage, render settings, playback

### Ouvrir un projet
`Fichier > Ouvrir projet` ou `Ctrl+O`

### Exporter BVH
`Fichier > Exporter BVH`
- Exporte la pose actuelle du squelette au format BVH
- Utilisable dans Blender, Maya, MotionBuilder, etc.

---

## 11. Undo / Redo

| Action | Raccourci |
|--------|-----------|
| Annuler | `Ctrl+Z` |
| Refaire | `Ctrl+Y` ou `Ctrl+Shift+Z` |

L'historique garde les 100 dernieres operations.
Accessible aussi via `Edition > Undo / Redo`.

---

## 12. Tous les raccourcis clavier

| Raccourci | Action |
|-----------|--------|
| `Ctrl+S` | Sauvegarder projet |
| `Ctrl+Z` | Annuler |
| `Ctrl+Y` | Refaire |
| `Space` | Play / Pause |
| `Home` | Debut animation |
| `End` | Fin animation |
| `←` / `→` | Image precedente / suivante |
| `R` | Reset camera |
| `F` | Focus sur modele |
| `G` | Basculer grille |
| `L` | Basculer boucle |
| `M` | Basculer miroir |
| `1` | Outil Select |
| `2` | Outil Move |
| `3` | Outil Rotate |
| `4` | Outil Measure |
| `5` | Outil IK |
| `X` / `Y` / `Z` | Contrainte d'axe |
| `Escape` | Reset outil |
| `W/A/S/D` | Deplacement (mode libre) |
| `E` / `Q` | Monter / Descendre (mode libre) |

---

## 13. Pipeline de rendu

Le moteur utilise un pipeline de rendu differe (deferred) en 9 passes :

1. **Shadow Map** - Carte de profondeur depuis la lumiere (2048x2048)
2. **G-Buffer** - Albedo, normales, profondeur lineaire
3. **SSAO** - Occultation ambiante en espace ecran
4. **Blur H** - Flou horizontal du SSAO
5. **Blur V** - Flou vertical du SSAO
6. **Lighting** - Eclairage differe (combine G-buffer + SSAO + ombres)
7. **Bloom** - Post-process de halo lumineux
8. **FXAA** - Anti-aliasing final
9. **Line Overlay** - Grille, squelette, gizmos

---

## 14. Architecture technique

```
ai4animation-rs/
  crates/
    anim-math/       Mathematiques (glam, Transform, Quaternion)
    anim-core/       ECS leger (Scene, Entity, Time, i18n)
    anim-import/     Import GLB/BVH, Export BVH
    anim-animation/  Motion, Hierarchy, interpolation
    anim-ik/         Solveur FABRIK
    anim-render/     Pipeline wgpu (deferred, 10 shaders WGSL)
    anim-gui/        Panneaux egui, AppState, outils
    anim-app/        Point d'entree, boucle principale
```

### Technologies
- **Rust** - Langage systeme, performances natives
- **wgpu** - API graphique cross-platform (Vulkan/DX12/Metal)
- **egui** - Interface utilisateur immediate-mode
- **glam** - Mathematiques 3D avec SIMD
- **gltf** - Chargement de modeles GLB/glTF

---

## 15. Formats supportes

| Format | Import | Export |
|--------|--------|--------|
| GLB/glTF | Oui | - |
| BVH | Oui | Oui |
| FBX | Prevu | - |
| NPZ | Prevu | - |
